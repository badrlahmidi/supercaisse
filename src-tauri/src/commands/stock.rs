use crate::db::*;
use tauri::State;
use crate::session::{autoriser, Acces, AuthState};

#[tauri::command]
pub fn get_mouvements_stock(db: State<DbState>, auth: State<AuthState>, token: String, article_id: Option<i64>, debut: Option<String>, fin: Option<String>) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let _me = autoriser(&auth, &conn, &token, Acces::Module("stock", "voir"))?;
    let mut where_clause = String::new();
    let mut qp: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    if let Some(aid) = article_id {
        where_clause.push_str(" AND m.article_id = ?");
        qp.push(Box::new(aid));
    }
    if let Some(d) = &debut { if !d.is_empty() { where_clause.push_str(" AND m.date >= ?"); qp.push(Box::new(d.clone())); } }
    if let Some(f) = &fin { if !f.is_empty() { where_clause.push_str(" AND m.date <= ?"); qp.push(Box::new(f.clone())); } }
    let sql = format!(
        "SELECT m.id, m.date, m.article_id, a.designation, m.quantite, m.mtype, m.reference_id, m.reference_type
         FROM mouvements_stock m
         JOIN articles a ON m.article_id = a.id
         WHERE 1=1 {} ORDER BY m.date DESC LIMIT 200", where_clause
    );
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let pr: Vec<&dyn rusqlite::types::ToSql> = qp.iter().map(|p| p.as_ref()).collect();
    let rows = stmt.query_map(pr.as_slice(), |row| {
        Ok(serde_json::json!({
            "id": row.get::<_, i64>(0)?,
            "date": row.get::<_, String>(1)?,
            "article_id": row.get::<_, i64>(2)?,
            "designation": row.get::<_, String>(3)?,
            "quantite": row.get::<_, f64>(4)?,
            "mtype": row.get::<_, String>(5)?,
            "reference_id": row.get::<_, Option<i64>>(6)?,
            "reference_type": row.get::<_, Option<String>>(7)?,
        }))
    }).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_articles_stock_alerte(db: State<DbState>, auth: State<AuthState>, token: String) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let _me = autoriser(&auth, &conn, &token, Acces::Module("reappro", "voir"))?;
    let mut stmt = conn.prepare(
        "SELECT a.id, a.designation, a.stock, a.stock_alerte, c.nom as categorie_nom,
                f.nom as fournisseur_nom, a.fournisseur_id, a.prix_achat,
                CAST((a.stock_alerte * 2 - a.stock) AS INTEGER) as suggestion_qte
         FROM articles a
         LEFT JOIN categories c ON a.categorie_id = c.id
         LEFT JOIN fournisseurs f ON a.fournisseur_id = f.id
         WHERE a.actif=1 AND a.stock <= a.stock_alerte AND a.stock_alerte > 0
         ORDER BY (a.stock_alerte - a.stock) DESC"
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |row| {
        Ok(serde_json::json!({
            "id": row.get::<_, i64>(0)?,
            "designation": row.get::<_, String>(1)?,
            "stock": row.get::<_, f64>(2)?,
            "stock_alerte": row.get::<_, f64>(3)?,
            "categorie_nom": row.get::<_, Option<String>>(4)?,
            "fournisseur_nom": row.get::<_, Option<String>>(5)?,
            "fournisseur_id": row.get::<_, Option<i64>>(6)?,
            "prix_achat": row.get::<_, f64>(7)?,
            "suggestion_qte": row.get::<_, i64>(8)?,
        }))
    }).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}
