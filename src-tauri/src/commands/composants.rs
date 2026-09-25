use crate::db::*;
use crate::session::{autoriser, Acces, AuthState};
use rusqlite::params;
use tauri::State;

#[tauri::command]
pub fn add_article_composant(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    article_id: i64,
    composant_id: i64,
    quantite: f64,
) -> Result<i64, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let _me = autoriser(&auth, &conn, &token, Acces::Module("articles", "modifier"))?;
    if article_id == composant_id {
        return Err("Un article ne peut pas être son propre composant".to_string());
    }
    conn.execute(
        "INSERT INTO article_composants (article_id, composant_id, quantite) VALUES (?1, ?2, ?3)",
        params![article_id, composant_id, quantite],
    )
    .map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

#[tauri::command]
pub fn get_article_composants(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    article_id: i64,
) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let _me = autoriser(&auth, &conn, &token, Acces::Connecte)?;
    let mut stmt = conn
        .prepare(
            "SELECT c.id, c.composant_id, a.designation, a.stock, c.quantite
         FROM article_composants c
         JOIN articles a ON a.id = c.composant_id
         WHERE c.article_id = ?1 ORDER BY a.designation",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![article_id], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, i64>(0)?,
                "composant_id": row.get::<_, i64>(1)?,
                "designation": row.get::<_, String>(2)?,
                "stock": row.get::<_, f64>(3)?,
                "quantite": row.get::<_, f64>(4)?,
            }))
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_article_composant_quantite(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    id: i64,
    quantite: f64,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let _me = autoriser(&auth, &conn, &token, Acces::Module("articles", "modifier"))?;
    conn.execute(
        "UPDATE article_composants SET quantite=?1 WHERE id=?2",
        params![quantite, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn delete_article_composant(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    id: i64,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let _me = autoriser(&auth, &conn, &token, Acces::Module("articles", "modifier"))?;
    conn.execute("DELETE FROM article_composants WHERE id=?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}
