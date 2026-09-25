use crate::db::*;
use rusqlite::{params, OptionalExtension};
use tauri::State;

use super::default_magasin_id;

#[tauri::command]
pub fn add_article_variante(db: State<DbState>, article_id: i64, taille: Option<String>,
    couleur: Option<String>, code_barre: Option<String>, stock_initial: f64) -> Result<i64, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO article_variantes (article_id, taille, couleur, code_barre, stock_dedie) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![article_id, taille, couleur, code_barre, stock_initial],
    ).map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

#[tauri::command]
pub fn get_article_variantes(db: State<DbState>, article_id: i64) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT id, taille, couleur, code_barre, stock_dedie FROM article_variantes WHERE article_id = ?1 ORDER BY taille, couleur"
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map(params![article_id], |row| {
        Ok(serde_json::json!({
            "id": row.get::<_, i64>(0)?,
            "taille": row.get::<_, Option<String>>(1)?,
            "couleur": row.get::<_, Option<String>>(2)?,
            "code_barre": row.get::<_, Option<String>>(3)?,
            "stock_dedie": row.get::<_, f64>(4)?,
        }))
    }).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_article_variante(db: State<DbState>, id: i64, taille: Option<String>,
    couleur: Option<String>, code_barre: Option<String>) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE article_variantes SET taille=?1, couleur=?2, code_barre=?3 WHERE id=?4",
        params![taille, couleur, code_barre, id],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn adjust_article_variante_stock(db: State<DbState>, id: i64, quantite: f64) -> Result<(), String> {
    let mut conn = db.conn.lock().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let article_id: i64 = tx.query_row("SELECT article_id FROM article_variantes WHERE id = ?1", params![id], |r| r.get(0))
        .map_err(|_| "Variante introuvable".to_string())?;
    tx.execute("UPDATE article_variantes SET stock_dedie = stock_dedie + ?1 WHERE id = ?2", params![quantite, id])
        .map_err(|e| e.to_string())?;
    let magasin_id = default_magasin_id(&tx)?;
    let mtype = if quantite >= 0.0 { "entree" } else { "sortie" };
    tx.execute(
        "INSERT INTO mouvements_stock (article_id, quantite, mtype, reference_id, reference_type, magasin_id) VALUES (?1, ?2, ?3, ?4, 'variante_ajustement', ?5)",
        params![article_id, quantite.abs(), mtype, id, magasin_id],
    ).map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn delete_article_variante(db: State<DbState>, id: i64) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM article_variantes WHERE id=?1", params![id]).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn find_variante_by_barcode(db: State<DbState>, code_barre: String) -> Result<Option<serde_json::Value>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.query_row(
        "SELECT v.id, v.article_id, v.taille, v.couleur, v.stock_dedie,
                a.designation, a.prix_vente, a.tva, a.actif
         FROM article_variantes v
         JOIN articles a ON a.id = v.article_id
         WHERE v.code_barre = ?1",
        params![code_barre],
        |row| Ok(serde_json::json!({
            "variante_id": row.get::<_, i64>(0)?,
            "article_id": row.get::<_, i64>(1)?,
            "taille": row.get::<_, Option<String>>(2)?,
            "couleur": row.get::<_, Option<String>>(3)?,
            "stock_dedie": row.get::<_, f64>(4)?,
            "designation": row.get::<_, String>(5)?,
            "prix_vente": row.get::<_, f64>(6)?,
            "tva": row.get::<_, f64>(7)?,
            "actif": row.get::<_, i32>(8)? != 0,
        })),
    ).optional().map_err(|e| e.to_string())
}
