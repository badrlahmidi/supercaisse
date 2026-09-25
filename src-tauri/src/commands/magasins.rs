use crate::db::*;
use crate::session::{autoriser, Acces, AuthState};
use rusqlite::params;
use tauri::State;

use super::{adjust_article_stock, log_audit};

#[tauri::command]
pub fn get_magasins(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let _me = autoriser(&auth, &conn, &token, Acces::Connecte)?;
    let mut stmt = conn
        .prepare("SELECT id, nom, adresse FROM magasins ORDER BY id")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, i64>(0)?,
                "nom": row.get::<_, String>(1)?,
                "adresse": row.get::<_, Option<String>>(2)?
            }))
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_magasin(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    nom: String,
    adresse: Option<String>,
) -> Result<i64, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let _me = autoriser(&auth, &conn, &token, Acces::Module("magasins", "creer"))?;
    conn.execute(
        "INSERT INTO magasins (nom, adresse) VALUES (?1, ?2)",
        params![nom, adresse],
    )
    .map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

#[tauri::command]
pub fn update_magasin(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    id: i64,
    nom: String,
    adresse: Option<String>,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let _me = autoriser(&auth, &conn, &token, Acces::Module("magasins", "modifier"))?;
    conn.execute(
        "UPDATE magasins SET nom=?1, adresse=?2 WHERE id=?3",
        params![nom, adresse, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn delete_magasin(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    id: i64,
) -> Result<(), String> {
    let mut conn = db.conn.lock().map_err(|e| e.to_string())?;
    let _me = autoriser(&auth, &conn, &token, Acces::Module("magasins", "modifier"))?;
    let count: i64 = conn
        .query_row("SELECT count(*) FROM magasins", [], |r| r.get(0))
        .unwrap_or(0);
    if count <= 1 {
        return Err("Impossible de supprimer le dernier magasin".to_string());
    }
    let has_sessions: bool = conn
        .query_row(
            "SELECT count(*) > 0 FROM sessions_caisse WHERE magasin_id = ?1",
            params![id],
            |r| r.get(0),
        )
        .unwrap_or(false);
    if has_sessions {
        return Err("Ce magasin a des sessions de caisse associées".to_string());
    }
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    tx.execute(
        "DELETE FROM article_stocks WHERE magasin_id = ?1",
        params![id],
    )
    .map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM magasins WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    tx.execute(
        "UPDATE articles SET stock = COALESCE((SELECT SUM(quantite) FROM article_stocks WHERE article_id = articles.id), 0)",
        [],
    ).map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_transferts(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let _me = autoriser(&auth, &conn, &token, Acces::Module("stock", "voir"))?;
    let mut stmt = conn.prepare(
        "SELECT t.id, t.date, t.statut, ms.nom AS source_nom, md.nom AS dest_nom, u.nom AS utilisateur_nom
         FROM transferts_stock t
         JOIN magasins ms ON ms.id = t.source_id
         JOIN magasins md ON md.id = t.dest_id
         LEFT JOIN utilisateurs u ON u.id = t.utilisateur_id
         ORDER BY t.date DESC"
    ).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, i64>(0)?,
                "date": row.get::<_, String>(1)?,
                "statut": row.get::<_, String>(2)?,
                "source_nom": row.get::<_, String>(3)?,
                "dest_nom": row.get::<_, String>(4)?,
                "utilisateur_nom": row.get::<_, Option<String>>(5)?
            }))
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_stock_par_magasin(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    magasin_id: i64,
) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let _me = autoriser(&auth, &conn, &token, Acces::Module("stock", "voir"))?;
    let mut stmt = conn.prepare(
        "SELECT a.id, a.designation, a.code_barre, COALESCE(s.quantite, 0) AS stock, a.stock_alerte
         FROM articles a
         LEFT JOIN article_stocks s ON s.article_id = a.id AND s.magasin_id = ?1
         WHERE a.actif = 1
         ORDER BY a.designation"
    ).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![magasin_id], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, i64>(0)?,
                "designation": row.get::<_, String>(1)?,
                "code_barre": row.get::<_, Option<String>>(2)?,
                "stock": row.get::<_, f64>(3)?,
                "stock_alerte": row.get::<_, Option<f64>>(4)?
            }))
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_transfert(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    source_id: i64,
    dest_id: i64,
    articles: Vec<serde_json::Value>,
) -> Result<i64, String> {
    let mut conn = db.conn.lock().map_err(|e| e.to_string())?;
    let me = autoriser(&auth, &conn, &token, Acces::Module("stock", "creer"))?;
    let utilisateur_id = Some(me.user_id);
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    tx.execute(
        "INSERT INTO transferts_stock (source_id, dest_id, utilisateur_id, statut) VALUES (?1, ?2, ?3, 'en_attente')",
        params![source_id, dest_id, utilisateur_id]
    ).map_err(|e| e.to_string())?;

    let transfert_id = tx.last_insert_rowid();

    for a in articles {
        let article_id = a["article_id"]
            .as_i64()
            .ok_or("article_id manquant ou invalide dans la ligne")?;
        let quantite = a["quantite"].as_f64().unwrap_or(0.0);
        tx.execute(
            "INSERT INTO transfert_lignes (transfert_id, article_id, quantite) VALUES (?1, ?2, ?3)",
            params![transfert_id, article_id, quantite],
        )
        .map_err(|e| e.to_string())?;
    }

    tx.commit().map_err(|e| e.to_string())?;
    Ok(transfert_id)
}

#[tauri::command]
pub fn validate_transfert(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    transfert_id: i64,
) -> Result<(), String> {
    let mut conn = db.conn.lock().map_err(|e| e.to_string())?;
    let me = autoriser(&auth, &conn, &token, Acces::Module("stock", "modifier"))?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    let (source_id, dest_id, statut): (i64, i64, String) = tx
        .query_row(
            "SELECT source_id, dest_id, statut FROM transferts_stock WHERE id = ?1",
            params![transfert_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(|_| "Transfert introuvable".to_string())?;

    if statut != "en_attente" {
        return Err("Ce transfert a déjà été traité".to_string());
    }

    let mut stmt = tx
        .prepare("SELECT article_id, quantite FROM transfert_lignes WHERE transfert_id = ?1")
        .map_err(|e| e.to_string())?;
    let lignes = stmt
        .query_map(params![transfert_id], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, f64>(1)?))
        })
        .map_err(|e| e.to_string())?;

    for res in lignes {
        let (article_id, qte) = res.map_err(|e| e.to_string())?;

        adjust_article_stock(&tx, article_id, source_id, -qte)?;
        tx.execute(
            "INSERT INTO mouvements_stock (article_id, quantite, mtype, reference_id, reference_type, magasin_id) VALUES (?1, ?2, 'sortie', ?3, 'transfert', ?4)",
            params![article_id, qte, transfert_id, source_id]
        ).map_err(|e| e.to_string())?;

        adjust_article_stock(&tx, article_id, dest_id, qte)?;
        tx.execute(
            "INSERT INTO mouvements_stock (article_id, quantite, mtype, reference_id, reference_type, magasin_id) VALUES (?1, ?2, 'entree', ?3, 'transfert', ?4)",
            params![article_id, qte, transfert_id, dest_id]
        ).map_err(|e| e.to_string())?;
    }

    tx.execute(
        "UPDATE transferts_stock SET statut = 'valide' WHERE id = ?1",
        params![transfert_id],
    )
    .map_err(|e| e.to_string())?;

    log_audit(
        &tx,
        Some(me.user_id),
        "valider_transfert",
        &format!(
            "Validation transfert #{} (magasin {} → {})",
            transfert_id, source_id, dest_id
        ),
        Some("transfert"),
        Some(transfert_id),
    );

    drop(stmt);
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}
