use crate::db::*;
use rusqlite::params;
use tauri::State;
use crate::session::{autoriser, Acces, AuthState};

#[tauri::command]
pub fn get_journal_caisse(db: State<DbState>, auth: State<AuthState>, token: String, debut: Option<String>, fin: Option<String>) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let _me = autoriser(&auth, &conn, &token, Acces::Module("journal", "voir"))?;
    let mut where_clause = String::new();
    let mut qp: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    if let Some(d) = &debut { if !d.is_empty() { where_clause.push_str(" AND j.date >= ?"); qp.push(Box::new(d.clone())); } }
    if let Some(f) = &fin { if !f.is_empty() { where_clause.push_str(" AND j.date <= ?"); qp.push(Box::new(f.clone())); } }
    let sql = format!(
        "SELECT j.id, j.date, j.utilisateur_id, j.jtype, j.montant, j.description, u.nom as user_nom
         FROM journal_caisse j LEFT JOIN utilisateurs u ON j.utilisateur_id = u.id
         WHERE 1=1 {} ORDER BY j.date DESC LIMIT 200", where_clause
    );
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let pr: Vec<&dyn rusqlite::types::ToSql> = qp.iter().map(|p| p.as_ref()).collect();
    let rows = stmt.query_map(pr.as_slice(), |row| {
        Ok(serde_json::json!({
            "id": row.get::<_, i64>(0)?,
            "date": row.get::<_, String>(1)?,
            "utilisateur_id": row.get::<_, Option<i64>>(2)?,
            "jtype": row.get::<_, String>(3)?,
            "montant": row.get::<_, f64>(4)?,
            "description": row.get::<_, Option<String>>(5)?,
            "user_nom": row.get::<_, Option<String>>(6)?,
        }))
    }).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_journal_caisse(db: State<DbState>, auth: State<AuthState>, token: String, jtype: String, montant: f64, description: Option<String>) -> Result<i64, String> {
    let mut conn = db.conn.lock().map_err(|e| e.to_string())?;
    let me = autoriser(&auth, &conn, &token, Acces::Module("journal", "creer"))?;
    let utilisateur_id = Some(me.user_id);
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    let session_id: Option<i64> = if let Some(uid) = utilisateur_id {
        tx.query_row(
            "SELECT id FROM sessions_caisse WHERE caissier_id = ?1 AND statut = 'ouverte'",
            params![uid],
            |row| row.get(0)
        ).ok()
    } else { None };

    tx.execute(
        "INSERT INTO journal_caisse (utilisateur_id, jtype, montant, description, session_id) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![utilisateur_id, jtype, montant, description, session_id],
    ).map_err(|e| e.to_string())?;
    let id = tx.last_insert_rowid();
    tx.commit().map_err(|e| e.to_string())?;
    Ok(id)
}
