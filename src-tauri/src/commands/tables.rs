use crate::db::*;
use crate::session::{autoriser, Acces, AuthState};
use rusqlite::params;
use tauri::State;

#[tauri::command(async)]
pub fn get_tables(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.lecture()?;
    let _me = autoriser(&auth, &conn, &token, Acces::Connecte)?;
    let mut stmt = conn
        .prepare("SELECT id, nom, statut, ticket_id FROM tables_resto ORDER BY id")
        .map_err(|e| e.to_string())?;
    let tables = stmt
        .query_map([], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, i64>(0)?,
                "nom": row.get::<_, String>(1)?,
                "statut": row.get::<_, String>(2)?,
                "ticket_id": row.get::<_, Option<String>>(3)?,
            }))
        })
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect::<Vec<_>>();
    Ok(tables)
}

#[tauri::command(async)]
pub fn update_table_status(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    id: i64,
    statut: String,
    ticket_id: Option<String>,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let _me = autoriser(&auth, &conn, &token, Acces::Connecte)?;
    conn.execute(
        "UPDATE tables_resto SET statut = ?1, ticket_id = ?2 WHERE id = ?3",
        params![statut, ticket_id, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
