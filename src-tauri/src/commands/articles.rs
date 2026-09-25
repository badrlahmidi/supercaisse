use crate::db::*;
use crate::session::{autoriser, Acces, AuthState};
use rusqlite::params;
use tauri::State;

use super::{adjust_article_stock, default_magasin_id, log_audit};

#[tauri::command]
pub fn get_articles(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    recherche: Option<String>,
) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let _me = autoriser(&auth, &conn, &token, Acces::Connecte)?;
    let image_col = "a.image_url";
    let (query, params_vec): (String, Vec<Box<dyn rusqlite::types::ToSql>>) = match recherche {
        Some(ref q) if !q.is_empty() => (
            format!("SELECT a.id, a.code_barre, a.designation, a.prix_achat, a.prix_vente, a.tva, a.stock, a.stock_alerte,
                    a.categorie_id, a.fournisseur_id, a.actif, {image_col},
                    c.nom as categorie_nom, f.nom as fournisseur_nom, a.suivi_lot, a.prix_grossiste, a.est_kit,
                    EXISTS(SELECT 1 FROM article_variantes v WHERE v.article_id = a.id) as a_variantes
             FROM articles a
             LEFT JOIN categories c ON a.categorie_id = c.id
             LEFT JOIN fournisseurs f ON a.fournisseur_id = f.id
             WHERE a.designation LIKE ?1 OR a.code_barre LIKE ?1
             ORDER BY a.designation"),
            vec![Box::new(format!("%{}%", q))]
        ),
        _ => (
            format!("SELECT a.id, a.code_barre, a.designation, a.prix_achat, a.prix_vente, a.tva, a.stock, a.stock_alerte,
                    a.categorie_id, a.fournisseur_id, a.actif, {image_col},
                    c.nom as categorie_nom, f.nom as fournisseur_nom, a.suivi_lot, a.prix_grossiste, a.est_kit,
                    EXISTS(SELECT 1 FROM article_variantes v WHERE v.article_id = a.id) as a_variantes
             FROM articles a
             LEFT JOIN categories c ON a.categorie_id = c.id
             LEFT JOIN fournisseurs f ON a.fournisseur_id = f.id
             ORDER BY a.designation"),
            vec![]
        ),
    };
    let mut stmt = conn.prepare(&query).map_err(|e| e.to_string())?;
    let params_refs: Vec<&dyn rusqlite::types::ToSql> =
        params_vec.iter().map(|p| p.as_ref()).collect();
    let rows = stmt
        .query_map(params_refs.as_slice(), |row| {
            let actif_int: i32 = row.get(10)?;
            let suivi_lot_int: Option<i32> = row.get(14)?;
            let est_kit_int: Option<i32> = row.get(16)?;
            let a_variantes_int: i32 = row.get(17)?;
            Ok(serde_json::json!({
                "id": row.get::<_, i64>(0)?,
                "code_barre": row.get::<_, Option<String>>(1)?,
                "designation": row.get::<_, String>(2)?,
                "prix_achat": row.get::<_, f64>(3)?,
                "prix_vente": row.get::<_, f64>(4)?,
                "tva": row.get::<_, f64>(5)?,
                "stock": row.get::<_, f64>(6)?,
                "stock_alerte": row.get::<_, Option<f64>>(7)?,
                "categorie_id": row.get::<_, Option<i64>>(8)?,
                "fournisseur_id": row.get::<_, Option<i64>>(9)?,
                "actif": actif_int != 0,
                "image_url": row.get::<_, Option<String>>(11)?,
                "categorie_nom": row.get::<_, Option<String>>(12)?,
                "fournisseur_nom": row.get::<_, Option<String>>(13)?,
                "suivi_lot": suivi_lot_int.unwrap_or(0) != 0,
                "prix_grossiste": row.get::<_, Option<f64>>(15)?,
                "est_kit": est_kit_int.unwrap_or(0) != 0,
                "a_variantes": a_variantes_int != 0,
            }))
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_article(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    code_barre: Option<String>,
    designation: String,
    description: Option<String>,
    image_url: Option<String>,
    prix_achat: f64,
    prix_vente: f64,
    tva: f64,
    stock: f64,
    stock_alerte: Option<f64>,
    categorie_id: Option<i64>,
    fournisseur_id: Option<i64>,
    suivi_lot: Option<bool>,
    prix_grossiste: Option<f64>,
    est_kit: Option<bool>,
) -> Result<i64, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let _me = autoriser(&auth, &conn, &token, Acces::Module("articles", "creer"))?;
    let effective_code_barre = match &code_barre {
        Some(cb) if !cb.trim().is_empty() => code_barre.clone(),
        _ => None,
    };
    conn.execute(
        "INSERT INTO articles (code_barre, designation, description, image_url, prix_achat, prix_vente, tva, stock, stock_alerte, categorie_id, fournisseur_id, suivi_lot, prix_grossiste, est_kit)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        params![effective_code_barre, designation, description, image_url, prix_achat, prix_vente, tva, stock, stock_alerte, categorie_id, fournisseur_id, suivi_lot.unwrap_or(false) as i32, prix_grossiste, est_kit.unwrap_or(false) as i32],
    ).map_err(|e| e.to_string())?;
    let article_id = conn.last_insert_rowid();
    if effective_code_barre.is_none() {
        let auto_barcode = format!("INT-{:06}", article_id);
        conn.execute(
            "UPDATE articles SET code_barre = ?1 WHERE id = ?2",
            params![auto_barcode, article_id],
        )
        .map_err(|e| e.to_string())?;
    }
    if stock != 0.0 {
        let magasin_id = default_magasin_id(&conn)?;
        adjust_article_stock(&conn, article_id, magasin_id, stock)?;
    }
    Ok(article_id)
}

#[tauri::command]
pub fn update_article(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    id: i64,
    code_barre: Option<String>,
    designation: String,
    description: Option<String>,
    image_url: Option<String>,
    prix_achat: f64,
    prix_vente: f64,
    tva: f64,
    stock_alerte: Option<f64>,
    categorie_id: Option<i64>,
    fournisseur_id: Option<i64>,
    actif: bool,
    suivi_lot: Option<bool>,
    prix_grossiste: Option<f64>,
    est_kit: Option<bool>,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let me = autoriser(&auth, &conn, &token, Acces::Module("articles", "modifier"))?;
    let old: Option<(f64, f64, String)> = conn
        .query_row(
            "SELECT prix_vente, prix_achat, designation FROM articles WHERE id = ?1",
            params![id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .ok();
    conn.execute(
        "UPDATE articles SET code_barre=?1, designation=?2, description=?3, image_url=?4, prix_achat=?5, prix_vente=?6, tva=?7, stock_alerte=?8, categorie_id=?9, fournisseur_id=?10, actif=?11, suivi_lot=?12, prix_grossiste=?13, est_kit=?14 WHERE id=?15",
        params![code_barre, designation, description, image_url, prix_achat, prix_vente, tva, stock_alerte, categorie_id, fournisseur_id, actif as i32, suivi_lot.unwrap_or(false) as i32, prix_grossiste, est_kit.unwrap_or(false) as i32, id],
    ).map_err(|e| e.to_string())?;
    if let Some((old_pv, old_pa, old_name)) = old {
        let mut changes = Vec::new();
        if (old_pv - prix_vente).abs() > 0.001 {
            changes.push(format!("prix vente: {:.2} → {:.2}", old_pv, prix_vente));
        }
        if (old_pa - prix_achat).abs() > 0.001 {
            changes.push(format!("prix achat: {:.2} → {:.2}", old_pa, prix_achat));
        }
        if old_name != designation {
            changes.push(format!("nom: {} → {}", old_name, designation));
        }
        if !changes.is_empty() {
            log_audit(
                &conn,
                Some(me.user_id),
                "modifier_article",
                &format!("{} (ID {}) — {}", designation, id, changes.join(", ")),
                Some("article"),
                Some(id),
            );
        }
    }
    Ok(())
}

#[tauri::command]
pub fn delete_article(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    id: i64,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let me = autoriser(&auth, &conn, &token, Acces::Module("articles", "modifier"))?;
    let designation: String = conn
        .query_row(
            "SELECT designation FROM articles WHERE id = ?1",
            params![id],
            |r| r.get(0),
        )
        .unwrap_or_else(|_| format!("ID {}", id));
    conn.execute("DELETE FROM articles WHERE id=?1", params![id])
        .map_err(|e| e.to_string())?;
    log_audit(
        &conn,
        Some(me.user_id),
        "supprimer_article",
        &format!("Suppression article: {} (ID {})", designation, id),
        Some("article"),
        Some(id),
    );
    Ok(())
}

#[tauri::command]
pub fn update_article_stock(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    article_id: i64,
    quantite: f64,
) -> Result<(), String> {
    let mut conn = db.conn.lock().map_err(|e| e.to_string())?;
    let _me = autoriser(&auth, &conn, &token, Acces::Module("stock", "modifier"))?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let magasin_id = default_magasin_id(&tx)?;
    adjust_article_stock(&tx, article_id, magasin_id, quantite)?;
    let mtype = if quantite >= 0.0 { "entree" } else { "sortie" };
    tx.execute(
        "INSERT INTO mouvements_stock (article_id, quantite, mtype, reference_type, magasin_id) VALUES (?1, ?2, ?3, 'ajustement', ?4)",
        params![article_id, quantite.abs(), mtype, magasin_id],
    ).map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn import_articles_csv(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    csv_content: String,
) -> Result<String, String> {
    let mut conn = db.conn.lock().map_err(|e| e.to_string())?;
    let me = autoriser(&auth, &conn, &token, Acces::Module("articles", "creer"))?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let magasin_id = default_magasin_id(&tx)?;
    let mut imported = 0u32;
    let mut errors: Vec<String> = Vec::new();

    for (i, line) in csv_content.lines().enumerate() {
        if i == 0 {
            continue;
        }
        if line.trim().is_empty() {
            continue;
        }
        let cols: Vec<&str> = line.split(';').collect();
        if cols.len() < 5 {
            errors.push(format!(
                "Ligne {}: format invalide (minimum 5 colonnes attendues)",
                i + 1
            ));
            continue;
        }
        let designation = cols[0].trim().trim_matches('"');
        let code_barre = if cols[1].trim().is_empty() || cols[1].trim() == "\"\"" {
            None
        } else {
            Some(cols[1].trim().trim_matches('"').to_string())
        };
        let prix_achat: f64 = cols[2]
            .trim()
            .trim_matches('"')
            .replace(',', ".")
            .parse()
            .unwrap_or(0.0);
        let prix_vente: f64 = cols[3]
            .trim()
            .trim_matches('"')
            .replace(',', ".")
            .parse()
            .unwrap_or(0.0);
        let tva: f64 = if cols.len() > 4 {
            cols[4]
                .trim()
                .trim_matches('"')
                .replace(',', ".")
                .parse()
                .unwrap_or(0.0)
        } else {
            0.0
        };
        let stock: f64 = if cols.len() > 5 {
            cols[5]
                .trim()
                .trim_matches('"')
                .replace(',', ".")
                .parse()
                .unwrap_or(0.0)
        } else {
            0.0
        };
        let stock_alerte: Option<f64> =
            if cols.len() > 6 && !cols[6].trim().is_empty() && cols[6].trim() != "\"\"" {
                Some(
                    cols[6]
                        .trim()
                        .trim_matches('"')
                        .replace(',', ".")
                        .parse()
                        .unwrap_or(0.0),
                )
            } else {
                None
            };
        let image_url: Option<String> =
            if cols.len() > 7 && !cols[7].trim().is_empty() && cols[7].trim() != "\"\"" {
                Some(cols[7].trim().trim_matches('"').to_string())
            } else {
                None
            };

        match tx.execute(
            "INSERT INTO articles (code_barre, designation, prix_achat, prix_vente, tva, stock, stock_alerte, image_url)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![code_barre, designation, prix_achat, prix_vente, tva, stock, stock_alerte, image_url],
        ) {
            Ok(_) => {
                imported += 1;
                if stock != 0.0 {
                    let article_id = tx.last_insert_rowid();
                    if let Err(e) = adjust_article_stock(&tx, article_id, magasin_id, stock) {
                        errors.push(format!("Ligne {} (stock): {}", i+1, e));
                    }
                }
            }
            Err(e) => errors.push(format!("Ligne {}: {}", i+1, e)),
        }
    }

    log_audit(
        &tx,
        Some(me.user_id),
        "importer_csv",
        &format!(
            "Import CSV: {} articles importés, {} erreurs",
            imported,
            errors.len()
        ),
        None,
        None,
    );

    tx.commit().map_err(|e| e.to_string())?;

    let mut report = format!("Import terminé. {} articles importés.", imported);
    if !errors.is_empty() {
        report.push_str(&format!(
            "\n{} erreur(s):\n{}",
            errors.len(),
            errors.join("\n")
        ));
    }
    Ok(report)
}
