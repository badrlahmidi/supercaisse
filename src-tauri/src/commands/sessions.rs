use crate::db::*;
use rusqlite::params;
use tauri::State;

use super::default_magasin_id;

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

    Ok(conn.last_insert_rowid())
}

#[tauri::command]
pub fn close_session(db: State<DbState>, session_id: i64, total_especes_declare: f64) -> Result<(), String> {
    let mut conn = db.conn.lock().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    let fond_initial: f64 = tx.query_row(
        "SELECT fond_initial FROM sessions_caisse WHERE id = ?1 AND statut = 'ouverte'",
        params![session_id],
        |row| row.get(0)
    ).map_err(|_| "Session introuvable ou déjà clôturée".to_string())?;

    let ventes_especes: f64 = tx.query_row(
        "SELECT COALESCE(SUM(montant), 0) FROM paiements WHERE ptype = 'especes' AND date >= (SELECT date_ouverture FROM sessions_caisse WHERE id = ?1)",
        params![session_id],
        |row| row.get(0)
    ).unwrap_or(0.0);

    let entrees_caisse: f64 = tx.query_row(
        "SELECT COALESCE(SUM(montant), 0) FROM journal_caisse WHERE jtype = 'entree' AND session_id = ?1",
        params![session_id],
        |row| row.get(0)
    ).unwrap_or(0.0);

    let sorties_caisse: f64 = tx.query_row(
        "SELECT COALESCE(SUM(montant), 0) FROM journal_caisse WHERE jtype = 'sortie' AND session_id = ?1",
        params![session_id],
        |row| row.get(0)
    ).unwrap_or(0.0);

    let total_attendu = fond_initial + ventes_especes + entrees_caisse - sorties_caisse;
    let ecart = total_especes_declare - total_attendu;
    let date_cloture = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    tx.execute(
        "UPDATE sessions_caisse
         SET date_cloture = ?1, total_especes_attendu = ?2, total_especes_declare = ?3, ecart = ?4, statut = 'cloturee'
         WHERE id = ?5",
        params![date_cloture, total_attendu, total_especes_declare, ecart, session_id]
    ).map_err(|e| e.to_string())?;

    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}
