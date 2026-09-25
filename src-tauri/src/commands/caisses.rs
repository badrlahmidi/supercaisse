use crate::db::*;
use rusqlite::params;
use tauri::State;
use crate::session::{autoriser, Acces, AuthState};

#[tauri::command]
pub fn get_caisses(db: State<DbState>, auth: State<AuthState>, token: String) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let _me = autoriser(&auth, &conn, &token, Acces::Module("journal", "voir"))?;
    let mut stmt = conn.prepare(
        "SELECT c.id, c.nom, c.utilisateur_id, c.statut, c.ouverture_date, c.fermeture_date,
                c.fond_initial, c.recettes_especes, c.recettes_cb, c.recettes_cheque,
                c.recettes_virement, c.depenses, c.ecart, c.note, u.nom AS utilisateur_nom
         FROM caisses c
         LEFT JOIN utilisateurs u ON c.utilisateur_id = u.id
         ORDER BY c.id DESC"
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |row| {
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
    }).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_caisse(db: State<DbState>, auth: State<AuthState>, token: String, nom: String, fond_initial: f64, utilisateur_id: Option<i64>) -> Result<i64, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let _me = autoriser(&auth, &conn, &token, Acces::Module("journal", "creer"))?;
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    conn.execute(
        "INSERT INTO caisses (nom, utilisateur_id, statut, ouverture_date, fond_initial) VALUES (?1, ?2, 'ouverte', ?3, ?4)",
        params![nom, utilisateur_id, now, fond_initial],
    ).map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

#[tauri::command]
pub fn close_caisse(db: State<DbState>, auth: State<AuthState>, token: String, id: i64, note: Option<String>) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let _me = autoriser(&auth, &conn, &token, Acces::Module("journal", "modifier"))?;

    let ouverture_date: String = conn.query_row(
        "SELECT ouverture_date FROM caisses WHERE id = ?1 AND statut = 'ouverte'",
        params![id],
        |row| row.get(0),
    ).map_err(|_| "Caisse introuvable ou déjà fermée".to_string())?;

    let recettes_especes: f64 = conn.query_row(
        "SELECT COALESCE(SUM(montant_total - montant_remise), 0) FROM ventes WHERE mode_paiement = 'especes' AND statut = 'validee' AND date >= ?1",
        params![ouverture_date],
        |row| row.get(0),
    ).unwrap_or(0.0);

    let recettes_cb: f64 = conn.query_row(
        "SELECT COALESCE(SUM(montant_total - montant_remise), 0) FROM ventes WHERE mode_paiement = 'cb' AND statut = 'validee' AND date >= ?1",
        params![ouverture_date],
        |row| row.get(0),
    ).unwrap_or(0.0);

    let recettes_cheque: f64 = conn.query_row(
        "SELECT COALESCE(SUM(montant_total - montant_remise), 0) FROM ventes WHERE mode_paiement = 'cheque' AND statut = 'validee' AND date >= ?1",
        params![ouverture_date],
        |row| row.get(0),
    ).unwrap_or(0.0);

    let recettes_virement: f64 = conn.query_row(
        "SELECT COALESCE(SUM(montant_total - montant_remise), 0) FROM ventes WHERE mode_paiement = 'virement' AND statut = 'validee' AND date >= ?1",
        params![ouverture_date],
        |row| row.get(0),
    ).unwrap_or(0.0);

    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    conn.execute(
        "UPDATE caisses SET statut = 'fermee', fermeture_date = ?1, recettes_especes = ?2, recettes_cb = ?3, recettes_cheque = ?4, recettes_virement = ?5, note = ?6 WHERE id = ?7",
        params![now, recettes_especes, recettes_cb, recettes_cheque, recettes_virement, note, id],
    ).map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn get_tresorerie(db: State<DbState>, auth: State<AuthState>, token: String) -> Result<serde_json::Value, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
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
        let especes: f64 = conn.query_row(
            "SELECT COALESCE(SUM(montant_total - montant_remise), 0) FROM ventes WHERE mode_paiement = 'especes' AND statut = 'validee' AND date >= ?1",
            params![since], |r| r.get(0),
        ).unwrap_or(0.0);
        let cb: f64 = conn.query_row(
            "SELECT COALESCE(SUM(montant_total - montant_remise), 0) FROM ventes WHERE mode_paiement = 'cb' AND statut = 'validee' AND date >= ?1",
            params![since], |r| r.get(0),
        ).unwrap_or(0.0);
        let cheque: f64 = conn.query_row(
            "SELECT COALESCE(SUM(montant_total - montant_remise), 0) FROM ventes WHERE mode_paiement = 'cheque' AND statut = 'validee' AND date >= ?1",
            params![since], |r| r.get(0),
        ).unwrap_or(0.0);
        let virement: f64 = conn.query_row(
            "SELECT COALESCE(SUM(montant_total - montant_remise), 0) FROM ventes WHERE mode_paiement = 'virement' AND statut = 'validee' AND date >= ?1",
            params![since], |r| r.get(0),
        ).unwrap_or(0.0);
        Ok(serde_json::json!({
            "especes": especes,
            "cb": cb,
            "cheque": cheque,
            "virement": virement,
            "total": especes + cb + cheque + virement,
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
