use super::calcul::{montant_positif, round2};
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

#[tauri::command(async)]
pub fn get_caisses(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.lecture()?;
    let _me = autoriser(&auth, &conn, &token, Acces::Module("journal", "voir"))?;
    let mut stmt = conn
        .prepare(
            "SELECT c.id, c.nom, c.utilisateur_id, c.statut, c.ouverture_date, c.fermeture_date,
                c.fond_initial, c.recettes_especes, c.recettes_cb, c.recettes_cheque,
                c.recettes_virement, c.depenses, c.ecart, c.note, u.nom AS utilisateur_nom
         FROM caisses c
         LEFT JOIN utilisateurs u ON c.utilisateur_id = u.id
         ORDER BY c.id DESC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, i64>(0)?,
                "nom": row.get::<_, String>(1)?,
                "utilisateur_id": row.get::<_, Option<i64>>(2)?,
                "statut": row.get::<_, String>(3)?,
                "ouverture_date": row.get::<_, Option<String>>(4)?,
                "fermeture_date": row.get::<_, Option<String>>(5)?,
                "fond_initial": row.get::<_, f64>(6)?,
                "recettes_especes": row.get::<_, f64>(7)?,
                "recettes_cb": row.get::<_, f64>(8)?,
                "recettes_cheque": row.get::<_, f64>(9)?,
                "recettes_virement": row.get::<_, f64>(10)?,
                "depenses": row.get::<_, f64>(11)?,
                "ecart": row.get::<_, f64>(12)?,
                "note": row.get::<_, Option<String>>(13)?,
                "utilisateur_nom": row.get::<_, Option<String>>(14)?,
            }))
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

#[tauri::command(async)]
pub fn open_caisse(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    nom: String,
    fond_initial: f64,
    utilisateur_id: Option<i64>,
) -> Result<i64, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let _me = autoriser(&auth, &conn, &token, Acces::Module("journal", "creer"))?;
    let fond_initial = montant_positif("Fond de caisse", fond_initial)?;
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    conn.execute(
        "INSERT INTO caisses (nom, utilisateur_id, statut, ouverture_date, fond_initial) VALUES (?1, ?2, 'ouverte', ?3, ?4)",
        params![nom, utilisateur_id, now, fond_initial],
    ).map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

#[tauri::command(async)]
pub fn close_caisse(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    id: i64,
    note: Option<String>,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let _me = autoriser(&auth, &conn, &token, Acces::Module("journal", "modifier"))?;

    let ouverture_date: String = conn
        .query_row(
            "SELECT ouverture_date FROM caisses WHERE id = ?1 AND statut = 'ouverte'",
            params![id],
            |row| row.get(0),
        )
        .map_err(|_| "Caisse introuvable ou déjà fermée".to_string())?;

    let Recettes {
        especes: recettes_especes,
        cb: recettes_cb,
        cheque: recettes_cheque,
        virement: recettes_virement,
    } = recettes_depuis(&conn, &ouverture_date)?;

    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    conn.execute(
        "UPDATE caisses SET statut = 'fermee', fermeture_date = ?1, recettes_especes = ?2, recettes_cb = ?3, recettes_cheque = ?4, recettes_virement = ?5, note = ?6 WHERE id = ?7",
        params![now, recettes_especes, recettes_cb, recettes_cheque, recettes_virement, note, id],
    ).map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command(async)]
pub fn get_tresorerie(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
) -> Result<serde_json::Value, String> {
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

    let fetch_recettes = |since: &str| -> Result<serde_json::Value, String> {
        let r = recettes_depuis(&conn, since)?;
        Ok(serde_json::json!({
            "especes": r.especes,
            "cb": r.cb,
            "cheque": r.cheque,
            "virement": r.virement,
            "total": round2(r.especes + r.cb + r.cheque + r.virement),
        }))
    };

    let jour = fetch_recettes(&today_start)?;
    let semaine = fetch_recettes(&week_start)?;
    let mois = fetch_recettes(&month_start)?;

    Ok(serde_json::json!({
        "jour": jour,
        "semaine": semaine,
        "mois": mois,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::ventes::{annuler_vente_impl, convert_document_impl, create_vente_impl};
    use serde_json::json;

    fn document(conn: &mut Connection, dtype: &str, mode: &str) -> i64 {
        create_vente_impl(
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
        .unwrap()["id"]
            .as_i64()
            .unwrap()
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
        annuler_vente_impl(&mut conn, annulee, None, None).unwrap();
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
}
