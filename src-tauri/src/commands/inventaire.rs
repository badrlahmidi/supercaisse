use crate::db::*;
use rusqlite::params;
use tauri::State;
use crate::session::{autoriser, Acces, AuthState};

use super::log_audit;

#[tauri::command]
pub fn create_inventaire(db: State<DbState>, auth: State<AuthState>, token: String, magasin_id: i64) -> Result<serde_json::Value, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let me = autoriser(&auth, &conn, &token, Acces::Module("inventaire", "creer"))?;
    let utilisateur_id = me.user_id;
    conn.execute(
        "INSERT INTO inventaires (magasin_id, utilisateur_id) VALUES (?1, ?2)",
        params![magasin_id, utilisateur_id],
    ).map_err(|e| e.to_string())?;
    let inv_id = conn.last_insert_rowid();

    let mut stmt = conn.prepare(
        "SELECT a.id, COALESCE(s.quantite, 0)
         FROM articles a
         LEFT JOIN article_stocks s ON s.article_id = a.id AND s.magasin_id = ?1
         WHERE a.actif = 1
         ORDER BY a.designation"
    ).map_err(|e| e.to_string())?;
    let rows: Vec<(i64, f64)> = stmt.query_map(params![magasin_id], |r| {
        Ok((r.get(0)?, r.get(1)?))
    }).map_err(|e| e.to_string())?.filter_map(Result::ok).collect();

    for (article_id, stock_theo) in &rows {
        conn.execute(
            "INSERT INTO inventaire_lignes (inventaire_id, article_id, stock_theorique) VALUES (?1, ?2, ?3)",
            params![inv_id, article_id, stock_theo],
        ).map_err(|e| e.to_string())?;
    }

    log_audit(&conn, Some(utilisateur_id), "creer_inventaire", &format!("Inventaire #{} créé ({} articles)", inv_id, rows.len()), Some("inventaire"), Some(inv_id));

    Ok(serde_json::json!({ "id": inv_id, "nb_articles": rows.len() }))
}

#[tauri::command]
pub fn get_inventaire(db: State<DbState>, auth: State<AuthState>, token: String, inventaire_id: i64) -> Result<serde_json::Value, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let _me = autoriser(&auth, &conn, &token, Acces::Module("inventaire", "voir"))?;

    let (date_debut, statut, magasin_id): (String, String, i64) = conn.query_row(
        "SELECT date_debut, statut, magasin_id FROM inventaires WHERE id = ?1",
        params![inventaire_id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    ).map_err(|_| "Inventaire introuvable".to_string())?;

    let mut stmt = conn.prepare(
        "SELECT il.id, il.article_id, a.designation, a.code_barre, il.stock_theorique, il.stock_compte, il.ecart
         FROM inventaire_lignes il
         JOIN articles a ON a.id = il.article_id
         WHERE il.inventaire_id = ?1
         ORDER BY a.designation"
    ).map_err(|e| e.to_string())?;
    let lignes = stmt.query_map(params![inventaire_id], |r| {
        Ok(serde_json::json!({
            "id": r.get::<_, i64>(0)?,
            "article_id": r.get::<_, i64>(1)?,
            "designation": r.get::<_, String>(2)?,
            "code_barre": r.get::<_, Option<String>>(3)?,
            "stock_theorique": r.get::<_, f64>(4)?,
            "stock_compte": r.get::<_, Option<f64>>(5)?,
            "ecart": r.get::<_, Option<f64>>(6)?
        }))
    }).map_err(|e| e.to_string())?.filter_map(Result::ok).collect::<Vec<_>>();

    Ok(serde_json::json!({
        "id": inventaire_id,
        "date_debut": date_debut,
        "statut": statut,
        "magasin_id": magasin_id,
        "lignes": lignes,
    }))
}

#[tauri::command]
pub fn get_inventaires(db: State<DbState>, auth: State<AuthState>, token: String) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let _me = autoriser(&auth, &conn, &token, Acces::Module("inventaire", "voir"))?;
    let mut stmt = conn.prepare(
        "SELECT i.id, i.date_debut, i.date_fin, i.statut, i.magasin_id, m.nom, u.nom,
                (SELECT COUNT(*) FROM inventaire_lignes WHERE inventaire_id = i.id),
                (SELECT COUNT(*) FROM inventaire_lignes WHERE inventaire_id = i.id AND stock_compte IS NOT NULL)
         FROM inventaires i
         JOIN magasins m ON m.id = i.magasin_id
         LEFT JOIN utilisateurs u ON u.id = i.utilisateur_id
         ORDER BY i.date_debut DESC LIMIT 50"
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |r| {
        Ok(serde_json::json!({
            "id": r.get::<_, i64>(0)?,
            "date_debut": r.get::<_, String>(1)?,
            "date_fin": r.get::<_, Option<String>>(2)?,
            "statut": r.get::<_, String>(3)?,
            "magasin_id": r.get::<_, i64>(4)?,
            "magasin_nom": r.get::<_, String>(5)?,
            "utilisateur_nom": r.get::<_, Option<String>>(6)?,
            "nb_articles": r.get::<_, i64>(7)?,
            "nb_comptes": r.get::<_, i64>(8)?
        }))
    }).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_inventaire_ligne(db: State<DbState>, auth: State<AuthState>, token: String, ligne_id: i64, stock_compte: f64) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let _me = autoriser(&auth, &conn, &token, Acces::Module("inventaire", "modifier"))?;
    let stock_theorique: f64 = conn.query_row(
        "SELECT stock_theorique FROM inventaire_lignes WHERE id = ?1",
        params![ligne_id], |r| r.get(0),
    ).map_err(|_| "Ligne introuvable".to_string())?;
    let ecart = stock_compte - stock_theorique;
    conn.execute(
        "UPDATE inventaire_lignes SET stock_compte = ?1, ecart = ?2 WHERE id = ?3",
        params![stock_compte, ecart, ligne_id],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn valider_inventaire(db: State<DbState>, auth: State<AuthState>, token: String, inventaire_id: i64) -> Result<(), String> {
    let mut conn = db.conn.lock().map_err(|e| e.to_string())?;
    let me = autoriser(&auth, &conn, &token, Acces::Module("inventaire", "modifier"))?;
    let utilisateur_id = me.user_id;

    let statut: String = conn.query_row(
        "SELECT statut FROM inventaires WHERE id = ?1",
        params![inventaire_id], |r| r.get(0),
    ).map_err(|_| "Inventaire introuvable".to_string())?;
    if statut != "en_cours" {
        return Err("Cet inventaire est déjà validé".to_string());
    }

    let magasin_id: i64 = conn.query_row(
        "SELECT magasin_id FROM inventaires WHERE id = ?1",
        params![inventaire_id], |r| r.get(0),
    ).map_err(|e| e.to_string())?;

    let mut stmt = conn.prepare(
        "SELECT article_id, stock_compte, ecart FROM inventaire_lignes WHERE inventaire_id = ?1 AND stock_compte IS NOT NULL AND ecart != 0"
    ).map_err(|e| e.to_string())?;
    let adjustments: Vec<(i64, f64, f64)> = stmt.query_map(params![inventaire_id], |r| {
        Ok((r.get(0)?, r.get(1)?, r.get(2)?))
    }).map_err(|e| e.to_string())?.filter_map(Result::ok).collect();
    drop(stmt);

    let tx = conn.transaction().map_err(|e| e.to_string())?;

    for (article_id, stock_compte, ecart) in &adjustments {
        tx.execute(
            "INSERT INTO article_stocks (article_id, magasin_id, quantite) VALUES (?1, ?2, ?3)
             ON CONFLICT(article_id, magasin_id) DO UPDATE SET quantite = ?3",
            params![article_id, magasin_id, stock_compte],
        ).map_err(|e| e.to_string())?;

        let total_stock: f64 = tx.query_row(
            "SELECT COALESCE(SUM(quantite), 0) FROM article_stocks WHERE article_id = ?1",
            params![article_id], |r| r.get(0),
        ).unwrap_or(0.0);
        tx.execute("UPDATE articles SET stock = ?1 WHERE id = ?2", params![total_stock, article_id]).map_err(|e| e.to_string())?;

        tx.execute(
            "INSERT INTO mouvements_stock (article_id, quantite, mtype, reference_id, reference_type, magasin_id)
             VALUES (?1, ?2, 'inventaire', ?3, 'inventaire', ?4)",
            params![article_id, ecart, inventaire_id, magasin_id],
        ).map_err(|e| e.to_string())?;
    }

    tx.execute(
        "UPDATE inventaires SET statut = 'valide', date_fin = datetime('now','localtime') WHERE id = ?1",
        params![inventaire_id],
    ).map_err(|e| e.to_string())?;

    log_audit(&tx, Some(utilisateur_id), "valider_inventaire",
        &format!("Inventaire #{} validé ({} écarts appliqués)", inventaire_id, adjustments.len()),
        Some("inventaire"), Some(inventaire_id));

    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}
