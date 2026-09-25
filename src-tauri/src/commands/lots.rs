use crate::db::*;
use crate::session::{autoriser, Acces, AuthState};
use rusqlite::params;
use tauri::State;

use super::mouvements::retirer_stock;
use super::{adjust_article_stock, default_magasin_id};

#[tauri::command]
pub fn add_article_lot(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    article_id: i64,
    numero_lot: Option<String>,
    date_peremption: Option<String>,
    quantite: f64,
) -> Result<i64, String> {
    let mut conn = db.conn.lock().map_err(|e| e.to_string())?;
    let _me = autoriser(&auth, &conn, &token, Acces::Module("stock", "modifier"))?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let magasin_id = default_magasin_id(&tx)?;
    tx.execute(
        "INSERT INTO article_lots (article_id, magasin_id, numero_lot, date_peremption, quantite) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![article_id, magasin_id, numero_lot, date_peremption, quantite],
    ).map_err(|e| e.to_string())?;
    let lot_id = tx.last_insert_rowid();
    if quantite != 0.0 {
        adjust_article_stock(&tx, article_id, magasin_id, quantite)?;
        tx.execute(
            "INSERT INTO mouvements_stock (article_id, quantite, mtype, reference_id, reference_type, magasin_id) VALUES (?1, ?2, 'entree', ?3, 'lot', ?4)",
            params![article_id, quantite, lot_id, magasin_id],
        ).map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(lot_id)
}

#[tauri::command]
pub fn get_article_lots(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    article_id: i64,
) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let _me = autoriser(&auth, &conn, &token, Acces::Module("stock", "voir"))?;
    let mut stmt = conn
        .prepare(
            "SELECT id, numero_lot, date_peremption, quantite, date_reception
         FROM article_lots WHERE article_id = ?1
         ORDER BY (date_peremption IS NULL), date_peremption ASC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![article_id], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, i64>(0)?,
                "numero_lot": row.get::<_, Option<String>>(1)?,
                "date_peremption": row.get::<_, Option<String>>(2)?,
                "quantite": row.get::<_, f64>(3)?,
                "date_reception": row.get::<_, String>(4)?,
            }))
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_lots_peremption_proche(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    jours: i64,
) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let _me = autoriser(&auth, &conn, &token, Acces::Module("stock", "voir"))?;
    let mut stmt = conn
        .prepare(
            "SELECT l.id, l.article_id, a.designation, l.numero_lot, l.date_peremption, l.quantite
         FROM article_lots l
         JOIN articles a ON a.id = l.article_id
         WHERE l.quantite > 0 AND l.date_peremption IS NOT NULL
           AND date(l.date_peremption) <= date('now', ?1 || ' days')
         ORDER BY l.date_peremption ASC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![jours.to_string()], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, i64>(0)?,
                "article_id": row.get::<_, i64>(1)?,
                "designation": row.get::<_, String>(2)?,
                "numero_lot": row.get::<_, Option<String>>(3)?,
                "date_peremption": row.get::<_, String>(4)?,
                "quantite": row.get::<_, f64>(5)?,
            }))
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn discard_article_lot(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    lot_id: i64,
    quantite: f64,
    motif: Option<String>,
) -> Result<(), String> {
    let mut conn = db.conn.lock().map_err(|e| e.to_string())?;
    let _me = autoriser(&auth, &conn, &token, Acces::Module("stock", "modifier"))?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    let (article_id, magasin_id, lot_quantite): (i64, i64, f64) = tx
        .query_row(
            "SELECT article_id, magasin_id, quantite FROM article_lots WHERE id = ?1",
            params![lot_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(|_| "Lot introuvable".to_string())?;

    if quantite <= 0.0 || quantite > lot_quantite {
        return Err(format!(
            "Quantité invalide (disponible dans ce lot : {})",
            lot_quantite
        ));
    }

    tx.execute(
        "UPDATE article_lots SET quantite = quantite - ?1 WHERE id = ?2",
        params![quantite, lot_id],
    )
    .map_err(|e| e.to_string())?;
    retirer_stock(&tx, article_id, magasin_id, quantite)?;
    tx.execute(
        "INSERT INTO mouvements_stock (article_id, quantite, mtype, reference_id, reference_type, magasin_id) VALUES (?1, ?2, 'sortie', ?3, ?4, ?5)",
        params![article_id, quantite, lot_id, motif.unwrap_or_else(|| "peremption".to_string()), magasin_id],
    ).map_err(|e| e.to_string())?;

    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}
