use crate::db::*;
use rusqlite::params;
use tauri::State;

#[tauri::command]
pub fn login(db: State<DbState>, login: String, password: String) -> Result<Option<Utilisateur>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT id, login, nom, role, password_hash FROM utilisateurs WHERE login = ?1"
    ).map_err(|e| e.to_string())?;
    let result = stmt.query_row(params![login], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
        ))
    }).ok();
    match result {
        None => Ok(None),
        Some((id, ulogin, nom, role, hash)) => {
            if !verify_password(&password, &hash) {
                return Ok(None);
            }
            if !hash.starts_with("$argon2") {
                let new_hash = hash_password(&password);
                let _ = conn.execute(
                    "UPDATE utilisateurs SET password_hash = ?1 WHERE id = ?2",
                    params![new_hash, id],
                );
            }
            Ok(Some(Utilisateur { id: Some(id), login: ulogin, nom, role }))
        }
    }
}

#[tauri::command]
pub fn login_pin(db: State<DbState>, pin: String) -> Result<Option<Utilisateur>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT id, login, nom, role, pin_hash FROM utilisateurs WHERE pin_hash IS NOT NULL AND pin_hash != ''"
    ).map_err(|e| e.to_string())?;
    let users: Vec<(i64, String, String, String, String)> = stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
        ))
    }).map_err(|e| e.to_string())?.filter_map(Result::ok).collect();
    for (id, ulogin, nom, role, hash) in users {
        if verify_password(&pin, &hash) {
            return Ok(Some(Utilisateur { id: Some(id), login: ulogin, nom, role }));
        }
    }
    Ok(None)
}

#[tauri::command]
pub fn set_user_pin(db: State<DbState>, user_id: i64, pin: String) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let hash = if pin.is_empty() {
        String::new()
    } else {
        hash_password(&pin)
    };
    conn.execute(
        "UPDATE utilisateurs SET pin_hash = ?1 WHERE id = ?2",
        params![hash, user_id],
    ).map_err(|e| e.to_string())?;
    Ok(())
}
