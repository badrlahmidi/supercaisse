use crate::db::*;
use rusqlite::params;
use tauri::State;

use super::log_audit;

#[tauri::command]
pub fn get_utilisateurs(db: State<DbState>) -> Result<Vec<Utilisateur>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare("SELECT id, login, nom, role FROM utilisateurs ORDER BY nom")
        .map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |row| {
        Ok(Utilisateur {
            id: Some(row.get(0)?),
            login: row.get(1)?,
            nom: row.get(2)?,
            role: row.get(3)?,
        })
    }).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_utilisateur(db: State<DbState>, login: String, nom: String, role: String, password: String) -> Result<i64, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let hash = hash_password(&password);
    conn.execute(
        "INSERT INTO utilisateurs (login, password_hash, nom, role) VALUES (?1, ?2, ?3, ?4)",
        params![login, hash, nom, role],
    ).map_err(|e| e.to_string())?;
    let id = conn.last_insert_rowid();
    log_audit(&conn, None, "ajouter_utilisateur",
        &format!("Nouvel utilisateur: {} ({}) - rôle {}", nom, login, role),
        Some("utilisateur"), Some(id));
    Ok(id)
}

#[tauri::command]
pub fn update_utilisateur(db: State<DbState>, id: i64, login: String, nom: String, role: String, password: Option<String>) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    match password {
        Some(ref pwd) if !pwd.is_empty() => {
            let hash = hash_password(pwd);
            conn.execute(
                "UPDATE utilisateurs SET login=?1, nom=?2, role=?3, password_hash=?4 WHERE id=?5",
                params![login, nom, role, hash, id],
            ).map_err(|e| e.to_string())?;
        }
        _ => {
            conn.execute(
                "UPDATE utilisateurs SET login=?1, nom=?2, role=?3 WHERE id=?4",
                params![login, nom, role, id],
            ).map_err(|e| e.to_string())?;
        }
    }
    log_audit(&conn, None, "modifier_utilisateur",
        &format!("Modification utilisateur #{}: {} ({}) - rôle {}", id, nom, login, role),
        Some("utilisateur"), Some(id));
    Ok(())
}

#[tauri::command]
pub fn delete_utilisateur(db: State<DbState>, id: i64) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let nom: String = conn.query_row(
        "SELECT nom FROM utilisateurs WHERE id = ?1", params![id], |r| r.get(0)
    ).unwrap_or_else(|_| format!("ID {}", id));
    conn.execute("DELETE FROM utilisateurs WHERE id=?1", params![id])
        .map_err(|e| e.to_string())?;
    log_audit(&conn, None, "supprimer_utilisateur",
        &format!("Suppression utilisateur: {} (ID {})", nom, id),
        Some("utilisateur"), Some(id));
    Ok(())
}
