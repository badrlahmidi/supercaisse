use crate::db::*;
use crate::session::{autoriser, Acces, AuthState};
use rusqlite::{params, OptionalExtension};
use tauri::State;

use super::log_audit;

#[tauri::command(async)]
pub fn get_categories(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
) -> Result<Vec<Category>, String> {
    let conn = db.lecture()?;
    let _me = autoriser(&auth, &conn, &token, Acces::Connecte)?;
    let mut stmt = conn
        .prepare("SELECT id, nom, description FROM categories ORDER BY nom")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok(Category {
                id: Some(row.get(0)?),
                nom: row.get(1)?,
                description: row.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

#[tauri::command(async)]
pub fn add_category(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    nom: String,
    description: Option<String>,
) -> Result<i64, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let _me = autoriser(&auth, &conn, &token, Acces::Module("categories", "creer"))?;
    conn.execute(
        "INSERT INTO categories (nom, description) VALUES (?1, ?2)",
        params![nom, description],
    )
    .map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

#[tauri::command(async)]
pub fn update_category(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    id: i64,
    nom: String,
    description: Option<String>,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let _me = autoriser(
        &auth,
        &conn,
        &token,
        Acces::Module("categories", "modifier"),
    )?;
    conn.execute(
        "UPDATE categories SET nom=?1, description=?2 WHERE id=?3",
        params![nom, description, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command(async)]
pub fn delete_category(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    id: i64,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let me = autoriser(
        &auth,
        &conn,
        &token,
        Acces::Module("categories", "modifier"),
    )?;
    let conn = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    let nom: String = conn
        .query_row(
            "SELECT nom FROM categories WHERE id = ?1",
            params![id],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?
        .unwrap_or_else(|| format!("ID {}", id));
    conn.execute("DELETE FROM categories WHERE id = ?1", params![id])
        .map_err(|e| super::erreur_suppression(e, "cette catégorie"))?;
    log_audit(
        &conn,
        Some(me.user_id),
        "supprimer_categorie",
        &format!("Suppression catégorie: {} (ID {})", nom, id),
        Some("categorie"),
        Some(id),
    )?;
    conn.commit().map_err(|e| e.to_string())?;
    Ok(())
}
