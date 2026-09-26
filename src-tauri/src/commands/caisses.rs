use super::calcul::{en_dh, round2};
use crate::db::*;
use crate::session::{autoriser, Acces, AuthState};
use rusqlite::{params, Connection};
use tauri::State;

#[derive(Debug, Default, PartialEq)]
pub(crate) struct Recettes {
    pub especes: f64,
    pub cb: f64,
    pub cheque: f64,
    pub virement: f64,
}

pub(crate) fn recettes_depuis(conn: &Connection, depuis: &str) -> Result<Recettes, String> {
    let sql = concat!(
        "SELECT mode, COALESCE(SUM(ROUND((montant) * 100)) / 100.0, 0) FROM (
            SELECT vp.mode AS mode, vp.montant AS montant
            FROM vente_paiements vp
            JOIN ventes v ON v.id = COALESCE(
                (SELECT f.id FROM ventes f WHERE f.source_vente_id = vp.vente_id AND f.dtype = 'facture' LIMIT 1),
                vp.vente_id)
            WHERE v.date >= ?1 AND ", filtre_ca!(), "
            UNION ALL
            SELECT v.mode_paiement AS mode, v.montant_total - v.montant_remise AS montant
            FROM ventes v
            WHERE v.date >= ?1 AND ", filtre_ca!(), "
              AND NOT EXISTS (SELECT 1 FROM vente_paiements vp2 WHERE vp2.vente_id = v.id OR (v.dtype = 'facture' AND vp2.vente_id = v.source_vente_id))
        ) GROUP BY mode"
    );
    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    let lignes = stmt
        .query_map(params![depuis], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, f64>(1)?))
        })
        .map_err(|e| e.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| e.to_string())?;
    let mut r = Recettes::default();
    for (mode, montant) in lignes {
        match mode.as_str() {
            "especes" => r.especes += montant,
            "cb" | "carte" => r.cb += montant,
            "cheque" => r.cheque += montant,
            "virement" => r.virement += montant,
            _ => {}
        }
    }
    Ok(Recettes {
        especes: round2(r.especes),
        cb: round2(r.cb),
        cheque: round2(r.cheque),
        virement: round2(r.virement),
    })
}

pub(crate) fn lister_sessions_caisse(
    conn: &Connection,
) -> Result<Vec<super::contrats::SessionSupervision>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT s.id, s.caissier_id, u.nom, m.nom, s.statut, s.date_ouverture, s.date_cloture,
                s.fond_initial,
                COALESCE((SELECT SUM(ROUND(vp.montant * 100)) FROM vente_paiements vp JOIN ventes v ON v.id = vp.vente_id
                          WHERE vp.session_id = s.id AND v.statut != 'annulee' AND vp.mode = 'especes'), 0),
                COALESCE((SELECT SUM(ROUND(vp.montant * 100)) FROM vente_paiements vp JOIN ventes v ON v.id = vp.vente_id
                          WHERE vp.session_id = s.id AND v.statut != 'annulee' AND vp.mode IN ('carte', 'cb')), 0),
                COALESCE((SELECT SUM(ROUND(vp.montant * 100)) FROM vente_paiements vp JOIN ventes v ON v.id = vp.vente_id
                          WHERE vp.session_id = s.id AND v.statut != 'annulee' AND vp.mode = 'cheque'), 0),
                COALESCE((SELECT SUM(ROUND(vp.montant * 100)) FROM vente_paiements vp JOIN ventes v ON v.id = vp.vente_id
                          WHERE vp.session_id = s.id AND v.statut != 'annulee' AND vp.mode = 'virement'), 0),
                COALESCE((SELECT SUM(ROUND(ABS(j.montant) * 100)) FROM journal_caisse j
                          WHERE j.session_id = s.id AND j.jtype = 'sortie'), 0),
                COALESCE((SELECT SUM(ROUND(j.montant * 100)) FROM journal_caisse j
                          WHERE j.session_id = s.id AND j.jtype = 'entree'), 0),
                s.total_especes_attendu, s.total_especes_declare, s.ecart
             FROM sessions_caisse s
             LEFT JOIN utilisateurs u ON u.id = s.caissier_id
             LEFT JOIN magasins m ON m.id = s.magasin_id
             ORDER BY (s.statut = 'ouverte') DESC, s.id DESC
             LIMIT 200",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            let cloturee = row.get::<_, String>(4)? != "ouverte";
            let centimes = |i: usize| -> rusqlite::Result<f64> {
                Ok(en_dh(row.get::<_, f64>(i)?.round() as i64))
            };
            Ok(super::contrats::SessionSupervision {
                id: row.get(0)?,
                caissier_id: row.get(1)?,
                caissier_nom: row.get(2)?,
                magasin_nom: row.get(3)?,
                statut: row.get(4)?,
                date_ouverture: row.get(5)?,
                date_cloture: row.get(6)?,
                fond_initial: row.get(7)?,
                recettes_especes: centimes(8)?,
                recettes_cb: centimes(9)?,
                recettes_cheque: centimes(10)?,
                recettes_virement: centimes(11)?,
                sorties: centimes(12)?,
                entrees: centimes(13)?,
                especes_attendu: if cloturee { row.get(14)? } else { None },
                especes_declare: if cloturee { row.get(15)? } else { None },
                ecart: if cloturee { row.get(16)? } else { None },
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

#[tauri::command(async)]
pub fn get_caisses(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
) -> Result<Vec<super::contrats::SessionSupervision>, String> {
    let conn = db.lecture()?;
    let _me = autoriser(&auth, &conn, &token, Acces::Module("journal", "voir"))?;
    lister_sessions_caisse(&conn)
}

#[tauri::command(async)]
pub fn get_tresorerie(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
) -> Result<super::contrats::Tresorerie, String> {
    let conn = db.lecture()?;
    let _me = autoriser(&auth, &conn, &token, Acces::Module("rapports", "voir"))?;

    let today_start = chrono::Local::now().format("%Y-%m-%d 00:00:00").to_string();

    let week_start = {
        use chrono::Datelike;
        let now = chrono::Local::now();
        let weekday = now.weekday().num_days_from_monday();
        let monday = now - chrono::Duration::days(weekday as i64);
        monday.format("%Y-%m-%d 00:00:00").to_string()
    };

    let month_start = chrono::Local::now().format("%Y-%m-01 00:00:00").to_string();

    let fetch_recettes = |since: &str| -> Result<super::contrats::RecettesPeriode, String> {
        let r = recettes_depuis(&conn, since)?;
        Ok(super::contrats::RecettesPeriode {
            especes: r.especes,
            cb: r.cb,
            cheque: r.cheque,
            virement: r.virement,
            total: round2(r.especes + r.cb + r.cheque + r.virement),
        })
    };

    let jour = fetch_recettes(&today_start)?;
    let semaine = fetch_recettes(&week_start)?;
    let mois = fetch_recettes(&month_start)?;

    Ok(super::contrats::Tresorerie {
        jour,
        semaine,
        mois,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::ventes::{annuler_vente_impl, convert_document_impl};
    use serde_json::json;

    fn document(conn: &mut Connection, dtype: &str, mode: &str) -> i64 {
        crate::commands::ventes::vendre_json(
            conn,
            Some(1),
            Some(1),
            vec![json!({ "article_id": 1, "quantite": 1 })],
            None,
            mode.into(),
            Some(vec![json!({ "mode": mode, "montant": 120 })]),
            Some(dtype.into()),
            None,
            Some(1),
        )
        .unwrap()
        .id
    }

    fn scenario() -> Connection {
        let mut conn = crate::db::init_db(":memory:").unwrap();
        conn.execute_batch(
            "
            INSERT INTO articles (id, designation, prix_vente, tva) VALUES (1, 'Huile', 100, 20);
            INSERT INTO clients (id, nom) VALUES (1, 'Client');
            UPDATE settings SET value = 'false' WHERE key = 'fidelite_actif';
            INSERT INTO article_stocks (article_id, magasin_id, quantite) SELECT id, 1, 100 FROM articles; UPDATE articles SET stock = 100;
        ",
        )
        .unwrap();
        document(&mut conn, "devis", "especes");
        document(&mut conn, "commande", "especes");
        let bl = document(&mut conn, "bl", "credit");
        convert_document_impl(&mut conn, bl, "facture".into()).unwrap();
        let facture = document(&mut conn, "facture", "especes");
        convert_document_impl(&mut conn, facture, "avoir".into()).unwrap();
        let annulee = document(&mut conn, "facture", "especes");
        annuler_vente_impl(&mut conn, annulee, None, Some("Test")).unwrap();
        document(&mut conn, "facture", "carte");
        conn
    }

    #[test]
    fn test_chiffre_d_affaires_sans_devis_ni_double_comptage() {
        let conn = scenario();
        let ca: f64 = conn.query_row(
            concat!("SELECT COALESCE(SUM(ROUND((v.montant_total - v.montant_remise) * 100)) / 100.0, 0) FROM ventes v WHERE ", filtre_ca!()), [], |r| r.get(0),
        ).unwrap();
        assert_eq!(ca, 240.0);
        let quantite: f64 = conn
            .query_row(
                concat!(
                    "SELECT COALESCE(SUM(",
                    quantite_signee!("va"),
                    "), 0) FROM vente_articles va JOIN ventes v ON v.id = va.vente_id WHERE ",
                    filtre_ca!()
                ),
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(quantite, 2.0);
        let nb_devis: i64 = conn
            .query_row(
                concat!(
                    "SELECT COUNT(*) FROM ventes v WHERE v.dtype IN ('devis', 'commande') AND ",
                    filtre_ca!()
                ),
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(nb_devis, 0);
    }

    #[test]
    fn test_recettes_par_mode() {
        let conn = scenario();
        let r = recettes_depuis(&conn, "2000-01-01").unwrap();
        assert_eq!(
            r,
            Recettes {
                especes: 0.0,
                cb: 120.0,
                cheque: 0.0,
                virement: 0.0
            }
        );
    }

    #[test]
    fn test_supervision_des_sessions() {
        let mut conn = crate::db::init_db(":memory:").unwrap();
        conn.execute_batch(
            "
            INSERT INTO articles (id, designation, prix_vente, tva) VALUES (1, 'Huile', 100, 20);
            INSERT INTO article_stocks (article_id, magasin_id, quantite) VALUES (1, 1, 100);
            UPDATE articles SET stock = 100;
            UPDATE settings SET value = 'false' WHERE key = 'fidelite_actif';
            INSERT INTO sessions_caisse (id, caissier_id, fond_initial, statut, magasin_id) VALUES (1, 1, 200, 'ouverte', 1);
            INSERT INTO utilisateurs (id, login, password_hash, nom, role) VALUES (2, 'caisse2', 'x', 'Caisse 2', 'caissier');
            INSERT INTO sessions_caisse (id, caissier_id, fond_initial, statut, magasin_id) VALUES (2, 2, 50, 'ouverte', 1);
        ",
        )
        .unwrap();
        let vendre = |conn: &mut Connection, paiements: serde_json::Value, caissier: i64| {
            crate::commands::ventes::vendre_json(
                conn,
                None,
                Some(caissier),
                vec![json!({ "article_id": 1, "quantite": 1 })],
                None,
                "especes".into(),
                Some(paiements.as_array().unwrap().clone()),
                Some("facture".into()),
                None,
                Some(1),
            )
            .unwrap()
            .id
        };
        vendre(
            &mut conn,
            json!([{ "mode": "especes", "montant": 70 }, { "mode": "carte", "montant": 50 }]),
            1,
        );
        let annulee = vendre(&mut conn, json!([{ "mode": "especes", "montant": 120 }]), 1);
        conn.execute(
            "UPDATE ventes SET statut = 'annulee' WHERE id = ?1",
            params![annulee],
        )
        .unwrap();
        vendre(&mut conn, json!([{ "mode": "cheque", "montant": 120 }]), 2);
        conn.execute(
            "INSERT INTO journal_caisse (jtype, montant, session_id) VALUES ('sortie', -15, 1), ('entree', 10, 1)",
            [],
        )
        .unwrap();
        crate::commands::sessions::close_session_impl(&mut conn, 1, 250.0, None).unwrap();

        let sessions = lister_sessions_caisse(&conn).unwrap();
        assert_eq!(sessions.len(), 2);
        let ouverte = &sessions[0];
        assert_eq!((ouverte.id, ouverte.statut.as_str()), (2, "ouverte"));
        assert_eq!(ouverte.recettes_cheque, 120.0);
        assert_eq!(ouverte.ecart, None);
        let close = &sessions[1];
        assert_eq!(close.recettes_especes, 70.0);
        assert_eq!(close.recettes_cb, 50.0);
        assert_eq!(close.sorties, 15.0);
        assert_eq!(close.entrees, 10.0);
        assert_eq!(close.especes_attendu, Some(265.0));
        assert_eq!(close.especes_declare, Some(250.0));
        assert_eq!(close.ecart, Some(-15.0));
        assert!(close.date_cloture.is_some());
    }
}
