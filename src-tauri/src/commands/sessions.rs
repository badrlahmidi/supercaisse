use crate::db::*;
use crate::session::{autoriser, verifier_acces, Acces, AuthState, SessionUtilisateur};
use rusqlite::{params, Connection, OptionalExtension};
use tauri::State;

use super::calcul::{en_dh, montant_positif, vers_centimes};
use super::{default_magasin_id, log_audit};

#[tauri::command(async)]
pub fn get_current_session(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
) -> Result<Option<super::contrats::SessionCaisse>, String> {
    let conn = db.lecture()?;
    let me = autoriser(&auth, &conn, &token, Acces::Connecte)?;
    let caissier_id = me.user_id;
    let mut stmt = conn
        .prepare(
            "SELECT id, caissier_id, date_ouverture, fond_initial, statut, magasin_id
         FROM sessions_caisse
         WHERE caissier_id = ?1 AND statut = 'ouverte'
         ORDER BY id DESC LIMIT 1",
        )
        .map_err(|e| e.to_string())?;

    let mut rows = stmt
        .query_map(params![caissier_id], |row| {
            Ok(super::contrats::SessionCaisse {
                id: row.get(0)?,
                caissier_id: row.get(1)?,
                date_ouverture: row.get(2)?,
                fond_initial: row.get(3)?,
                statut: row.get(4)?,
                magasin_id: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?;

    if let Some(row) = rows.next() {
        return Ok(Some(row.map_err(|e| e.to_string())?));
    }
    Ok(None)
}

#[tauri::command(async)]
pub fn open_session(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    fond_initial: f64,
    magasin_id: Option<i64>,
) -> Result<i64, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let me = autoriser(&auth, &conn, &token, Acces::Connecte)?;
    let conn = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    let fond_initial = montant_positif("Fond de caisse", fond_initial)?;
    let caissier_id = me.user_id;

    let count: i64 = conn
        .query_row(
            "SELECT count(*) FROM sessions_caisse WHERE caissier_id = ?1 AND statut = 'ouverte'",
            params![caissier_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    if count > 0 {
        return Err("Une session est déjà ouverte pour ce caissier".to_string());
    }

    let mid = match magasin_id {
        Some(id) => id,
        None => default_magasin_id(&conn)?,
    };
    conn.execute(
        "INSERT INTO sessions_caisse (caissier_id, fond_initial, statut, magasin_id) VALUES (?1, ?2, 'ouverte', ?3)",
        params![caissier_id, fond_initial, mid]
    ).map_err(|e| e.to_string())?;

    let session_id = conn.last_insert_rowid();
    log_audit(
        &conn,
        Some(caissier_id),
        "ouvrir_session",
        &format!(
            "Ouverture session #{} - fond initial: {} DH",
            session_id, fond_initial
        ),
        Some("session"),
        Some(session_id),
    )?;
    conn.commit().map_err(|e| e.to_string())?;
    Ok(session_id)
}

pub(crate) struct TotauxEspecesSession {
    pub ventes_especes: f64,
    pub entrees: f64,
    pub sorties: f64,
}

pub(crate) fn totaux_especes_session(
    conn: &Connection,
    session_id: i64,
) -> Result<TotauxEspecesSession, String> {
    let ventes_especes: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(ROUND((CASE WHEN v.dtype = 'avoir' THEN -vp.montant ELSE vp.montant END) * 100)) / 100.0, 0)
         FROM vente_paiements vp
         JOIN ventes v ON v.id = vp.vente_id
         WHERE vp.session_id = ?1 AND vp.mode = 'especes' AND v.statut != 'annulee'",
            params![session_id],
            |row| row.get(0),
        )
        .map_err(|e| format!("Calcul des ventes espèces impossible : {}", e))?;
    let (entrees, sorties): (f64, f64) = conn
        .query_row(
            "SELECT COALESCE(SUM(ROUND((CASE WHEN jtype = 'entree' THEN montant END) * 100)) / 100.0, 0),
                COALESCE(SUM(ROUND((CASE WHEN jtype = 'sortie' THEN ABS(montant) END) * 100)) / 100.0, 0)
         FROM journal_caisse WHERE session_id = ?1",
            params![session_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|e| format!("Calcul des mouvements de caisse impossible : {}", e))?;
    Ok(TotauxEspecesSession {
        ventes_especes,
        entrees,
        sorties,
    })
}

#[tauri::command(async)]
pub fn close_session(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    session_id: i64,
    total_especes_declare: f64,
) -> Result<(), String> {
    let mut conn = db.conn.lock().map_err(|e| e.to_string())?;
    let me = autoriser(&auth, &conn, &token, Acces::Connecte)?;
    verifier_session_propre(&conn, &me, session_id, "modifier")?;
    super::tracer(
        &format!("Clôture de la session de caisse {}", session_id),
        close_session_impl(
            &mut conn,
            session_id,
            total_especes_declare,
            Some(me.user_id),
        ),
    )
}

pub(crate) fn verifier_session_propre(
    conn: &Connection,
    me: &SessionUtilisateur,
    session_id: i64,
    action: &'static str,
) -> Result<(), String> {
    let caissier: Option<i64> = conn
        .query_row(
            "SELECT caissier_id FROM sessions_caisse WHERE id = ?1",
            params![session_id],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    match caissier {
        None => Err("Session introuvable".to_string()),
        Some(id) if id == me.user_id => Ok(()),
        Some(_) => verifier_acces(conn, me, Acces::Module("journal", action)),
    }
}

pub(crate) fn close_session_impl(
    conn: &mut Connection,
    session_id: i64,
    total_especes_declare: f64,
    utilisateur_id: Option<i64>,
) -> Result<(), String> {
    let total_especes_declare = montant_positif("Total espèces déclaré", total_especes_declare)?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    let fond_initial: f64 = tx
        .query_row(
            "SELECT fond_initial FROM sessions_caisse WHERE id = ?1 AND statut = 'ouverte'",
            params![session_id],
            |row| row.get(0),
        )
        .map_err(|_| "Session introuvable ou déjà clôturée".to_string())?;

    let totaux = totaux_especes_session(&tx, session_id)?;
    let attendu_centimes = vers_centimes(fond_initial)
        + vers_centimes(totaux.ventes_especes)
        + vers_centimes(totaux.entrees)
        - vers_centimes(totaux.sorties);
    let total_attendu = en_dh(attendu_centimes);
    let ecart = en_dh(vers_centimes(total_especes_declare) - attendu_centimes);
    let date_cloture = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    tx.execute(
        "UPDATE sessions_caisse
         SET date_cloture = ?1, total_especes_attendu = ?2, total_especes_declare = ?3, ecart = ?4, statut = 'cloturee'
         WHERE id = ?5",
        params![date_cloture, total_attendu, total_especes_declare, ecart, session_id]
    ).map_err(|e| e.to_string())?;

    log_audit(
        &tx,
        utilisateur_id,
        "fermer_session",
        &format!(
            "Clôture session #{} - attendu: {:.2} DH, déclaré: {:.2} DH, écart: {:.2} DH",
            session_id, total_attendu, total_especes_declare, ecart
        ),
        Some("session"),
        Some(session_id),
    )?;

    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::ventes::normaliser_paiements;
    use serde_json::json;

    fn setup() -> Connection {
        let conn = crate::db::init_db(":memory:").unwrap();
        conn.execute("INSERT INTO articles (id, designation, prix_vente, tva) VALUES (1, 'Article', 100, 20)", []).unwrap();
        conn.execute_batch("INSERT INTO article_stocks (article_id, magasin_id, quantite) SELECT id, 1, 100 FROM articles; UPDATE articles SET stock = 100;").unwrap();
        conn.execute("INSERT INTO sessions_caisse (id, caissier_id, fond_initial, statut, magasin_id) VALUES (1, 1, 100, 'ouverte', 1)", []).unwrap();
        conn
    }

    fn ligne() -> Vec<serde_json::Value> {
        vec![json!({ "article_id": 1, "quantite": 1 })]
    }

    fn vendre(
        conn: &mut Connection,
        dtype: &str,
        paiements: serde_json::Value,
    ) -> Result<i64, String> {
        let r = crate::commands::ventes::vendre_json(
            conn,
            None,
            Some(1),
            ligne(),
            None,
            "especes".into(),
            Some(paiements.as_array().unwrap().clone()),
            Some(dtype.into()),
            None,
            Some(1),
        )?;
        Ok(r.id)
    }

    #[test]
    fn test_cloture_compte_les_ventes_especes_de_la_session() {
        let mut conn = setup();
        vendre(
            &mut conn,
            "facture",
            json!([{ "mode": "especes", "montant": 120 }]),
        )
        .unwrap();
        vendre(
            &mut conn,
            "facture",
            json!([{ "mode": "especes", "montant": 50 }, { "mode": "carte", "montant": 70 }]),
        )
        .unwrap();
        vendre(
            &mut conn,
            "bl",
            json!([{ "mode": "carte", "montant": 120 }]),
        )
        .unwrap();
        vendre(
            &mut conn,
            "devis",
            json!([{ "mode": "especes", "montant": 999 }]),
        )
        .unwrap();
        let annulee = vendre(
            &mut conn,
            "facture",
            json!([{ "mode": "especes", "montant": 120 }]),
        )
        .unwrap();
        conn.execute(
            "UPDATE ventes SET statut = 'annulee' WHERE id = ?1",
            params![annulee],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO journal_caisse (jtype, montant, session_id) VALUES ('sortie', 20, 1)",
            [],
        )
        .unwrap();

        let totaux = totaux_especes_session(&conn, 1).unwrap();
        assert_eq!(totaux.ventes_especes, 170.0);
        assert_eq!(totaux.sorties, 20.0);

        close_session_impl(&mut conn, 1, 250.0, None).unwrap();
        let (attendu, ecart, statut): (f64, f64, String) = conn
            .query_row(
                "SELECT total_especes_attendu, ecart, statut FROM sessions_caisse WHERE id = 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(attendu, 250.0);
        assert_eq!(ecart, 0.0);
        assert_eq!(statut, "cloturee");
    }

    #[test]
    fn test_cloture_exacte_au_centime_apres_de_nombreuses_petites_ventes() {
        let mut conn = setup();
        conn.execute_batch(
            "UPDATE articles SET prix_vente = 0.0833, stock = 1000 WHERE id = 1;
             UPDATE article_stocks SET quantite = 1000 WHERE article_id = 1;",
        )
        .unwrap();
        for _ in 0..300 {
            crate::commands::ventes::vendre_json(
                &mut conn,
                None,
                Some(1),
                vec![json!({ "article_id": 1, "quantite": 1 })],
                None,
                "especes".to_string(),
                None,
                Some("facture".to_string()),
                None,
                Some(1),
            )
            .unwrap();
        }
        conn.execute(
            "INSERT INTO journal_caisse (jtype, montant, session_id) VALUES ('entree', 0.1, 1), ('sortie', 0.2, 1)",
            [],
        )
        .unwrap();
        let totaux = totaux_especes_session(&conn, 1).unwrap();
        assert_eq!(totaux.ventes_especes, 30.0);
        close_session_impl(&mut conn, 1, 129.9, None).unwrap();
        let (attendu, ecart): (f64, f64) = conn
            .query_row(
                "SELECT total_especes_attendu, ecart FROM sessions_caisse WHERE id = 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(attendu, 129.9);
        assert_eq!(ecart, 0.0);
        assert!(close_session_impl(&mut conn, 1, f64::NAN, None).is_err());
    }

    #[test]
    fn test_cloture_annulee_si_l_audit_echoue() {
        let mut conn = setup();
        conn.execute_batch(
            "CREATE TRIGGER audit_bloque BEFORE INSERT ON audit_log BEGIN SELECT RAISE(ABORT, 'disque plein'); END;",
        )
        .unwrap();
        assert!(close_session_impl(&mut conn, 1, 100.0, None).is_err());
        let statut: String = conn
            .query_row("SELECT statut FROM sessions_caisse WHERE id = 1", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(statut, "ouverte");
    }

    #[test]
    fn test_cloture_detecte_un_manque() {
        let mut conn = setup();
        vendre(
            &mut conn,
            "facture",
            json!([{ "mode": "especes", "montant": 120 }]),
        )
        .unwrap();
        close_session_impl(&mut conn, 1, 200.0, None).unwrap();
        let ecart: f64 = conn
            .query_row("SELECT ecart FROM sessions_caisse WHERE id = 1", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(ecart, -20.0);
    }

    #[test]
    fn test_cloture_ignore_les_autres_sessions() {
        let mut conn = setup();
        conn.execute("INSERT INTO utilisateurs (id, login, password_hash, nom) VALUES (2, 'c2', 'x', 'Caissier 2')", []).unwrap();
        conn.execute("INSERT INTO sessions_caisse (id, caissier_id, fond_initial, statut, magasin_id) VALUES (2, 2, 0, 'ouverte', 1)", []).unwrap();
        crate::commands::ventes::vendre_json(
            &mut conn,
            None,
            Some(2),
            ligne(),
            None,
            "especes".into(),
            Some(vec![json!({ "mode": "especes", "montant": 120 })]),
            Some("facture".into()),
            None,
            Some(1),
        )
        .unwrap();
        assert_eq!(
            totaux_especes_session(&conn, 1).unwrap().ventes_especes,
            0.0
        );
        assert_eq!(
            totaux_especes_session(&conn, 2).unwrap().ventes_especes,
            120.0
        );
    }

    #[test]
    fn test_paiement_sans_detail_utilise_le_ttc() {
        let mut conn = setup();
        crate::commands::ventes::vendre_json(
            &mut conn,
            None,
            Some(1),
            ligne(),
            Some(10.0),
            "especes".into(),
            None,
            Some("facture".into()),
            None,
            Some(1),
        )
        .unwrap();
        assert_eq!(
            totaux_especes_session(&conn, 1).unwrap().ventes_especes,
            108.0
        );
    }

    #[test]
    fn test_credit_en_paiement_fractionne_respecte_le_plafond() {
        let mut conn = setup();
        conn.execute("INSERT INTO clients (id, nom, credit_plafond, credit_actuel) VALUES (1, 'Client', 100, 0)", []).unwrap();
        let depasse = crate::commands::ventes::vendre_json(
            &mut conn,
            Some(1),
            Some(1),
            ligne(),
            None,
            "especes".into(),
            Some(vec![
                json!({ "mode": "especes", "montant": 10 }),
                json!({ "mode": "credit", "montant": 110 }),
            ]),
            Some("facture".into()),
            None,
            Some(1),
        );
        assert!(depasse.unwrap_err().contains("Plafond"));
        crate::commands::ventes::vendre_json(
            &mut conn,
            Some(1),
            Some(1),
            ligne(),
            None,
            "especes".into(),
            Some(vec![
                json!({ "mode": "especes", "montant": 40 }),
                json!({ "mode": "credit", "montant": 80 }),
            ]),
            Some("facture".into()),
            None,
            Some(1),
        )
        .unwrap();
        let credit: f64 = conn
            .query_row("SELECT credit_actuel FROM clients WHERE id = 1", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(credit, 80.0);
        assert_eq!(
            totaux_especes_session(&conn, 1).unwrap().ventes_especes,
            40.0
        );
    }

    #[test]
    fn test_paiements_invalides_refuses() {
        use crate::commands::contrats::{ModePaiement, PaiementSaisi};
        let splits = |v: serde_json::Value| serde_json::from_value::<Vec<PaiementSaisi>>(v);
        assert!(splits(json!([{ "mode": "bitcoin", "montant": 10 }])).is_err());
        assert!(splits(json!([{ "mode": "especes", "amount": 10 }])).is_err());
        assert!(serde_json::from_value::<ModePaiement>(json!("inconnu")).is_err());
        let negatif = splits(json!([{ "mode": "especes", "montant": -5 }])).unwrap();
        assert!(normaliser_paiements(ModePaiement::Especes, Some(&negatif), 0.0).is_err());
        let mixte = splits(json!([{ "mode": "mixte", "montant": 5 }])).unwrap();
        assert!(normaliser_paiements(ModePaiement::Especes, Some(&mixte), 0.0).is_err());
        assert!(normaliser_paiements(ModePaiement::Mixte, None, 10.0).is_err());
        assert_eq!(
            normaliser_paiements(ModePaiement::Carte, None, 42.0).unwrap(),
            vec![("carte".to_string(), 42.0)]
        );
    }

    #[test]
    fn test_reprise_des_ventes_de_session_ouverte_a_la_mise_a_jour() {
        let path = std::env::temp_dir().join(format!(
            "supercaisse_test_backfill_{}_{}.db",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path_str = path.to_string_lossy().to_string();
        {
            let conn = crate::db::init_db(&path_str).unwrap();
            conn.execute("INSERT INTO articles (id, designation, prix_vente, tva) VALUES (1, 'Article', 100, 20)", []).unwrap();
            conn.execute_batch("INSERT INTO article_stocks (article_id, magasin_id, quantite) SELECT id, 1, 100 FROM articles; UPDATE articles SET stock = 100;").unwrap();
            conn.execute("INSERT INTO sessions_caisse (id, caissier_id, fond_initial, statut) VALUES (1, 1, 0, 'ouverte')", []).unwrap();
            conn.execute("INSERT INTO sessions_caisse (id, caissier_id, fond_initial, statut) VALUES (2, 1, 0, 'cloturee')", []).unwrap();
            conn.execute_batch("
                INSERT INTO ventes (id, mode_paiement, montant_remise, session_id, dtype, statut) VALUES (1, 'especes', 0, 1, 'facture', 'validee');
                INSERT INTO vente_articles (vente_id, article_id, quantite, prix_unitaire, total_ligne) VALUES (1, 1, 1, 100, 120);
                INSERT INTO ventes (id, mode_paiement, montant_remise, session_id, dtype, statut) VALUES (2, 'especes', 0, 2, 'facture', 'validee');
                INSERT INTO vente_articles (vente_id, article_id, quantite, prix_unitaire, total_ligne) VALUES (2, 1, 1, 100, 120);
                PRAGMA user_version = 0;
            ").unwrap();
        }
        let conn = crate::db::init_db(&path_str).unwrap();
        assert_eq!(
            totaux_especes_session(&conn, 1).unwrap().ventes_especes,
            120.0
        );
        assert_eq!(
            totaux_especes_session(&conn, 2).unwrap().ventes_especes,
            0.0
        );
        drop(conn);
        let conn = crate::db::init_db(&path_str).unwrap();
        assert_eq!(
            totaux_especes_session(&conn, 1).unwrap().ventes_especes,
            120.0
        );
        drop(conn);
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{}", path_str, suffix));
        }
    }
}
