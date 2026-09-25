use crate::db::*;
use tauri::State;
use crate::session::{autoriser, Acces, AuthState};

#[tauri::command]
pub fn get_audit_log(db: State<DbState>, auth: State<AuthState>, token: String, debut: Option<String>, fin: Option<String>, action_filter: Option<String>) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let _me = autoriser(&auth, &conn, &token, Acces::Module("audit", "voir"))?;
    let mut where_clause = String::new();
    let mut qp: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    if let Some(d) = &debut { if !d.is_empty() { where_clause.push_str(" AND a.date >= ?"); qp.push(Box::new(d.clone())); } }
    if let Some(f) = &fin { if !f.is_empty() { where_clause.push_str(" AND a.date <= ?"); qp.push(Box::new(format!("{} 23:59:59", f))); } }
    if let Some(af) = &action_filter { if !af.is_empty() { where_clause.push_str(" AND a.action = ?"); qp.push(Box::new(af.clone())); } }
    let sql = format!(
        "SELECT a.id, a.date, a.utilisateur_id, a.action, a.detail, a.reference_type, a.reference_id, u.nom as user_nom
         FROM audit_log a LEFT JOIN utilisateurs u ON a.utilisateur_id = u.id
         WHERE 1=1 {} ORDER BY a.date DESC LIMIT 500", where_clause
    );
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let pr: Vec<&dyn rusqlite::types::ToSql> = qp.iter().map(|p| p.as_ref()).collect();
    let rows = stmt.query_map(pr.as_slice(), |row| {
        Ok(serde_json::json!({
            "id": row.get::<_, i64>(0)?,
            "date": row.get::<_, String>(1)?,
            "utilisateur_id": row.get::<_, Option<i64>>(2)?,
            "action": row.get::<_, String>(3)?,
            "detail": row.get::<_, Option<String>>(4)?,
            "reference_type": row.get::<_, Option<String>>(5)?,
            "reference_id": row.get::<_, Option<i64>>(6)?,
            "user_nom": row.get::<_, Option<String>>(7)?,
        }))
    }).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}
