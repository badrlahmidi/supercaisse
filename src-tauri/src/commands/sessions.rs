use crate::db::*;
use rusqlite::{params, Connection};
use tauri::State;

use super::{default_magasin_id, log_audit};

#[tauri::command]
pub fn get_current_session(db: State<DbState>, caissier_id: i64) -> Result<Option<serde_json::Value>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT id, caissier_id, date_ouverture, fond_initial, statut, magasin_id
         FROM sessions_caisse
         WHERE caissier_id = ?1 AND statut = 'ouverte'
         ORDER BY id DESC LIMIT 1"
    ).map_err(|e| e.to_string())?;

    let mut rows = stmt.query_map(params![caissier_id], |row| {
        Ok(serde_json::json!({
            "id": row.get::<_, i64>(0)?,
            "caissier_id": row.get::<_, i64>(1)?,
            "date_ouverture": row.get::<_, String>(2)?,
            "fond_initial": row.get::<_, f64>(3)?,
            "statut": row.get::<_, String>(4)?,
            "magasin_id": row.get::<_, Option<i64>>(5)?
        }))
    }).map_err(|e| e.to_string())?;

    if let Some(row) = rows.next() {
        return Ok(Some(row.map_err(|e| e.to_string())?));
    }
    Ok(None)
}

#[tauri::command]
pub fn open_session(db: State<DbState>, caissier_id: i64, fond_initial: f64, magasin_id: Option<i64>) -> Result<i64, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let count: i64 = conn.query_row(
        "SELECT count(*) FROM sessions_caisse WHERE caissier_id = ?1 AND statut = 'ouverte'",
        params![caissier_id],
        |row| row.get(0)
    ).unwrap_or(0);

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
    log_audit(&conn, Some(caissier_id), "ouvrir_session",
        &format!("Ouverture session #{} - fond initial: {} DH", session_id, fond_initial),
        Some("session"), Some(session_id));
    Ok(session_id)
}

pub(crate) struct TotauxEspecesSession {
    pub ventes_especes: f64,
    pub entrees: f64,
    pub sorties: f64,
}

pub(crate) fn totaux_especes_session(conn: &Connection, session_id: i64) -> Result<TotauxEspecesSession, String> {
    let ventes_especes: f64 = conn.query_row(
        "SELECT COALESCE(SUM(vp.montant), 0)
         FROM vente_paiements vp
         JOIN ventes v ON v.id = vp.vente_id
         WHERE vp.session_id = ?1 AND vp.mode = 'especes' AND v.statut != 'annulee'",
        params![session_id],
        |row| row.get(0),
    ).map_err(|e| format!("Calcul des ventes espèces impossible : {}", e))?;
    let (entrees, sorties): (f64, f64) = conn.query_row(
        "SELECT COALESCE(SUM(CASE WHEN jtype = 'entree' THEN montant END), 0),
                COALESCE(SUM(CASE WHEN jtype = 'sortie' THEN ABS(montant) END), 0)
         FROM journal_caisse WHERE session_id = ?1",
        params![session_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    ).map_err(|e| format!("Calcul des mouvements de caisse impossible : {}", e))?;
    Ok(TotauxEspecesSession { ventes_especes, entrees, sorties })
}

#[tauri::command]
pub fn close_session(db: State<DbState>, session_id: i64, total_especes_declare: f64) -> Result<(), String> {
    let mut conn = db.conn.lock().map_err(|e| e.to_string())?;
    close_session_impl(&mut conn, session_id, total_especes_declare)
}

pub(crate) fn close_session_impl(conn: &mut Connection, session_id: i64, total_especes_declare: f64) -> Result<(), String> {
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    let fond_initial: f64 = tx.query_row(
        "SELECT fond_initial FROM sessions_caisse WHERE id = ?1 AND statut = 'ouverte'",
        params![session_id],
        |row| row.get(0)
    ).map_err(|_| "Session introuvable ou déjà clôturée".to_string())?;

    let totaux = totaux_especes_session(&tx, session_id)?;
    let total_attendu = fond_initial + totaux.ventes_especes + totaux.entrees - totaux.sorties;
    let ecart = total_especes_declare - total_attendu;
    let date_cloture = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    tx.execute(
        "UPDATE sessions_caisse
         SET date_cloture = ?1, total_especes_attendu = ?2, total_especes_declare = ?3, ecart = ?4, statut = 'cloturee'
         WHERE id = ?5",
        params![date_cloture, total_attendu, total_especes_declare, ecart, session_id]
    ).map_err(|e| e.to_string())?;

    log_audit(&tx, None, "fermer_session",
        &format!("Clôture session #{} - attendu: {:.2} DH, déclaré: {:.2} DH, écart: {:.2} DH", session_id, total_attendu, total_especes_declare, ecart),
        Some("session"), Some(session_id));

    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::ventes::{create_vente_impl, normaliser_paiements};
    use serde_json::json;

    fn setup() -> Connection {
        let conn = crate::db::init_db(":memory:").unwrap();
        conn.execute("INSERT INTO articles (id, designation, prix_vente, tva) VALUES (1, 'Article', 100, 20)", []).unwrap();
        conn.execute("INSERT INTO sessions_caisse (id, caissier_id, fond_initial, statut, magasin_id) VALUES (1, 1, 100, 'ouverte', 1)", []).unwrap();
        conn
    }

    fn ligne() -> Vec<serde_json::Value> {
        vec![json!({ "article_id": 1, "quantite": 1, "prix_unitaire": 100, "tva": 20 })]
    }

    fn vendre(conn: &mut Connection, dtype: &str, paiements: serde_json::Value) -> Result<i64, String> {
        let r = create_vente_impl(conn, None, Some(1), ligne(), 0.0, "especes".into(),
            Some(paiements.as_array().unwrap().clone()), Some(dtype.into()), None, None, Some(1))?;
        Ok(r["id"].as_i64().unwrap())
    }

    #[test]
    fn test_cloture_compte_les_ventes_especes_de_la_session() {
        let mut conn = setup();
        vendre(&mut conn, "facture", json!([{ "mode": "especes", "montant": 120 }])).unwrap();
        vendre(&mut conn, "facture", json!([{ "mode": "especes", "montant": 50 }, { "mode": "carte", "montant": 70 }])).unwrap();
        vendre(&mut conn, "bl", json!([{ "mode": "carte", "montant": 120 }])).unwrap();
        vendre(&mut conn, "devis", json!([{ "mode": "especes", "montant": 999 }])).unwrap();
        let annulee = vendre(&mut conn, "facture", json!([{ "mode": "especes", "montant": 120 }])).unwrap();
        conn.execute("UPDATE ventes SET statut = 'annulee' WHERE id = ?1", params![annulee]).unwrap();
        conn.execute("INSERT INTO journal_caisse (jtype, montant, session_id) VALUES ('sortie', 20, 1)", []).unwrap();

        let totaux = totaux_especes_session(&conn, 1).unwrap();
        assert_eq!(totaux.ventes_especes, 170.0);
        assert_eq!(totaux.sorties, 20.0);

        close_session_impl(&mut conn, 1, 250.0).unwrap();
        let (attendu, ecart, statut): (f64, f64, String) = conn.query_row(
            "SELECT total_especes_attendu, ecart, statut FROM sessions_caisse WHERE id = 1", [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        ).unwrap();
        assert_eq!(attendu, 250.0);
        assert_eq!(ecart, 0.0);
        assert_eq!(statut, "cloturee");
    }

    #[test]
    fn test_cloture_detecte_un_manque() {
        let mut conn = setup();
        vendre(&mut conn, "facture", json!([{ "mode": "especes", "montant": 120 }])).unwrap();
        close_session_impl(&mut conn, 1, 200.0).unwrap();
        let ecart: f64 = conn.query_row("SELECT ecart FROM sessions_caisse WHERE id = 1", [], |r| r.get(0)).unwrap();
        assert_eq!(ecart, -20.0);
    }

    #[test]
    fn test_cloture_ignore_les_autres_sessions() {
        let mut conn = setup();
        conn.execute("INSERT INTO utilisateurs (id, login, password_hash, nom) VALUES (2, 'c2', 'x', 'Caissier 2')", []).unwrap();
        conn.execute("INSERT INTO sessions_caisse (id, caissier_id, fond_initial, statut, magasin_id) VALUES (2, 2, 0, 'ouverte', 1)", []).unwrap();
        create_vente_impl(&mut conn, None, Some(2), ligne(), 0.0, "especes".into(),
            Some(vec![json!({ "mode": "especes", "montant": 500 })]), Some("facture".into()), None, None, Some(1)).unwrap();
        assert_eq!(totaux_especes_session(&conn, 1).unwrap().ventes_especes, 0.0);
        assert_eq!(totaux_especes_session(&conn, 2).unwrap().ventes_especes, 500.0);
    }

    #[test]
    fn test_paiement_sans_detail_utilise_le_ttc() {
        let mut conn = setup();
        create_vente_impl(&mut conn, None, Some(1), ligne(), 10.0, "especes".into(),
            None, Some("facture".into()), None, None, Some(1)).unwrap();
        assert_eq!(totaux_especes_session(&conn, 1).unwrap().ventes_especes, 110.0);
    }

    #[test]
    fn test_credit_en_paiement_fractionne_respecte_le_plafond() {
        let mut conn = setup();
        conn.execute("INSERT INTO clients (id, nom, credit_plafond, credit_actuel) VALUES (1, 'Client', 100, 0)", []).unwrap();
        let depasse = create_vente_impl(&mut conn, Some(1), Some(1), ligne(), 0.0, "especes+credit".into(),
            Some(vec![json!({ "mode": "especes", "montant": 20 }), json!({ "mode": "credit", "montant": 150 })]),
            Some("facture".into()), None, None, Some(1));
        assert!(depasse.unwrap_err().contains("Plafond"));
        create_vente_impl(&mut conn, Some(1), Some(1), ligne(), 0.0, "especes+credit".into(),
            Some(vec![json!({ "mode": "especes", "montant": 40 }), json!({ "mode": "credit", "montant": 80 })]),
            Some("facture".into()), None, None, Some(1)).unwrap();
        let credit: f64 = conn.query_row("SELECT credit_actuel FROM clients WHERE id = 1", [], |r| r.get(0)).unwrap();
        assert_eq!(credit, 80.0);
        assert_eq!(totaux_especes_session(&conn, 1).unwrap().ventes_especes, 40.0);
    }

    #[test]
    fn test_paiements_invalides_refuses() {
        assert!(normaliser_paiements("especes", Some(&[json!({ "mode": "bitcoin", "montant": 10 })]), 0.0).is_err());
        assert!(normaliser_paiements("especes", Some(&[json!({ "mode": "especes", "montant": -5 })]), 0.0).is_err());
        assert!(normaliser_paiements("especes", Some(&[json!({ "mode": "especes", "amount": 10 })]), 0.0).is_err());
        assert!(normaliser_paiements("inconnu", None, 10.0).is_err());
        assert_eq!(normaliser_paiements("carte", None, 42.0).unwrap(), vec![("carte".to_string(), 42.0)]);
    }

    #[test]
    fn test_reprise_des_ventes_de_session_ouverte_a_la_mise_a_jour() {
        let path = std::env::temp_dir().join(format!(
            "supercaisse_test_backfill_{}_{}.db",
            std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ));
        let path_str = path.to_string_lossy().to_string();
        {
            let conn = crate::db::init_db(&path_str).unwrap();
            conn.execute("INSERT INTO articles (id, designation, prix_vente, tva) VALUES (1, 'Article', 100, 20)", []).unwrap();
            conn.execute("INSERT INTO sessions_caisse (id, caissier_id, fond_initial, statut) VALUES (1, 1, 0, 'ouverte')", []).unwrap();
            conn.execute("INSERT INTO sessions_caisse (id, caissier_id, fond_initial, statut) VALUES (2, 1, 0, 'cloturee')", []).unwrap();
            conn.execute_batch("
                INSERT INTO ventes (id, mode_paiement, montant_remise, session_id, dtype, statut) VALUES (1, 'especes', 0, 1, 'facture', 'validee');
                INSERT INTO vente_articles (vente_id, article_id, quantite, prix_unitaire, total_ligne) VALUES (1, 1, 1, 100, 120);
                INSERT INTO ventes (id, mode_paiement, montant_remise, session_id, dtype, statut) VALUES (2, 'especes', 0, 2, 'facture', 'validee');
                INSERT INTO vente_articles (vente_id, article_id, quantite, prix_unitaire, total_ligne) VALUES (2, 1, 1, 100, 120);
            ").unwrap();
        }
        let conn = crate::db::init_db(&path_str).unwrap();
        assert_eq!(totaux_especes_session(&conn, 1).unwrap().ventes_especes, 120.0);
        assert_eq!(totaux_especes_session(&conn, 2).unwrap().ventes_especes, 0.0);
        drop(conn);
        let conn = crate::db::init_db(&path_str).unwrap();
        assert_eq!(totaux_especes_session(&conn, 1).unwrap().ventes_especes, 120.0);
        drop(conn);
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{}", path_str, suffix));
        }
    }
}
