use crate::db::*;
use crate::session::{autoriser, Acces, AuthState};
use rusqlite::params;
use tauri::State;

#[tauri::command(async)]
pub fn get_tables(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
) -> Result<Vec<super::contrats::TableRestaurant>, String> {
    let conn = db.lecture()?;
    let _me = autoriser(&auth, &conn, &token, Acces::Connecte)?;
    let mut stmt = conn
        .prepare("SELECT id, nom, statut, ticket_id FROM tables_resto ORDER BY id")
        .map_err(|e| e.to_string())?;
    let tables = stmt
        .query_map([], |row| {
            Ok(super::contrats::TableRestaurant {
                id: row.get(0)?,
                nom: row.get(1)?,
                statut: row.get(2)?,
                ticket_id: row.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| e.to_string())?;
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
