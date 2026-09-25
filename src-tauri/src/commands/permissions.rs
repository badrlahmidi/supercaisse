use crate::db::*;
use crate::session::{autoriser, Acces, AuthState};
use rusqlite::params;
use tauri::State;

use super::log_audit;

#[tauri::command(async)]
pub fn get_permissions(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    role: String,
) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.lecture()?;
    let me = autoriser(&auth, &conn, &token, Acces::Connecte)?;
    if role != me.role && !me.est_admin() {
        return Err("Accès refusé : permissions d'un autre rôle".to_string());
    }
    let mut stmt = conn.prepare(
        "SELECT role, module, action, allowed FROM permissions WHERE role = ?1 ORDER BY module, action"
    ).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![role], |row| {
            Ok(serde_json::json!({
                "role": row.get::<_, String>(0)?,
                "module": row.get::<_, String>(1)?,
                "action": row.get::<_, String>(2)?,
                "allowed": row.get::<_, i32>(3)? != 0,
            }))
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

#[tauri::command(async)]
pub fn update_permission(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    role: String,
    module: String,
    action: String,
    allowed: bool,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let me = autoriser(&auth, &conn, &token, Acces::Admin)?;
    conn.execute(
        "INSERT INTO permissions (role, module, action, allowed) VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(role, module, action) DO UPDATE SET allowed = ?4",
        params![role, module, action, allowed as i32],
    )
    .map_err(|e| e.to_string())?;
    log_audit(
        &conn,
        Some(me.user_id),
        "modifier_permission",
        &format!(
            "Permission {} / {} / {} → {}",
            role,
            module,
            action,
            if allowed { "autorisé" } else { "refusé" }
        ),
        None,
        None,
    );
    Ok(())
}
