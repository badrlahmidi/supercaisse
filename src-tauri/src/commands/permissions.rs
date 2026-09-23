use crate::db::*;
use rusqlite::params;
use tauri::State;

#[tauri::command]
pub fn get_permissions(db: State<DbState>, role: String) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT role, module, action, allowed FROM permissions WHERE role = ?1 ORDER BY module, action"
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map(params![role], |row| {
        Ok(serde_json::json!({
            "role": row.get::<_, String>(0)?,
            "module": row.get::<_, String>(1)?,
            "action": row.get::<_, String>(2)?,
            "allowed": row.get::<_, i32>(3)? != 0,
        }))
    }).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_permission(db: State<DbState>, role: String, module: String, action: String, allowed: bool) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO permissions (role, module, action, allowed) VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(role, module, action) DO UPDATE SET allowed = ?4",
        params![role, module, action, allowed as i32],
    ).map_err(|e| e.to_string())?;
    Ok(())
}
