use crate::db::*;
use rusqlite::params;
use tauri::State;

use super::log_audit;

#[tauri::command]
pub fn get_categories(db: State<DbState>) -> Result<Vec<Category>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare("SELECT id, nom, description FROM categories ORDER BY nom").map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |row| {
        Ok(Category {
            id: Some(row.get(0)?),
            nom: row.get(1)?,
            description: row.get(2)?,
        })
    }).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_category(db: State<DbState>, nom: String, description: Option<String>) -> Result<i64, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute("INSERT INTO categories (nom, description) VALUES (?1, ?2)", params![nom, description])
        .map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

#[tauri::command]
pub fn update_category(db: State<DbState>, id: i64, nom: String, description: Option<String>) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE categories SET nom=?1, description=?2 WHERE id=?3",
        params![nom, description, id],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn delete_category(db: State<DbState>, id: i64) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let nom: String = conn.query_row(
        "SELECT nom FROM categories WHERE id = ?1", params![id], |r| r.get(0)
    ).unwrap_or_else(|_| format!("ID {}", id));
    conn.execute("DELETE FROM categories WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    log_audit(&conn, None, "supprimer_categorie",
        &format!("Suppression catégorie: {} (ID {})", nom, id),
        Some("categorie"), Some(id));
    Ok(())
}
