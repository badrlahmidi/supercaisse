use crate::db::*;
use rusqlite::params;
use tauri::State;

#[tauri::command]
pub fn get_paiements(db: State<DbState>, client_id: Option<i64>) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let (sql, params_vec) = match client_id {
        Some(cid) => (
            "SELECT p.id, p.client_id, p.date, p.montant, p.type, p.reference, c.nom as client_nom
             FROM paiements p JOIN clients c ON p.client_id = c.id WHERE p.client_id = ?1 ORDER BY p.date DESC LIMIT 100".to_string(),
            vec![Box::new(cid) as Box<dyn rusqlite::types::ToSql>]
        ),
        None => (
            "SELECT p.id, p.client_id, p.date, p.montant, p.type, p.reference, c.nom as client_nom
             FROM paiements p JOIN clients c ON p.client_id = c.id ORDER BY p.date DESC LIMIT 200".to_string(),
            vec![]
        ),
    };
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let pr: Vec<&dyn rusqlite::types::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
    let map_row = |row: &rusqlite::Row| -> rusqlite::Result<serde_json::Value> {
        Ok(serde_json::json!({
            "id": row.get::<_, i64>(0)?,
            "client_id": row.get::<_, i64>(1)?,
            "date": row.get::<_, String>(2)?,
            "montant": row.get::<_, f64>(3)?,
            "type": row.get::<_, String>(4)?,
            "reference": row.get::<_, Option<String>>(5)?,
            "client_nom": row.get::<_, Option<String>>(6)?,
        }))
    };
    let rows = stmt.query_map(pr.as_slice(), map_row).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_paiement(db: State<DbState>, client_id: i64, montant: f64, ptype: String, reference: Option<String>) -> Result<i64, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO paiements (client_id, montant, type, reference) VALUES (?1, ?2, ?3, ?4)",
        params![client_id, montant, ptype, reference],
    ).map_err(|e| e.to_string())?;
    conn.execute("UPDATE clients SET credit_actuel = credit_actuel - ?1 WHERE id = ?2",
        params![montant, client_id]).map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}
