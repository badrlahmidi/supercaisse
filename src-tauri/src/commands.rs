use crate::db::*;
use base64::engine::general_purpose;
use base64::Engine;
use rusqlite::{backup::Backup, params, Connection, OptionalExtension};
use std::time::Duration;
use tauri::State;

// ─── Stock multi-magasin ───
// `article_stocks` (par magasin) est la source de vérité ; `articles.stock` est
// maintenu comme agrégat (somme sur tous les magasins) pour rester compatible avec
// tout le code qui lit encore ce champ (grille POS, alertes, exports). Tant qu'il
// n'existe qu'un seul magasin, `articles.stock` == la valeur de ce magasin, donc ce
// changement est invisible en usage mono-boutique et corrige la dérive constatée en
// multi-boutique (cf. AUDIT_ARCHITECTURE_SENIOR_2026-09.md §2).

fn default_magasin_id(conn: &Connection) -> Result<i64, String> {
    conn.query_row("SELECT id FROM magasins ORDER BY id LIMIT 1", [], |r| r.get(0))
        .map_err(|e| format!("Aucun magasin configuré: {}", e))
}

fn adjust_article_stock(conn: &Connection, article_id: i64, magasin_id: i64, delta: f64) -> Result<(), String> {
    conn.execute(
        "INSERT INTO article_stocks (article_id, magasin_id, quantite) VALUES (?1, ?2, ?3)
         ON CONFLICT(article_id, magasin_id) DO UPDATE SET quantite = quantite + ?3",
        params![article_id, magasin_id, delta],
    ).map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE articles SET stock = (SELECT COALESCE(SUM(quantite), 0) FROM article_stocks WHERE article_id = ?1) WHERE id = ?1",
        params![article_id],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

fn log_audit(conn: &Connection, utilisateur_id: Option<i64>, action: &str, detail: &str, reference_type: Option<&str>, reference_id: Option<i64>) {
    let _ = conn.execute(
        "INSERT INTO audit_log (utilisateur_id, action, detail, reference_type, reference_id) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![utilisateur_id, action, detail, reference_type, reference_id],
    );
}

// ─── Auth ───

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

// ─── Categories ───

#[tauri::command]
pub fn get_categories(db: State<DbState>) -> Result<Vec<Category>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare("SELECT id, nom, description FROM categories ORDER BY nom").map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |row| {
        Ok(Category {
            id: Some(row.get(0)?),
            nom: row.get(1)?,
            description: row.get(2)?,
        })
    }).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_category(db: State<DbState>, nom: String, description: Option<String>) -> Result<i64, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute("INSERT INTO categories (nom, description) VALUES (?1, ?2)", params![nom, description])
        .map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

#[tauri::command]
pub fn delete_category(db: State<DbState>, id: i64) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM categories WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ─── Fournisseurs ───

#[tauri::command]
pub fn get_fournisseurs(db: State<DbState>) -> Result<Vec<Fournisseur>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare("SELECT id, nom, adresse, telephone, ice, email FROM fournisseurs ORDER BY nom")
        .map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |row| {
        Ok(Fournisseur {
            id: Some(row.get(0)?),
            nom: row.get(1)?,
            adresse: row.get(2)?,
            telephone: row.get(3)?,
            ice: row.get(4)?,
            email: row.get(5)?,
        })
    }).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_fournisseur(db: State<DbState>, nom: String, adresse: Option<String>, telephone: Option<String>, ice: Option<String>, email: Option<String>) -> Result<i64, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO fournisseurs (nom, adresse, telephone, ice, email) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![nom, adresse, telephone, ice, email],
    ).map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

// ─── Clients ───

#[tauri::command]
pub fn get_clients(db: State<DbState>) -> Result<Vec<Client>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare("SELECT id, code, nom, adresse, telephone, email, credit_plafond, credit_actuel, ice, segment FROM clients ORDER BY nom")
        .map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |row| {
        Ok(Client {
            id: Some(row.get(0)?),
            code: row.get(1)?,
            nom: row.get(2)?,
            adresse: row.get(3)?,
            telephone: row.get(4)?,
            email: row.get(5)?,
            credit_plafond: row.get(6)?,
            credit_actuel: row.get(7)?,
            ice: row.get(8)?,
            segment: row.get(9)?,
        })
    }).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_client(db: State<DbState>, code: Option<String>, nom: String, adresse: Option<String>, telephone: Option<String>, email: Option<String>, credit_plafond: Option<f64>, ice: Option<String>, segment: Option<String>) -> Result<i64, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO clients (code, nom, adresse, telephone, email, credit_plafond, ice, segment) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![code, nom, adresse, telephone, email, credit_plafond, ice, segment],
    ).map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

// ─── Fournisseurs (suite) ───

#[tauri::command]
pub fn update_fournisseur(db: State<DbState>, id: i64, nom: String, adresse: Option<String>, telephone: Option<String>, ice: Option<String>, email: Option<String>) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE fournisseurs SET nom=?1, adresse=?2, telephone=?3, ice=?4, email=?5 WHERE id=?6",
        params![nom, adresse, telephone, ice, email, id],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn delete_fournisseur(db: State<DbState>, id: i64) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM fournisseurs WHERE id=?1", params![id]).map_err(|e| e.to_string())?;
    Ok(())
}

// ─── Clients (suite) ───

#[tauri::command]
pub fn update_client(db: State<DbState>, id: i64, code: Option<String>, nom: String, adresse: Option<String>, telephone: Option<String>, email: Option<String>, credit_plafond: Option<f64>, ice: Option<String>, segment: Option<String>) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE clients SET code=?1, nom=?2, adresse=?3, telephone=?4, email=?5, credit_plafond=?6, ice=?7, segment=?8 WHERE id=?9",
        params![code, nom, adresse, telephone, email, credit_plafond, ice, segment, id],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn delete_client(db: State<DbState>, id: i64) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM clients WHERE id=?1", params![id]).map_err(|e| e.to_string())?;
    Ok(())
}

// ─── Articles ───

#[tauri::command]
pub fn get_articles(db: State<DbState>, recherche: Option<String>) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
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
    let params_refs: Vec<&dyn rusqlite::types::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
    let rows = stmt.query_map(params_refs.as_slice(), |row| {
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
    }).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_article(db: State<DbState>, code_barre: Option<String>, designation: String, description: Option<String>,
    image_url: Option<String>, prix_achat: f64, prix_vente: f64, tva: f64, stock: f64, stock_alerte: Option<f64>,
    categorie_id: Option<i64>, fournisseur_id: Option<i64>, suivi_lot: Option<bool>,
    prix_grossiste: Option<f64>, est_kit: Option<bool>) -> Result<i64, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
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
        ).map_err(|e| e.to_string())?;
    }
    if stock != 0.0 {
        let magasin_id = default_magasin_id(&conn)?;
        adjust_article_stock(&conn, article_id, magasin_id, stock)?;
    }
    Ok(article_id)
}

#[tauri::command]
pub fn update_article(db: State<DbState>, id: i64, code_barre: Option<String>, designation: String, description: Option<String>,
    image_url: Option<String>, prix_achat: f64, prix_vente: f64, tva: f64, stock_alerte: Option<f64>,
    categorie_id: Option<i64>, fournisseur_id: Option<i64>, actif: bool, suivi_lot: Option<bool>,
    prix_grossiste: Option<f64>, est_kit: Option<bool>) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let old: Option<(f64, f64, String)> = conn.query_row(
        "SELECT prix_vente, prix_achat, designation FROM articles WHERE id = ?1", params![id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))
    ).ok();
    conn.execute(
        "UPDATE articles SET code_barre=?1, designation=?2, description=?3, image_url=?4, prix_achat=?5, prix_vente=?6, tva=?7, stock_alerte=?8, categorie_id=?9, fournisseur_id=?10, actif=?11, suivi_lot=?12, prix_grossiste=?13, est_kit=?14 WHERE id=?15",
        params![code_barre, designation, description, image_url, prix_achat, prix_vente, tva, stock_alerte, categorie_id, fournisseur_id, actif as i32, suivi_lot.unwrap_or(false) as i32, prix_grossiste, est_kit.unwrap_or(false) as i32, id],
    ).map_err(|e| e.to_string())?;
    if let Some((old_pv, old_pa, old_name)) = old {
        let mut changes = Vec::new();
        if (old_pv - prix_vente).abs() > 0.001 { changes.push(format!("prix vente: {:.2} → {:.2}", old_pv, prix_vente)); }
        if (old_pa - prix_achat).abs() > 0.001 { changes.push(format!("prix achat: {:.2} → {:.2}", old_pa, prix_achat)); }
        if old_name != designation { changes.push(format!("nom: {} → {}", old_name, designation)); }
        if !changes.is_empty() {
            log_audit(&conn, None, "modifier_article",
                &format!("{} (ID {}) — {}", designation, id, changes.join(", ")),
                Some("article"), Some(id));
        }
    }
    Ok(())
}

#[tauri::command]
pub fn delete_article(db: State<DbState>, id: i64) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let designation: String = conn.query_row(
        "SELECT designation FROM articles WHERE id = ?1", params![id], |r| r.get(0)
    ).unwrap_or_else(|_| format!("ID {}", id));
    conn.execute("DELETE FROM articles WHERE id=?1", params![id]).map_err(|e| e.to_string())?;
    log_audit(&conn, None, "supprimer_article",
        &format!("Suppression article: {} (ID {})", designation, id),
        Some("article"), Some(id));
    Ok(())
}

#[tauri::command]
pub fn update_article_stock(db: State<DbState>, article_id: i64, quantite: f64) -> Result<(), String> {
    let mut conn = db.conn.lock().map_err(|e| e.to_string())?;
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

// ─── Lots / péremption (DLC-DLUO) ───
// Couche informative au-dessus du stock agrégé : chaque réception avec suivi de lot
// crée une ligne `article_lots` ET incrémente `article_stocks`/`articles.stock` via
// adjust_article_stock, donc le total reste toujours cohérent avec ou sans lots.
// La vente ne consomme pas encore un lot précis (pas de FEFO automatique) : c'est une
// limite connue, trackée dans ROADMAP_STATUS.md.

#[tauri::command]
pub fn add_article_lot(db: State<DbState>, article_id: i64, numero_lot: Option<String>,
    date_peremption: Option<String>, quantite: f64) -> Result<i64, String> {
    let mut conn = db.conn.lock().map_err(|e| e.to_string())?;
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
pub fn get_article_lots(db: State<DbState>, article_id: i64) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT id, numero_lot, date_peremption, quantite, date_reception
         FROM article_lots WHERE article_id = ?1
         ORDER BY (date_peremption IS NULL), date_peremption ASC"
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map(params![article_id], |row| {
        Ok(serde_json::json!({
            "id": row.get::<_, i64>(0)?,
            "numero_lot": row.get::<_, Option<String>>(1)?,
            "date_peremption": row.get::<_, Option<String>>(2)?,
            "quantite": row.get::<_, f64>(3)?,
            "date_reception": row.get::<_, String>(4)?,
        }))
    }).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_lots_peremption_proche(db: State<DbState>, jours: i64) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT l.id, l.article_id, a.designation, l.numero_lot, l.date_peremption, l.quantite
         FROM article_lots l
         JOIN articles a ON a.id = l.article_id
         WHERE l.quantite > 0 AND l.date_peremption IS NOT NULL
           AND date(l.date_peremption) <= date('now', ?1 || ' days')
         ORDER BY l.date_peremption ASC"
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map(params![jours.to_string()], |row| {
        Ok(serde_json::json!({
            "id": row.get::<_, i64>(0)?,
            "article_id": row.get::<_, i64>(1)?,
            "designation": row.get::<_, String>(2)?,
            "numero_lot": row.get::<_, Option<String>>(3)?,
            "date_peremption": row.get::<_, String>(4)?,
            "quantite": row.get::<_, f64>(5)?,
        }))
    }).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn discard_article_lot(db: State<DbState>, lot_id: i64, quantite: f64, motif: Option<String>) -> Result<(), String> {
    let mut conn = db.conn.lock().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    let (article_id, magasin_id, lot_quantite): (i64, i64, f64) = tx.query_row(
        "SELECT article_id, magasin_id, quantite FROM article_lots WHERE id = ?1",
        params![lot_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ).map_err(|_| "Lot introuvable".to_string())?;

    if quantite <= 0.0 || quantite > lot_quantite {
        return Err(format!("Quantité invalide (disponible dans ce lot : {})", lot_quantite));
    }

    tx.execute("UPDATE article_lots SET quantite = quantite - ?1 WHERE id = ?2", params![quantite, lot_id])
        .map_err(|e| e.to_string())?;
    adjust_article_stock(&tx, article_id, magasin_id, -quantite)?;
    tx.execute(
        "INSERT INTO mouvements_stock (article_id, quantite, mtype, reference_id, reference_type, magasin_id) VALUES (?1, ?2, 'sortie', ?3, ?4, ?5)",
        params![article_id, quantite, lot_id, motif.unwrap_or_else(|| "peremption".to_string()), magasin_id],
    ).map_err(|e| e.to_string())?;

    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

// ─── Variantes (taille/couleur) ───
// Chaque variante a son propre stock dédié et son propre code-barres scannable.
// Le stock d'une variante n'est PAS reflété dans `articles.stock` : un article à
// variantes doit être lu variante par variante côté vente (create_vente accepte un
// `variante_id` optionnel par ligne). Limite connue : pas de suivi par magasin pour
// les variantes (une seule dimension à la fois, cohérent avec l'absence actuelle
// d'UI multi-boutique réelle — cf. ROADMAP_STATUS.md).

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

// ─── Produits composés (kits) ───
// Un kit (articles.est_kit=1) n'a pas son propre stock consommé à la vente : vendre 1
// kit décrémente chaque composant (article_composants) au prorata de sa quantité.
// Le kit garde quand même une fiche article normale (prix, TVA, code-barres) pour être
// scanné/vendu comme n'importe quel article — la résolution kit → composants se fait
// uniquement au moment de la vente, invisible du panier (pas de refonte d'identité de
// panier nécessaire, contrairement aux variantes).

fn get_composants(tx: &Connection, article_id: i64) -> Result<Vec<(i64, f64)>, String> {
    let mut stmt = tx.prepare("SELECT composant_id, quantite FROM article_composants WHERE article_id = ?1")
        .map_err(|e| e.to_string())?;
    let rows = stmt.query_map(params![article_id], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, f64>(1)?))
    }).map_err(|e| e.to_string())?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_article_composant(db: State<DbState>, article_id: i64, composant_id: i64, quantite: f64) -> Result<i64, String> {
    if article_id == composant_id {
        return Err("Un article ne peut pas être son propre composant".to_string());
    }
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO article_composants (article_id, composant_id, quantite) VALUES (?1, ?2, ?3)",
        params![article_id, composant_id, quantite],
    ).map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

#[tauri::command]
pub fn get_article_composants(db: State<DbState>, article_id: i64) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT c.id, c.composant_id, a.designation, a.stock, c.quantite
         FROM article_composants c
         JOIN articles a ON a.id = c.composant_id
         WHERE c.article_id = ?1 ORDER BY a.designation"
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map(params![article_id], |row| {
        Ok(serde_json::json!({
            "id": row.get::<_, i64>(0)?,
            "composant_id": row.get::<_, i64>(1)?,
            "designation": row.get::<_, String>(2)?,
            "stock": row.get::<_, f64>(3)?,
            "quantite": row.get::<_, f64>(4)?,
        }))
    }).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_article_composant_quantite(db: State<DbState>, id: i64, quantite: f64) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute("UPDATE article_composants SET quantite=?1 WHERE id=?2", params![quantite, id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn delete_article_composant(db: State<DbState>, id: i64) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM article_composants WHERE id=?1", params![id]).map_err(|e| e.to_string())?;
    Ok(())
}

// ─── Ventes ───

#[tauri::command]
pub fn create_vente(db: State<DbState>, client_id: Option<i64>, caissier_id: Option<i64>,
    articles: Vec<serde_json::Value>, montant_remise: f64, mode_paiement: String,
    splits: Option<Vec<serde_json::Value>>, dtype: Option<String>,
    points_utilises: Option<f64>, points_gagnes: Option<f64>,
    magasin_id: Option<i64>
) -> Result<i64, String> {
    let mut conn = db.conn.lock().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let magasin_id = match magasin_id {
        Some(id) => id,
        None => {
            if let Some(cid) = caissier_id {
                tx.query_row(
                    "SELECT magasin_id FROM sessions_caisse WHERE caissier_id = ?1 AND statut = 'ouverte' ORDER BY id DESC LIMIT 1",
                    params![cid], |r| r.get::<_, Option<i64>>(0)
                ).ok().flatten().unwrap_or(default_magasin_id(&tx)?)
            } else {
                default_magasin_id(&tx)?
            }
        }
    };
    let mut montant_total = 0.0;
    for a in &articles {
        let qte = a["quantite"].as_f64().unwrap_or(0.0);
        let pu = a["prix_unitaire"].as_f64().unwrap_or(0.0);
        montant_total += qte * pu;
    }

    let net_a_payer = montant_total - montant_remise;
    let document_type = dtype.unwrap_or_else(|| "facture".to_string());
    
    // Déterminer le préfixe
    let prefixe = match document_type.as_str() {
        "devis" => "DE",
        "commande" => "CO",
        "bl" => "BL",
        "avoir" => "AV",
        _ => "FA"
    };
    let ntype = format!("{}_client", document_type);

    // Calcul du crédit demandé
    let mut credit_demandé = 0.0;
    if mode_paiement == "credit" {
        credit_demandé = net_a_payer;
    } else if let Some(ref split_list) = splits {
        for s in split_list {
            if s["mode"].as_str().unwrap_or("") == "credit" {
                credit_demandé += s["montant"].as_f64().unwrap_or(0.0);
            }
        }
    }

    // Vérification du plafond de crédit
    if credit_demandé > 0.0 {
        if let Some(cid) = client_id {
            let (actuel, plafond): (f64, Option<f64>) = tx.query_row(
                "SELECT credit_actuel, credit_plafond FROM clients WHERE id = ?1",
                params![cid],
                |row| Ok((row.get(0)?, row.get(1)?)),
            ).map_err(|e| e.to_string())?;

            if let Some(plaf) = plafond {
                if plaf > 0.0 && actuel + credit_demandé > plaf {
                    return Err(format!("Plafond de crédit dépassé. Crédit actuel: {}, Plafond: {}, Demandé: {}", actuel, plaf, credit_demandé));
                }
            }

            if document_type == "facture" || document_type == "bl" {
                tx.execute("UPDATE clients SET credit_actuel = credit_actuel + ?1 WHERE id = ?2", params![credit_demandé, cid]).map_err(|e| e.to_string())?;
            }
        } else {
            return Err("Un client doit être sélectionné pour payer à crédit.".to_string());
        }
    }

    // Generate DGI sequential invoice number
    let annee = chrono::Local::now().format("%Y").to_string();
    let annee_i: i64 = annee.parse().unwrap_or(2026);
    tx.execute(
        "INSERT INTO numerotation (ntype, annee, prefixe, dernier_numero) VALUES (?1, ?2, ?3, 0)
         ON CONFLICT(ntype) DO UPDATE SET dernier_numero = dernier_numero",
        params![ntype, annee_i, prefixe],
    ).ok();
    tx.execute(
        "UPDATE numerotation SET dernier_numero = dernier_numero + 1 WHERE ntype = ?1 AND annee = ?2",
        params![ntype, annee_i],
    ).map_err(|e| e.to_string())?;
    let dernier_numero: i64 = tx.query_row(
        "SELECT dernier_numero FROM numerotation WHERE ntype = ?1 AND annee = ?2",
        params![ntype, annee_i],
        |r| r.get(0),
    ).map_err(|e| e.to_string())?;
    let numero_facture = format!("{}-{}-{:05}", prefixe, annee, dernier_numero);

    let current_session_id: Option<i64> = if let Some(cid) = caissier_id {
        tx.query_row(
            "SELECT id FROM sessions_caisse WHERE caissier_id = ?1 AND statut = 'ouverte'",
            params![cid],
            |row| row.get(0)
        ).ok()
    } else { None };

    let pts_utilises = points_utilises.unwrap_or(0.0);
    let pts_gagnes = points_gagnes.unwrap_or(0.0);

    tx.execute(
        "INSERT INTO ventes (client_id, caissier_id, montant_total, montant_remise, mode_paiement, numero_facture, dtype, session_id, points_utilises, points_gagnes, magasin_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![client_id, caissier_id, montant_total, montant_remise, mode_paiement, numero_facture, document_type, current_session_id, pts_utilises, pts_gagnes, magasin_id],
    ).map_err(|e| e.to_string())?;
    let vente_id = tx.last_insert_rowid();

    // Mettre à jour le solde de points de fidélité du client et tracer l'historique
    if let Some(cid) = client_id {
        if pts_utilises > 0.0 {
            tx.execute("UPDATE clients SET points_fidelite = MAX(0, points_fidelite - ?1) WHERE id = ?2", params![pts_utilises, cid]).ok();
            tx.execute("INSERT INTO mouvements_fidelite (client_id, vente_id, points, mtype) VALUES (?1, ?2, ?3, 'depense')", params![cid, vente_id, pts_utilises]).ok();
        }
        if pts_gagnes > 0.0 {
            tx.execute("UPDATE clients SET points_fidelite = points_fidelite + ?1 WHERE id = ?2", params![pts_gagnes, cid]).ok();
            tx.execute("INSERT INTO mouvements_fidelite (client_id, vente_id, points, mtype) VALUES (?1, ?2, ?3, 'gain')", params![cid, vente_id, pts_gagnes]).ok();
        }
    }

    for a in &articles {
        let article_id = a["article_id"].as_i64().ok_or("article_id manquant ou invalide dans la ligne")?;
        let variante_id = a["variante_id"].as_i64();
        let qte = a["quantite"].as_f64().unwrap_or(0.0);
        let pu = a["prix_unitaire"].as_f64().unwrap_or(0.0);
        let tva = a["tva"].as_f64().unwrap_or(0.0);
        let remise_ligne = a["remise_ligne"].as_f64().unwrap_or(0.0);
        let note_ligne = a["note"].as_str().map(|s| s.to_string());
        let prix_type = a["prix_type"].as_str().unwrap_or("public").to_string();
        let ligne_base = qte * pu * (1.0 + tva / 100.0);
        let total_ligne = ligne_base * (1.0 - remise_ligne / 100.0);
        tx.execute(
            "INSERT INTO vente_articles (vente_id, article_id, quantite, prix_unitaire, tva, total_ligne, remise_ligne, note, variante_id, prix_type) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![vente_id, article_id, qte, pu, tva, total_ligne, remise_ligne, note_ligne, variante_id, prix_type],
        ).map_err(|e| e.to_string())?;

        if document_type == "facture" || document_type == "bl" {
            if let Some(vid) = variante_id {
                tx.execute("UPDATE article_variantes SET stock_dedie = stock_dedie - ?1 WHERE id = ?2", params![qte, vid])
                    .map_err(|e| e.to_string())?;
                tx.execute(
                    "INSERT INTO mouvements_stock (article_id, quantite, mtype, reference_id, reference_type, magasin_id) VALUES (?1, ?2, 'sortie', ?3, 'vente_variante', ?4)",
                    params![article_id, qte, vid, magasin_id],
                ).map_err(|e| e.to_string())?;
            } else {
                let composants = get_composants(&tx, article_id)?;
                if !composants.is_empty() {
                    for (composant_id, comp_qte) in composants {
                        let qte_composant = comp_qte * qte;
                        adjust_article_stock(&tx, composant_id, magasin_id, -qte_composant)?;
                        tx.execute(
                            "INSERT INTO mouvements_stock (article_id, quantite, mtype, reference_id, reference_type, magasin_id) VALUES (?1, ?2, 'sortie', ?3, 'vente_kit', ?4)",
                            params![composant_id, qte_composant, vente_id, magasin_id],
                        ).map_err(|e| e.to_string())?;
                    }
                } else {
                    adjust_article_stock(&tx, article_id, magasin_id, -qte)?;
                    tx.execute(
                        "INSERT INTO mouvements_stock (article_id, quantite, mtype, reference_id, reference_type, magasin_id) VALUES (?1, ?2, 'sortie', ?3, 'vente', ?4)",
                        params![article_id, qte, vente_id, magasin_id],
                    ).map_err(|e| e.to_string())?;
                }
            }
        }
    }

    tx.commit().map_err(|e| e.to_string())?;

    if let Some(ref split_list) = splits {
        if split_list.len() > 1 {
            let conn2 = db.conn.lock().map_err(|e| e.to_string())?;
            for s in split_list {
                let mode = s["mode"].as_str().unwrap_or("inconnu").to_string();
                let montant = s["montant"].as_f64().unwrap_or(0.0);
                let desc = format!("Split vente #{}: {}", vente_id, mode);
                let _ = conn2.execute(
                    "INSERT INTO journal_caisse (utilisateur_id, jtype, montant, description) VALUES (?1, 'encaissement', ?2, ?3)",
                    params![caissier_id, montant, desc],
                );
            }
        }
    }

    Ok(vente_id)
}

#[tauri::command]
pub fn annuler_vente(db: State<DbState>, vente_id: i64) -> Result<(), String> {
    let mut conn = db.conn.lock().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    let (statut, dtype, vente_magasin_id): (String, String, Option<i64>) = tx.query_row(
        "SELECT statut, COALESCE(dtype, 'facture'), magasin_id FROM ventes WHERE id = ?1",
        params![vente_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ).map_err(|e| e.to_string())?;

    if statut == "annulee" {
        return Err("Cette vente est déjà annulée".to_string());
    }

    let stock_was_deducted = dtype == "facture" || dtype == "bl";
    let is_avoir = dtype == "avoir";

    if stock_was_deducted || is_avoir {
        let magasin_id = vente_magasin_id.map_or_else(|| default_magasin_id(&tx), Ok)?;
        let mut stmt = tx.prepare("SELECT article_id, quantite, variante_id FROM vente_articles WHERE vente_id = ?1").map_err(|e| e.to_string())?;
        let lignes = stmt.query_map(params![vente_id], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, f64>(1)?, row.get::<_, Option<i64>>(2)?))
        }).map_err(|e| e.to_string())?;
        let lignes: Vec<(i64, f64, Option<i64>)> = lignes.collect::<rusqlite::Result<Vec<_>>>().map_err(|e| e.to_string())?;
        drop(stmt);

        let (adjustment_sign, mtype_suffix) = if is_avoir {
            (-1.0_f64, "annulation_avoir")
        } else {
            (1.0_f64, "annulation_vente")
        };

        for (article_id, qte, variante_id) in lignes {
            if let Some(vid) = variante_id {
                let delta = qte * adjustment_sign;
                tx.execute("UPDATE article_variantes SET stock_dedie = stock_dedie + ?1 WHERE id = ?2", params![delta, vid])
                    .map_err(|e| e.to_string())?;
                let mtype = if is_avoir { "sortie" } else { "entree" };
                tx.execute(
                    "INSERT INTO mouvements_stock (article_id, quantite, mtype, reference_id, reference_type, magasin_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![article_id, qte, mtype, vid, format!("{}_variante", mtype_suffix), magasin_id],
                ).map_err(|e| e.to_string())?;
            } else {
                let composants = get_composants(&tx, article_id)?;
                if !composants.is_empty() {
                    for (composant_id, comp_qte) in composants {
                        let qte_composant = comp_qte * qte;
                        adjust_article_stock(&tx, composant_id, magasin_id, qte_composant * adjustment_sign)?;
                        let mtype = if is_avoir { "sortie" } else { "entree" };
                        tx.execute(
                            "INSERT INTO mouvements_stock (article_id, quantite, mtype, reference_id, reference_type, magasin_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                            params![composant_id, qte_composant, mtype, vente_id, format!("{}_kit", mtype_suffix), magasin_id],
                        ).map_err(|e| e.to_string())?;
                    }
                } else {
                    adjust_article_stock(&tx, article_id, magasin_id, qte * adjustment_sign)?;
                    let mtype = if is_avoir { "sortie" } else { "entree" };
                    tx.execute(
                        "INSERT INTO mouvements_stock (article_id, quantite, mtype, reference_id, reference_type, magasin_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                        params![article_id, qte, mtype, vente_id, mtype_suffix, magasin_id],
                    ).map_err(|e| e.to_string())?;
                }
            }
        }
    }

    let numero_facture: Option<String> = tx.query_row(
        "SELECT numero_facture FROM ventes WHERE id = ?1", params![vente_id], |r| r.get(0)
    ).ok();
    let montant: f64 = tx.query_row(
        "SELECT montant_total FROM ventes WHERE id = ?1", params![vente_id], |r| r.get(0)
    ).unwrap_or(0.0);
    tx.execute("UPDATE ventes SET statut = 'annulee' WHERE id = ?1", params![vente_id]).map_err(|e| e.to_string())?;
    log_audit(&tx, None, "annuler_vente",
        &format!("Annulation vente #{} ({}) - Montant: {:.2}", vente_id, numero_facture.unwrap_or_default(), montant),
        Some("vente"), Some(vente_id));
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_ventes(db: State<DbState>, debut: Option<String>, fin: Option<String>) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut where_clause = String::new();
    let mut query_params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    if let Some(d) = &debut {
        if !d.is_empty() {
            where_clause.push_str(" AND v.date >= ?");
            query_params.push(Box::new(d.clone()));
        }
    }
    if let Some(f) = &fin {
        if !f.is_empty() {
            where_clause.push_str(" AND v.date <= ?");
            query_params.push(Box::new(f.clone()));
        }
    }
    let sql = format!(
        "SELECT v.id, v.date, v.client_id, v.caissier_id, v.montant_total, v.montant_remise, v.mode_paiement, v.statut, v.numero_facture,
                c.nom as client_nom, u.nom as caissier_nom, v.dtype, c.telephone as client_telephone, c.email as client_email
         FROM ventes v
         LEFT JOIN clients c ON v.client_id = c.id
         LEFT JOIN utilisateurs u ON v.caissier_id = u.id
         WHERE 1=1 {}
         ORDER BY v.date DESC
         LIMIT 200", where_clause
    );
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let params_refs: Vec<&dyn rusqlite::types::ToSql> = query_params.iter().map(|p| p.as_ref()).collect();
    let rows = stmt.query_map(params_refs.as_slice(), |row| {
        Ok(serde_json::json!({
            "id": row.get::<_, i64>(0)?,
            "date": row.get::<_, String>(1)?,
            "client_id": row.get::<_, Option<i64>>(2)?,
            "caissier_id": row.get::<_, Option<i64>>(3)?,
            "montant_total": row.get::<_, f64>(4)?,
            "montant_remise": row.get::<_, f64>(5)?,
            "mode_paiement": row.get::<_, String>(6)?,
            "statut": row.get::<_, String>(7)?,
            "numero_facture": row.get::<_, Option<String>>(8)?,
            "client_nom": row.get::<_, Option<String>>(9)?,
            "caissier_nom": row.get::<_, Option<String>>(10)?,
            "dtype": row.get::<_, String>(11)?,
            "client_telephone": row.get::<_, Option<String>>(12)?,
            "client_email": row.get::<_, Option<String>>(13)?,
        }))
    }).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_vente_details(db: State<DbState>, vente_id: i64) -> Result<serde_json::Value, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let vente = conn.query_row(
        "SELECT v.id, v.date, v.montant_total, v.montant_remise, v.mode_paiement, v.statut, v.numero_facture,
                c.nom as client_nom, c.telephone as client_tel, u.nom as caissier_nom, v.dtype, c.ice as client_ice,
                v.source_vente_id, src.dtype as source_dtype, src.numero_facture as source_numero
         FROM ventes v
         LEFT JOIN clients c ON v.client_id = c.id
         LEFT JOIN utilisateurs u ON v.caissier_id = u.id
         LEFT JOIN ventes src ON v.source_vente_id = src.id
         WHERE v.id = ?1",
        params![vente_id],
        |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, i64>(0)?,
                "date": row.get::<_, String>(1)?,
                "montant_total": row.get::<_, f64>(2)?,
                "montant_remise": row.get::<_, f64>(3)?,
                "mode_paiement": row.get::<_, String>(4)?,
                "statut": row.get::<_, String>(5)?,
                "numero_facture": row.get::<_, Option<String>>(6)?,
                "client_nom": row.get::<_, Option<String>>(7)?,
                "client_tel": row.get::<_, Option<String>>(8)?,
                "caissier_nom": row.get::<_, Option<String>>(9)?,
                "dtype": row.get::<_, String>(10)?,
                "client_ice": row.get::<_, Option<String>>(11)?,
                "source_vente_id": row.get::<_, Option<i64>>(12)?,
                "source_dtype": row.get::<_, Option<String>>(13)?,
                "source_numero": row.get::<_, Option<String>>(14)?,
            }))
        }
    ).map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT va.id, va.article_id, a.designation, va.quantite, va.prix_unitaire, va.tva, va.total_ligne, va.remise_ligne
         FROM vente_articles va
         JOIN articles a ON va.article_id = a.id
         WHERE va.vente_id = ?1"
    ).map_err(|e| e.to_string())?;
    let lignes = stmt.query_map(params![vente_id], |row| {
        Ok(serde_json::json!({
            "id": row.get::<_, i64>(0)?,
            "article_id": row.get::<_, i64>(1)?,
            "designation": row.get::<_, String>(2)?,
            "quantite": row.get::<_, f64>(3)?,
            "prix_unitaire": row.get::<_, f64>(4)?,
            "tva": row.get::<_, f64>(5)?,
            "total_ligne": row.get::<_, f64>(6)?,
            "remise_ligne": row.get::<_, Option<f64>>(7)?,
        }))
    }).map_err(|e| e.to_string())?;
    let lignes: Vec<_> = lignes.collect::<Result<_, _>>().map_err(|e| e.to_string())?;
    Ok(serde_json::json!({ "vente": vente, "lignes": lignes }))
}

// ─── Conversion de documents (chaîne Devis → Commande → BL → Facture → Avoir) ───

#[tauri::command]
pub fn convert_document(db: State<DbState>, vente_id: i64, target_type: String) -> Result<i64, String> {
    let mut conn = db.conn.lock().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    let (source_dtype, client_id, caissier_id, montant_total, montant_remise, mode_paiement, statut, numero_facture_src, source_magasin_id): (String, Option<i64>, Option<i64>, f64, f64, String, String, Option<String>, Option<i64>) = tx.query_row(
        "SELECT dtype, client_id, caissier_id, montant_total, montant_remise, mode_paiement, statut, numero_facture, magasin_id FROM ventes WHERE id = ?1",
        params![vente_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?, row.get(7)?, row.get(8)?)),
    ).map_err(|_| "Document source introuvable".to_string())?;

    if statut == "annulee" {
        return Err("Impossible de convertir un document annulé".to_string());
    }

    let already_converted: bool = tx.query_row(
        "SELECT COUNT(*) > 0 FROM ventes WHERE source_vente_id = ?1 AND dtype = ?2 AND statut != 'annulee'",
        params![vente_id, target_type],
        |r| r.get(0),
    ).unwrap_or(false);
    if already_converted {
        return Err(format!("Ce document a déjà été converti en {}", target_type));
    }

    let allowed = match source_dtype.as_str() {
        "devis" => target_type == "facture" || target_type == "bl",
        "bl" => target_type == "facture",
        "facture" => target_type == "avoir",
        _ => false,
    };
    if !allowed {
        return Err(format!("Conversion {} → {} non autorisée", source_dtype, target_type));
    }

    let mut stmt = tx.prepare(
        "SELECT article_id, quantite, prix_unitaire, tva, total_ligne, remise_ligne, note, variante_id, prix_type FROM vente_articles WHERE vente_id = ?1"
    ).map_err(|e| e.to_string())?;
    let lignes: Vec<(i64, f64, f64, f64, f64, Option<f64>, Option<String>, Option<i64>, Option<String>)> = stmt.query_map(params![vente_id], |row| {
        Ok((
            row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?,
            row.get(4)?, row.get(5)?, row.get(6)?, row.get(7)?, row.get(8)?,
        ))
    }).map_err(|e| e.to_string())?
      .collect::<rusqlite::Result<Vec<_>>>()
      .map_err(|e| e.to_string())?;
    drop(stmt);

    let prefixe = match target_type.as_str() {
        "devis" => "DE",
        "commande" => "CO",
        "bl" => "BL",
        "avoir" => "AV",
        _ => "FA",
    };
    let ntype = format!("{}_client", target_type);
    let annee = chrono::Local::now().format("%Y").to_string();
    let annee_i: i64 = annee.parse().unwrap_or(2026);
    tx.execute(
        "INSERT INTO numerotation (ntype, annee, prefixe, dernier_numero) VALUES (?1, ?2, ?3, 0)
         ON CONFLICT(ntype) DO UPDATE SET dernier_numero = dernier_numero",
        params![ntype, annee_i, prefixe],
    ).ok();
    tx.execute(
        "UPDATE numerotation SET dernier_numero = dernier_numero + 1 WHERE ntype = ?1 AND annee = ?2",
        params![ntype, annee_i],
    ).map_err(|e| e.to_string())?;
    let dernier_numero: i64 = tx.query_row(
        "SELECT dernier_numero FROM numerotation WHERE ntype = ?1 AND annee = ?2",
        params![ntype, annee_i],
        |r| r.get(0),
    ).map_err(|e| e.to_string())?;
    let numero_facture = format!("{}-{}-{:05}", prefixe, annee, dernier_numero);

    let new_montant_total = if target_type == "avoir" { -montant_total.abs() } else { montant_total };
    let new_montant_remise = if target_type == "avoir" { -montant_remise.abs() } else { montant_remise };

    let magasin_id = source_magasin_id.map_or_else(|| default_magasin_id(&tx), Ok)?;

    tx.execute(
        "INSERT INTO ventes (client_id, caissier_id, montant_total, montant_remise, mode_paiement, numero_facture, dtype, source_vente_id, magasin_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![client_id, caissier_id, new_montant_total, new_montant_remise, mode_paiement, numero_facture, target_type, vente_id, magasin_id],
    ).map_err(|e| e.to_string())?;
    let new_vente_id = tx.last_insert_rowid();

    for (article_id, qte, pu, tva, total_ligne, remise_ligne, note, variante_id, prix_type) in &lignes {
        let new_total_ligne = if target_type == "avoir" { -total_ligne.abs() } else { *total_ligne };
        tx.execute(
            "INSERT INTO vente_articles (vente_id, article_id, quantite, prix_unitaire, tva, total_ligne, remise_ligne, note, variante_id, prix_type) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![new_vente_id, article_id, qte, pu, tva, new_total_ligne, remise_ligne, note, variante_id, prix_type],
        ).map_err(|e| e.to_string())?;

        if target_type == "avoir" {
            if let Some(vid) = variante_id {
                tx.execute("UPDATE article_variantes SET stock_dedie = stock_dedie + ?1 WHERE id = ?2", params![qte, vid])
                    .map_err(|e| e.to_string())?;
                tx.execute(
                    "INSERT INTO mouvements_stock (article_id, quantite, mtype, reference_id, reference_type, magasin_id) VALUES (?1, ?2, 'entree', ?3, 'avoir_variante', ?4)",
                    params![article_id, qte, new_vente_id, magasin_id],
                ).map_err(|e| e.to_string())?;
            } else {
                let composants = get_composants(&tx, *article_id)?;
                if !composants.is_empty() {
                    for (composant_id, comp_qte) in composants {
                        let qte_composant = comp_qte * qte;
                        adjust_article_stock(&tx, composant_id, magasin_id, qte_composant)?;
                        tx.execute(
                            "INSERT INTO mouvements_stock (article_id, quantite, mtype, reference_id, reference_type, magasin_id) VALUES (?1, ?2, 'entree', ?3, 'avoir_kit', ?4)",
                            params![composant_id, qte_composant, new_vente_id, magasin_id],
                        ).map_err(|e| e.to_string())?;
                    }
                } else {
                    adjust_article_stock(&tx, *article_id, magasin_id, *qte)?;
                    tx.execute(
                        "INSERT INTO mouvements_stock (article_id, quantite, mtype, reference_id, reference_type, magasin_id) VALUES (?1, ?2, 'entree', ?3, 'avoir', ?4)",
                        params![article_id, qte, new_vente_id, magasin_id],
                    ).map_err(|e| e.to_string())?;
                }
            }
        }

        if (target_type == "facture" || target_type == "bl") && source_dtype == "devis" {
            if let Some(vid) = variante_id {
                tx.execute("UPDATE article_variantes SET stock_dedie = stock_dedie - ?1 WHERE id = ?2", params![qte, vid])
                    .map_err(|e| e.to_string())?;
                tx.execute(
                    "INSERT INTO mouvements_stock (article_id, quantite, mtype, reference_id, reference_type, magasin_id) VALUES (?1, ?2, 'sortie', ?3, 'vente_variante', ?4)",
                    params![article_id, qte, new_vente_id, magasin_id],
                ).map_err(|e| e.to_string())?;
            } else {
                let composants = get_composants(&tx, *article_id)?;
                if !composants.is_empty() {
                    for (composant_id, comp_qte) in composants {
                        let qte_composant = comp_qte * qte;
                        adjust_article_stock(&tx, composant_id, magasin_id, -qte_composant)?;
                        tx.execute(
                            "INSERT INTO mouvements_stock (article_id, quantite, mtype, reference_id, reference_type, magasin_id) VALUES (?1, ?2, 'sortie', ?3, 'vente_kit', ?4)",
                            params![composant_id, qte_composant, new_vente_id, magasin_id],
                        ).map_err(|e| e.to_string())?;
                    }
                } else {
                    adjust_article_stock(&tx, *article_id, magasin_id, -qte)?;
                    tx.execute(
                        "INSERT INTO mouvements_stock (article_id, quantite, mtype, reference_id, reference_type, magasin_id) VALUES (?1, ?2, 'sortie', ?3, 'vente', ?4)",
                        params![article_id, qte, new_vente_id, magasin_id],
                    ).map_err(|e| e.to_string())?;
                }
            }
        }
    }

    tx.execute(
        "UPDATE ventes SET statut = 'convertie' WHERE id = ?1",
        params![vente_id],
    ).map_err(|e| e.to_string())?;

    if mode_paiement == "credit" && (target_type == "facture" || target_type == "bl") {
        if let Some(cid) = client_id {
            tx.execute(
                "UPDATE clients SET credit_actuel = credit_actuel + ?1 WHERE id = ?2",
                params![new_montant_total.abs(), cid],
            ).map_err(|e| e.to_string())?;
        }
    }
    if target_type == "avoir" {
        if let Some(cid) = client_id {
            if mode_paiement == "credit" {
                tx.execute(
                    "UPDATE clients SET credit_actuel = credit_actuel - ?1 WHERE id = ?2",
                    params![montant_total.abs(), cid],
                ).map_err(|e| e.to_string())?;
            }
        }
    }

    let source_ref = numero_facture_src.unwrap_or_else(|| format!("#{}", vente_id));
    log_audit(&tx, caissier_id, "convertir_document",
        &format!("Conversion {} {} → {} {}", source_dtype, source_ref, target_type, numero_facture),
        Some("vente"), Some(new_vente_id));

    tx.commit().map_err(|e| e.to_string())?;
    Ok(new_vente_id)
}

// ─── Achats ───

#[tauri::command]
pub fn create_achat(db: State<DbState>, fournisseur_id: Option<i64>, reference: Option<String>,
    articles: Vec<serde_json::Value>, statut_livraison: Option<String>, statut_paiement: Option<String>) -> Result<i64, String> {
    let mut conn = db.conn.lock().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let magasin_id = default_magasin_id(&tx)?;
    let mut montant_total = 0.0;
    for a in &articles {
        let qte = a["quantite"].as_f64().unwrap_or(0.0);
        let pu = a["prix_unitaire"].as_f64().unwrap_or(0.0);
        montant_total += qte * pu;
    }
    let sl = statut_livraison.unwrap_or_else(|| "recu".to_string());
    let sp = statut_paiement.unwrap_or_else(|| "non_paye".to_string());

    tx.execute(
        "INSERT INTO achats (fournisseur_id, reference, montant_total, statut_livraison, statut_paiement) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![fournisseur_id, reference, montant_total, sl, sp],
    ).map_err(|e| e.to_string())?;
    let achat_id = tx.last_insert_rowid();
    for a in &articles {
        let article_id = a["article_id"].as_i64().ok_or("article_id manquant ou invalide dans la ligne")?;
        let qte = a["quantite"].as_f64().unwrap_or(0.0);
        let pu = a["prix_unitaire"].as_f64().unwrap_or(0.0);
        let total_ligne = qte * pu;
        tx.execute(
            "INSERT INTO achat_articles (achat_id, article_id, quantite, prix_unitaire, total_ligne) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![achat_id, article_id, qte, pu, total_ligne],
        ).map_err(|e| e.to_string())?;

        if sl == "recu" {
            tx.execute("UPDATE articles SET prix_achat = ?1 WHERE id = ?2", params![pu, article_id])
                .map_err(|e| e.to_string())?;
            adjust_article_stock(&tx, article_id, magasin_id, qte)?;
            tx.execute(
                "INSERT INTO mouvements_stock (article_id, quantite, mtype, reference_id, reference_type, magasin_id) VALUES (?1, ?2, 'entree', ?3, 'achat', ?4)",
                params![article_id, qte, achat_id, magasin_id],
            ).map_err(|e| e.to_string())?;
        }
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(achat_id)
}

#[tauri::command]
pub fn get_achats(db: State<DbState>) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT a.id, a.date, a.fournisseur_id, a.reference, a.montant_total, a.statut, f.nom as fournisseur_nom, a.statut_livraison, a.statut_paiement
         FROM achats a LEFT JOIN fournisseurs f ON a.fournisseur_id = f.id
         ORDER BY a.date DESC LIMIT 200"
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |row| {
        Ok(serde_json::json!({
            "id": row.get::<_, i64>(0)?,
            "date": row.get::<_, String>(1)?,
            "fournisseur_id": row.get::<_, Option<i64>>(2)?,
            "reference": row.get::<_, Option<String>>(3)?,
            "montant_total": row.get::<_, f64>(4)?,
            "statut": row.get::<_, String>(5)?,
            "fournisseur_nom": row.get::<_, Option<String>>(6)?,
            "statut_livraison": row.get::<_, String>(7)?,
            "statut_paiement": row.get::<_, String>(8)?,
        }))
    }).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

// ─── Cheques ───

#[tauri::command]
pub fn get_cheques(db: State<DbState>) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT c.id, c.numero, c.banque, c.tireur, c.montant, c.date_emission, c.date_echeance, c.statut, c.ctype, c.client_id, c.fournisseur_id,
                cl.nom as client_nom, f.nom as fournisseur_nom
         FROM cheques c
         LEFT JOIN clients cl ON c.client_id = cl.id
         LEFT JOIN fournisseurs f ON c.fournisseur_id = f.id
         ORDER BY c.date_echeance ASC"
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |row| {
        Ok(serde_json::json!({
            "id": row.get::<_, i64>(0)?,
            "numero": row.get::<_, String>(1)?,
            "banque": row.get::<_, String>(2)?,
            "tireur": row.get::<_, Option<String>>(3)?,
            "montant": row.get::<_, f64>(4)?,
            "date_emission": row.get::<_, String>(5)?,
            "date_echeance": row.get::<_, String>(6)?,
            "statut": row.get::<_, String>(7)?,
            "ctype": row.get::<_, String>(8)?,
            "client_id": row.get::<_, Option<i64>>(9)?,
            "fournisseur_id": row.get::<_, Option<i64>>(10)?,
            "client_nom": row.get::<_, Option<String>>(11)?,
            "fournisseur_nom": row.get::<_, Option<String>>(12)?,
        }))
    }).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_cheque(db: State<DbState>, numero: String, banque: String, tireur: Option<String>, montant: f64,
                  date_emission: String, date_echeance: String, ctype: String,
                  client_id: Option<i64>, fournisseur_id: Option<i64>) -> Result<i64, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO cheques (numero, banque, tireur, montant, date_emission, date_echeance, ctype, client_id, fournisseur_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![numero, banque, tireur, montant, date_emission, date_echeance, ctype, client_id, fournisseur_id],
    ).map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

#[tauri::command]
pub fn update_cheque_status(db: State<DbState>, cheque_id: i64, statut: String) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute("UPDATE cheques SET statut = ?1 WHERE id = ?2", params![statut, cheque_id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ─── Sessions Caisse ───

#[tauri::command]
pub fn get_current_session(db: State<DbState>, caissier_id: i64) -> Result<Option<serde_json::Value>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT id, caissier_id, date_ouverture, fond_initial, statut, magasin_id
         FROM sessions_caisse
         WHERE caissier_id = ?1 AND statut = 'ouverte'
         ORDER BY id DESC LIMIT 1"
    ).map_err(|e| e.to_string())?;

    let mut rows = stmt.query_map(params![caissier_id], |row| {
        Ok(serde_json::json!({
            "id": row.get::<_, i64>(0)?,
            "caissier_id": row.get::<_, i64>(1)?,
            "date_ouverture": row.get::<_, String>(2)?,
            "fond_initial": row.get::<_, f64>(3)?,
            "statut": row.get::<_, String>(4)?,
            "magasin_id": row.get::<_, Option<i64>>(5)?
        }))
    }).map_err(|e| e.to_string())?;

    if let Some(row) = rows.next() {
        return Ok(Some(row.map_err(|e| e.to_string())?));
    }
    Ok(None)
}

#[tauri::command]
pub fn open_session(db: State<DbState>, caissier_id: i64, fond_initial: f64, magasin_id: Option<i64>) -> Result<i64, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let count: i64 = conn.query_row(
        "SELECT count(*) FROM sessions_caisse WHERE caissier_id = ?1 AND statut = 'ouverte'",
        params![caissier_id],
        |row| row.get(0)
    ).unwrap_or(0);

    if count > 0 {
        return Err("Une session est déjà ouverte pour ce caissier".to_string());
    }

    let mid = match magasin_id {
        Some(id) => id,
        None => default_magasin_id(&conn)?,
    };
    conn.execute(
        "INSERT INTO sessions_caisse (caissier_id, fond_initial, statut, magasin_id) VALUES (?1, ?2, 'ouverte', ?3)",
        params![caissier_id, fond_initial, mid]
    ).map_err(|e| e.to_string())?;

    Ok(conn.last_insert_rowid())
}

#[tauri::command]
pub fn add_magasin(db: State<DbState>, nom: String, adresse: Option<String>) -> Result<i64, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO magasins (nom, adresse) VALUES (?1, ?2)",
        params![nom, adresse],
    ).map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

#[tauri::command]
pub fn update_magasin(db: State<DbState>, id: i64, nom: String, adresse: Option<String>) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE magasins SET nom=?1, adresse=?2 WHERE id=?3",
        params![nom, adresse, id],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn delete_magasin(db: State<DbState>, id: i64) -> Result<(), String> {
    let mut conn = db.conn.lock().map_err(|e| e.to_string())?;
    let count: i64 = conn.query_row(
        "SELECT count(*) FROM magasins", [], |r| r.get(0)
    ).unwrap_or(0);
    if count <= 1 {
        return Err("Impossible de supprimer le dernier magasin".to_string());
    }
    let has_sessions: bool = conn.query_row(
        "SELECT count(*) > 0 FROM sessions_caisse WHERE magasin_id = ?1", params![id], |r| r.get(0)
    ).unwrap_or(false);
    if has_sessions {
        return Err("Ce magasin a des sessions de caisse associées".to_string());
    }
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM article_stocks WHERE magasin_id = ?1", params![id]).map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM magasins WHERE id = ?1", params![id]).map_err(|e| e.to_string())?;
    tx.execute(
        "UPDATE articles SET stock = COALESCE((SELECT SUM(quantite) FROM article_stocks WHERE article_id = articles.id), 0)",
        [],
    ).map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_transferts(db: State<DbState>) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT t.id, t.date, t.statut, ms.nom AS source_nom, md.nom AS dest_nom, u.nom AS utilisateur_nom
         FROM transferts_stock t
         JOIN magasins ms ON ms.id = t.source_id
         JOIN magasins md ON md.id = t.dest_id
         LEFT JOIN utilisateurs u ON u.id = t.utilisateur_id
         ORDER BY t.date DESC"
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |row| {
        Ok(serde_json::json!({
            "id": row.get::<_, i64>(0)?,
            "date": row.get::<_, String>(1)?,
            "statut": row.get::<_, String>(2)?,
            "source_nom": row.get::<_, String>(3)?,
            "dest_nom": row.get::<_, String>(4)?,
            "utilisateur_nom": row.get::<_, Option<String>>(5)?
        }))
    }).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_stock_par_magasin(db: State<DbState>, magasin_id: i64) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT a.id, a.designation, a.code_barre, COALESCE(s.quantite, 0) AS stock, a.stock_alerte
         FROM articles a
         LEFT JOIN article_stocks s ON s.article_id = a.id AND s.magasin_id = ?1
         WHERE a.actif = 1
         ORDER BY a.designation"
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map(params![magasin_id], |row| {
        Ok(serde_json::json!({
            "id": row.get::<_, i64>(0)?,
            "designation": row.get::<_, String>(1)?,
            "code_barre": row.get::<_, Option<String>>(2)?,
            "stock": row.get::<_, f64>(3)?,
            "stock_alerte": row.get::<_, Option<f64>>(4)?
        }))
    }).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn close_session(db: State<DbState>, session_id: i64, total_especes_declare: f64) -> Result<(), String> {
    let mut conn = db.conn.lock().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    let fond_initial: f64 = tx.query_row(
        "SELECT fond_initial FROM sessions_caisse WHERE id = ?1 AND statut = 'ouverte'",
        params![session_id],
        |row| row.get(0)
    ).map_err(|_| "Session introuvable ou déjà clôturée".to_string())?;

    // Calcul des ventes en espèces
    let ventes_especes: f64 = tx.query_row(
        "SELECT COALESCE(SUM(montant), 0) FROM paiements WHERE ptype = 'especes' AND date >= (SELECT date_ouverture FROM sessions_caisse WHERE id = ?1)",
        params![session_id],
        |row| row.get(0)
    ).unwrap_or(0.0);
    
    // Calcul des entrées/sorties de caisse (Journal)
    let entrees_caisse: f64 = tx.query_row(
        "SELECT COALESCE(SUM(montant), 0) FROM journal_caisse WHERE jtype = 'entree' AND session_id = ?1",
        params![session_id],
        |row| row.get(0)
    ).unwrap_or(0.0);
    
    let sorties_caisse: f64 = tx.query_row(
        "SELECT COALESCE(SUM(montant), 0) FROM journal_caisse WHERE jtype = 'sortie' AND session_id = ?1",
        params![session_id],
        |row| row.get(0)
    ).unwrap_or(0.0);

    let total_attendu = fond_initial + ventes_especes + entrees_caisse - sorties_caisse;
    let ecart = total_especes_declare - total_attendu;
    let date_cloture = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    tx.execute(
        "UPDATE sessions_caisse 
         SET date_cloture = ?1, total_especes_attendu = ?2, total_especes_declare = ?3, ecart = ?4, statut = 'cloturee' 
         WHERE id = ?5",
        params![date_cloture, total_attendu, total_especes_declare, ecart, session_id]
    ).map_err(|e| e.to_string())?;

    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn update_achat_status(db: State<DbState>, achat_id: i64, statut_livraison: String, statut_paiement: String) -> Result<(), String> {
    let mut conn = db.conn.lock().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    let old_sl: String = tx.query_row(
        "SELECT statut_livraison FROM achats WHERE id = ?1",
        params![achat_id],
        |row| row.get(0),
    ).map_err(|e| e.to_string())?;

    tx.execute(
        "UPDATE achats SET statut_livraison = ?1, statut_paiement = ?2 WHERE id = ?3",
        params![statut_livraison, statut_paiement, achat_id],
    ).map_err(|e| e.to_string())?;

    if old_sl != "recu" && statut_livraison == "recu" {
        let magasin_id = default_magasin_id(&tx)?;
        let mut stmt = tx.prepare("SELECT article_id, quantite, prix_unitaire FROM achat_articles WHERE achat_id = ?1").map_err(|e| e.to_string())?;
        let lignes = stmt.query_map(params![achat_id], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, f64>(1)?, row.get::<_, f64>(2)?))
        }).map_err(|e| e.to_string())?;
        let lignes: Vec<(i64, f64, f64)> = lignes.collect::<rusqlite::Result<Vec<_>>>().map_err(|e| e.to_string())?;
        drop(stmt);

        for (article_id, qte, pu) in lignes {
            tx.execute("UPDATE articles SET prix_achat = ?1 WHERE id = ?2", params![pu, article_id]).map_err(|e| e.to_string())?;
            adjust_article_stock(&tx, article_id, magasin_id, qte)?;
            tx.execute(
                "INSERT INTO mouvements_stock (article_id, quantite, mtype, reference_id, reference_type, magasin_id) VALUES (?1, ?2, 'entree', ?3, 'achat_reception', ?4)",
                params![article_id, qte, achat_id, magasin_id],
            ).map_err(|e| e.to_string())?;
        }
    }

    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

// ─── Paiements Crédit ───

#[tauri::command]
pub fn get_paiements(db: State<DbState>, client_id: Option<i64>) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let (sql, params_vec) = match client_id {
        Some(cid) => (
            "SELECT p.id, p.client_id, p.date, p.montant, p.type, p.reference, c.nom as client_nom
             FROM paiements p JOIN clients c ON p.client_id = c.id WHERE p.client_id = ?1 ORDER BY p.date DESC LIMIT 100".to_string(),
            vec![Box::new(cid) as Box<dyn rusqlite::types::ToSql>]
        ),
        None => (
            "SELECT p.id, p.client_id, p.date, p.montant, p.type, p.reference, c.nom as client_nom
             FROM paiements p JOIN clients c ON p.client_id = c.id ORDER BY p.date DESC LIMIT 200".to_string(),
            vec![]
        ),
    };
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let pr: Vec<&dyn rusqlite::types::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
    let map_row = |row: &rusqlite::Row| -> rusqlite::Result<serde_json::Value> {
        Ok(serde_json::json!({
            "id": row.get::<_, i64>(0)?,
            "client_id": row.get::<_, i64>(1)?,
            "date": row.get::<_, String>(2)?,
            "montant": row.get::<_, f64>(3)?,
            "type": row.get::<_, String>(4)?,
            "reference": row.get::<_, Option<String>>(5)?,
            "client_nom": row.get::<_, Option<String>>(6)?,
        }))
    };
    let rows = stmt.query_map(pr.as_slice(), map_row).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_paiement(db: State<DbState>, client_id: i64, montant: f64, ptype: String, reference: Option<String>) -> Result<i64, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO paiements (client_id, montant, type, reference) VALUES (?1, ?2, ?3, ?4)",
        params![client_id, montant, ptype, reference],
    ).map_err(|e| e.to_string())?;
    conn.execute("UPDATE clients SET credit_actuel = credit_actuel - ?1 WHERE id = ?2",
        params![montant, client_id]).map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

// ─── Mouvements Stock ───

#[tauri::command]
pub fn get_mouvements_stock(db: State<DbState>, article_id: Option<i64>, debut: Option<String>, fin: Option<String>) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
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

// ─── Import articles ───

#[tauri::command]
pub fn import_articles_csv(db: State<DbState>, csv_content: String) -> Result<String, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let magasin_id = default_magasin_id(&conn)?;
    let mut imported = 0u32;
    let mut errors: Vec<String> = Vec::new();

    for (i, line) in csv_content.lines().enumerate() {
        if i == 0 { continue; }
        if line.trim().is_empty() { continue; }
        let cols: Vec<&str> = line.split(';').collect();
        if cols.len() < 5 {
            errors.push(format!("Ligne {}: format invalide (minimum 5 colonnes attendues)", i+1));
            continue;
        }
        let designation = cols[0].trim().trim_matches('"');
        let code_barre = if cols[1].trim().is_empty() || cols[1].trim() == "\"\"" { None } else { Some(cols[1].trim().trim_matches('"').to_string()) };
        let prix_achat: f64 = cols[2].trim().trim_matches('"').replace(',', ".").parse().unwrap_or(0.0);
        let prix_vente: f64 = cols[3].trim().trim_matches('"').replace(',', ".").parse().unwrap_or(0.0);
        let tva: f64 = if cols.len() > 4 { cols[4].trim().trim_matches('"').replace(',', ".").parse().unwrap_or(0.0) } else { 0.0 };
        let stock: f64 = if cols.len() > 5 { cols[5].trim().trim_matches('"').replace(',', ".").parse().unwrap_or(0.0) } else { 0.0 };
        let stock_alerte: Option<f64> = if cols.len() > 6 && !cols[6].trim().is_empty() && cols[6].trim() != "\"\"" {
            Some(cols[6].trim().trim_matches('"').replace(',', ".").parse().unwrap_or(0.0))
        } else { None };
        let image_url: Option<String> = if cols.len() > 7 && !cols[7].trim().is_empty() && cols[7].trim() != "\"\"" {
            Some(cols[7].trim().trim_matches('"').to_string())
        } else { None };

        match conn.execute(
            "INSERT INTO articles (code_barre, designation, prix_achat, prix_vente, tva, stock, stock_alerte, image_url)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![code_barre, designation, prix_achat, prix_vente, tva, stock, stock_alerte, image_url],
        ) {
            Ok(_) => {
                imported += 1;
                if stock != 0.0 {
                    let article_id = conn.last_insert_rowid();
                    if let Err(e) = adjust_article_stock(&conn, article_id, magasin_id, stock) {
                        errors.push(format!("Ligne {} (stock): {}", i+1, e));
                    }
                }
            }
            Err(e) => errors.push(format!("Ligne {}: {}", i+1, e)),
        }
    }

    let mut report = format!("Import terminé. {} articles importés.", imported);
    if !errors.is_empty() {
        report.push_str(&format!("\n{} erreur(s):\n{}", errors.len(), errors.join("\n")));
    }
    Ok(report)
}

// ─── Stats ───

#[tauri::command]
pub fn get_stats(db: State<DbState>) -> Result<serde_json::Value, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    
    // Statistiques globales
    let total_ventes_30j: f64 = conn.query_row("SELECT COALESCE(SUM(montant_total - montant_remise),0) FROM ventes WHERE date >= datetime('now','-30 days','localtime') AND statut != 'annulee'", [], |r| r.get(0)).unwrap_or(0.0);
    let nb_articles: i64 = conn.query_row("SELECT COUNT(*) FROM articles WHERE actif=1", [], |r| r.get(0)).unwrap_or(0);
    let stock_alerte: i64 = conn.query_row("SELECT COUNT(*) FROM articles WHERE stock <= stock_alerte AND stock_alerte > 0", [], |r| r.get(0)).unwrap_or(0);
    let credit_total: f64 = conn.query_row("SELECT COALESCE(SUM(credit_actuel),0) FROM clients", [], |r| r.get(0)).unwrap_or(0.0);
    let nb_clients: i64 = conn.query_row("SELECT COUNT(*) FROM clients", [], |r| r.get(0)).unwrap_or(0);

    // CA et Marge
    let ca_jour: f64 = conn.query_row("SELECT COALESCE(SUM(montant_total - montant_remise),0) FROM ventes WHERE date >= date('now','localtime') AND statut != 'annulee'", [], |r| r.get(0)).unwrap_or(0.0);
    let ca_mois: f64 = conn.query_row("SELECT COALESCE(SUM(montant_total - montant_remise),0) FROM ventes WHERE strftime('%Y-%m', date) = strftime('%Y-%m', 'now', 'localtime') AND statut != 'annulee'", [], |r| r.get(0)).unwrap_or(0.0);
    
    // Calcul du bénéfice du mois (CA - Coût d'achat)
    let cout_achats_mois: f64 = conn.query_row("
        SELECT COALESCE(SUM(vl.quantite * a.prix_achat), 0)
        FROM vente_articles vl
        JOIN ventes v ON v.id = vl.vente_id
        JOIN articles a ON a.id = vl.article_id
        WHERE strftime('%Y-%m', v.date) = strftime('%Y-%m', 'now', 'localtime') AND v.statut != 'annulee'
    ", [], |r| r.get(0)).unwrap_or(0.0);
    let benefice_mois = ca_mois - cout_achats_mois;

    // Top 5 Articles
    let mut stmt = conn.prepare("
        SELECT a.designation, SUM(vl.quantite) as qte_vendue
        FROM vente_articles vl
        JOIN ventes v ON v.id = vl.vente_id
        JOIN articles a ON a.id = vl.article_id
        WHERE v.date >= datetime('now','-30 days','localtime') AND v.statut != 'annulee'
        GROUP BY a.id
        ORDER BY qte_vendue DESC
        LIMIT 5
    ").map_err(|e| e.to_string())?;
    let top_articles = stmt.query_map([], |r| {
        Ok(serde_json::json!({
            "designation": r.get::<_, String>(0)?,
            "quantite": r.get::<_, f64>(1)?
        }))
    }).map_err(|e| e.to_string())?.filter_map(Result::ok).collect::<Vec<_>>();

    // Top 5 Clients
    let mut stmt = conn.prepare("
        SELECT c.nom, SUM(v.montant_total - v.montant_remise) as depense
        FROM ventes v
        JOIN clients c ON c.id = v.client_id
        WHERE v.date >= datetime('now','-30 days','localtime') AND v.statut != 'annulee'
        GROUP BY c.id
        ORDER BY depense DESC
        LIMIT 5
    ").map_err(|e| e.to_string())?;
    let top_clients = stmt.query_map([], |r| {
        Ok(serde_json::json!({
            "nom": r.get::<_, String>(0)?,
            "depense": r.get::<_, f64>(1)?
        }))
    }).map_err(|e| e.to_string())?.filter_map(Result::ok).collect::<Vec<_>>();

    Ok(serde_json::json!({
        "total_ventes_30j": total_ventes_30j,
        "nb_articles": nb_articles,
        "stock_alerte": stock_alerte,
        "credit_total": credit_total,
        "nb_clients": nb_clients,
        "ca_jour": ca_jour,
        "ca_mois": ca_mois,
        "benefice_mois": benefice_mois,
        "top_articles": top_articles,
        "top_clients": top_clients,
    }))
}

// ─── Impression ───

#[tauri::command]
pub fn print_ticket(texte: String) -> Result<(), String> {
    let path = std::env::temp_dir().join("ticket_impression.txt");
    std::fs::write(&path, &texte).map_err(|e| format!("Erreur écriture ticket: {}", e))?;

    if cfg!(target_os = "windows") {
        let path_str = path.to_string_lossy().replace("'", "''");
        let ps = format!(
            "Start-Process -FilePath 'notepad.exe' -ArgumentList '/p', '{}' -WindowStyle Hidden -Wait",
            path_str
        );
        let _ = std::process::Command::new("powershell")
            .args(["-NonInteractive", "-Command", &ps])
            .output();
    } else {
        let _ = std::process::Command::new("lp")
            .arg(path.to_string_lossy().as_ref())
            .output();
    }
    Ok(())
}

#[tauri::command]
pub fn print_escpos(db: State<DbState>, base64_data: String) -> Result<(), String> {
    let printer_name: String = {
        let conn = db.conn.lock().map_err(|e| e.to_string())?;
        conn.query_row("SELECT value FROM settings WHERE key = 'printer_name'", [], |r| r.get(0))
            .unwrap_or_else(|_| "POS-80".to_string())
    };

    let bytes = general_purpose::STANDARD.decode(base64_data)
        .map_err(|e| format!("Erreur de décodage base64: {}", e))?;

    let path = std::env::temp_dir().join("ticket_escpos.bin");
    std::fs::write(&path, &bytes).map_err(|e| format!("Erreur d'écriture du flux binaire: {}", e))?;

    let path_str = path.to_string_lossy().to_string();

    if cfg!(target_os = "windows") {
        let printer_path = if printer_name.starts_with("\\\\") {
            printer_name.clone()
        } else {
            format!("\\\\localhost\\{}", printer_name)
        };
        let args = format!("COPY /B \"{}\" \"{}\"", path_str, printer_path);
        let output = std::process::Command::new("cmd")
            .args(["/c", &args])
            .output()
            .map_err(|e| e.to_string())?;
        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Erreur d'impression ESC/POS: {}", err));
        }
    } else if printer_name.starts_with("/dev/") {
        std::fs::write(&printer_name, &bytes)
            .map_err(|e| format!("Erreur écriture vers {}: {}", printer_name, e))?;
    } else {
        let output = std::process::Command::new("lp")
            .args(["-d", &printer_name, "-o", "raw", &path_str])
            .output()
            .map_err(|e| format!("Erreur lp: {}", e))?;
        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Erreur d'impression ESC/POS: {}", err));
        }
    }

    Ok(())
}

#[tauri::command]
pub fn open_cash_drawer(db: State<DbState>) -> Result<(), String> {
    // Commande ESC/POS universelle pour ouvrir le tiroir-caisse (ESC p m t1 t2)
    // m = 0 (Pin 2), t1 = 25 (* 2ms = 50ms), t2 = 250 (* 2ms = 500ms)
    let drawer_kick = vec![0x1B, 0x70, 0x00, 0x19, 0xFA];
    let base64_data = general_purpose::STANDARD.encode(&drawer_kick);
    print_escpos(db, base64_data)
}

#[tauri::command]
pub fn get_articles_stock_alerte(db: State<DbState>) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
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

// ─── Journal Caisse ───

#[tauri::command]
pub fn get_journal_caisse(db: State<DbState>, debut: Option<String>, fin: Option<String>) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
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
pub fn add_journal_caisse(db: State<DbState>, utilisateur_id: Option<i64>, jtype: String, montant: f64, description: Option<String>) -> Result<i64, String> {
    let mut conn = db.conn.lock().map_err(|e| e.to_string())?;
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

// ─── Magasins & Transferts (Multi-Dépôts) ───

#[tauri::command]
pub fn get_magasins(db: State<DbState>) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare("SELECT id, nom, adresse FROM magasins ORDER BY id").map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |row| {
        Ok(serde_json::json!({
            "id": row.get::<_, i64>(0)?,
            "nom": row.get::<_, String>(1)?,
            "adresse": row.get::<_, Option<String>>(2)?
        }))
    }).map_err(|e| e.to_string())?;
    
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_transfert(
    db: State<DbState>, 
    source_id: i64, 
    dest_id: i64, 
    utilisateur_id: Option<i64>, 
    articles: Vec<serde_json::Value>
) -> Result<i64, String> {
    let mut conn = db.conn.lock().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    tx.execute(
        "INSERT INTO transferts_stock (source_id, dest_id, utilisateur_id, statut) VALUES (?1, ?2, ?3, 'en_attente')",
        params![source_id, dest_id, utilisateur_id]
    ).map_err(|e| e.to_string())?;
    
    let transfert_id = tx.last_insert_rowid();

    for a in articles {
        let article_id = a["article_id"].as_i64().ok_or("article_id manquant ou invalide dans la ligne")?;
        let quantite = a["quantite"].as_f64().unwrap_or(0.0);
        tx.execute(
            "INSERT INTO transfert_lignes (transfert_id, article_id, quantite) VALUES (?1, ?2, ?3)",
            params![transfert_id, article_id, quantite]
        ).map_err(|e| e.to_string())?;
    }

    tx.commit().map_err(|e| e.to_string())?;
    Ok(transfert_id)
}

#[tauri::command]
pub fn validate_transfert(db: State<DbState>, transfert_id: i64) -> Result<(), String> {
    let mut conn = db.conn.lock().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    let (source_id, dest_id, statut): (i64, i64, String) = tx.query_row(
        "SELECT source_id, dest_id, statut FROM transferts_stock WHERE id = ?1",
        params![transfert_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))
    ).map_err(|_| "Transfert introuvable".to_string())?;

    if statut != "en_attente" {
        return Err("Ce transfert a déjà été traité".to_string());
    }

    let mut stmt = tx.prepare("SELECT article_id, quantite FROM transfert_lignes WHERE transfert_id = ?1").map_err(|e| e.to_string())?;
    let lignes = stmt.query_map(params![transfert_id], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, f64>(1)?))).map_err(|e| e.to_string())?;

    for res in lignes {
        let (article_id, qte) = res.map_err(|e| e.to_string())?;

        // 1. Soustraire de la source (upsert : évite de perdre la sortie si l'article
        // n'avait encore aucune ligne article_stocks pour ce magasin)
        adjust_article_stock(&tx, article_id, source_id, -qte)?;
        tx.execute(
            "INSERT INTO mouvements_stock (article_id, quantite, mtype, reference_id, reference_type, magasin_id) VALUES (?1, ?2, 'sortie', ?3, 'transfert', ?4)",
            params![article_id, qte, transfert_id, source_id]
        ).map_err(|e| e.to_string())?;

        // 2. Ajouter à la destination
        adjust_article_stock(&tx, article_id, dest_id, qte)?;
        tx.execute(
            "INSERT INTO mouvements_stock (article_id, quantite, mtype, reference_id, reference_type, magasin_id) VALUES (?1, ?2, 'entree', ?3, 'transfert', ?4)",
            params![article_id, qte, transfert_id, dest_id]
        ).map_err(|e| e.to_string())?;
    }

    tx.execute("UPDATE transferts_stock SET statut = 'valide' WHERE id = ?1", params![transfert_id]).map_err(|e| e.to_string())?;

    drop(stmt);
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

// ─── Categories (suite) ───

#[tauri::command]
pub fn update_category(db: State<DbState>, id: i64, nom: String, description: Option<String>) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE categories SET nom=?1, description=?2 WHERE id=?3",
        params![nom, description, id],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

// ─── Utilisateurs ───

#[tauri::command]
pub fn get_utilisateurs(db: State<DbState>) -> Result<Vec<Utilisateur>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare("SELECT id, login, nom, role FROM utilisateurs ORDER BY nom")
        .map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |row| {
        Ok(Utilisateur {
            id: Some(row.get(0)?),
            login: row.get(1)?,
            nom: row.get(2)?,
            role: row.get(3)?,
        })
    }).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_utilisateur(db: State<DbState>, login: String, nom: String, role: String, password: String) -> Result<i64, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let hash = hash_password(&password);
    conn.execute(
        "INSERT INTO utilisateurs (login, password_hash, nom, role) VALUES (?1, ?2, ?3, ?4)",
        params![login, hash, nom, role],
    ).map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

#[tauri::command]
pub fn update_utilisateur(db: State<DbState>, id: i64, login: String, nom: String, role: String, password: Option<String>) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    match password {
        Some(ref pwd) if !pwd.is_empty() => {
            let hash = hash_password(pwd);
            conn.execute(
                "UPDATE utilisateurs SET login=?1, nom=?2, role=?3, password_hash=?4 WHERE id=?5",
                params![login, nom, role, hash, id],
            ).map_err(|e| e.to_string())?;
        }
        _ => {
            conn.execute(
                "UPDATE utilisateurs SET login=?1, nom=?2, role=?3 WHERE id=?4",
                params![login, nom, role, id],
            ).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[tauri::command]
pub fn delete_utilisateur(db: State<DbState>, id: i64) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM utilisateurs WHERE id=?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ─── Tables Restaurant ───

#[tauri::command]
pub fn get_tables(db: State<DbState>) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare("SELECT id, nom, statut, ticket_id FROM tables_resto ORDER BY id").map_err(|e| e.to_string())?;
    let tables = stmt.query_map([], |row| {
        Ok(serde_json::json!({
            "id": row.get::<_, i64>(0)?,
            "nom": row.get::<_, String>(1)?,
            "statut": row.get::<_, String>(2)?,
            "ticket_id": row.get::<_, Option<String>>(3)?,
        }))
    }).map_err(|e| e.to_string())?.filter_map(Result::ok).collect::<Vec<_>>();
    Ok(tables)
}

#[tauri::command]
pub fn update_table_status(db: State<DbState>, id: i64, statut: String, ticket_id: Option<String>) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE tables_resto SET statut = ?1, ticket_id = ?2 WHERE id = ?3",
        params![statut, ticket_id, id]
    ).map_err(|e| e.to_string())?;
    Ok(())
}

// ─── Settings ───

#[tauri::command]
pub fn get_settings(db: State<DbState>) -> Result<Settings, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare("SELECT key, value FROM settings")
        .map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    }).map_err(|e| e.to_string())?;
    let mut map = std::collections::HashMap::new();
    for row in rows {
        let (k, v) = row.map_err(|e| e.to_string())?;
        map.insert(k, v);
    }
    Ok(Settings {
        shop_name: map.get("shop_name").cloned().unwrap_or_else(|| "SuperCaisse".to_string()),
        shop_address: map.get("shop_address").filter(|s| !s.is_empty()).cloned(),
        shop_phone: map.get("shop_phone").filter(|s| !s.is_empty()).cloned(),
        shop_email: map.get("shop_email").filter(|s| !s.is_empty()).cloned(),
        ice: map.get("ice").filter(|s| !s.is_empty()).cloned(),
        if_number: map.get("if_number").filter(|s| !s.is_empty()).cloned(),
        rc_number: map.get("rc_number").filter(|s| !s.is_empty()).cloned(),
        patente: map.get("patente").filter(|s| !s.is_empty()).cloned(),
        default_tva: map.get("default_tva").and_then(|v| v.parse::<f64>().ok()).unwrap_or(20.0),
        receipt_footer: map.get("receipt_footer").filter(|s| !s.is_empty()).cloned(),
        currency: map.get("currency").cloned().unwrap_or_else(|| "MAD".to_string()),
        printer_name: map.get("printer_name").filter(|s| !s.is_empty()).cloned(),
        fidelite_actif: map.get("fidelite_actif").cloned(),
        fidelite_dh_pour_1_point: map.get("fidelite_dh_pour_1_point").cloned(),
        fidelite_valeur_1_point: map.get("fidelite_valeur_1_point").cloned(),
        business_type: map.get("business_type").cloned(),
        idle_timeout: map.get("idle_timeout").cloned(),
        logo_base64: map.get("logo_base64").filter(|s| !s.is_empty()).cloned(),
        receipt_header: map.get("receipt_header").filter(|s| !s.is_empty()).cloned(),
        doc_primary_color: map.get("doc_primary_color").filter(|s| !s.is_empty()).cloned(),
    })
}

#[tauri::command]
pub fn update_settings(db: State<DbState>, shop_name: String, shop_address: Option<String>, shop_phone: Option<String>,
    shop_email: Option<String>, ice: Option<String>, if_number: Option<String>, rc_number: Option<String>,
    patente: Option<String>, default_tva: f64,
    receipt_footer: Option<String>, currency: String,
    printer_name: Option<String>, business_type: Option<String>,
    fidelite_actif: Option<String>, fidelite_dh_pour_1_point: Option<String>,
    fidelite_valeur_1_point: Option<String>,
    idle_timeout: Option<String>,
    logo_base64: Option<String>,
    receipt_header: Option<String>,
    doc_primary_color: Option<String>) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let pairs: Vec<(&str, String)> = vec![
        ("shop_name", shop_name),
        ("shop_address", shop_address.unwrap_or_default()),
        ("shop_phone", shop_phone.unwrap_or_default()),
        ("shop_email", shop_email.unwrap_or_default()),
        ("ice", ice.unwrap_or_default()),
        ("if_number", if_number.unwrap_or_default()),
        ("rc_number", rc_number.unwrap_or_default()),
        ("patente", patente.unwrap_or_default()),
        ("default_tva", default_tva.to_string()),
        ("receipt_footer", receipt_footer.unwrap_or_default()),
        ("currency", currency),
        ("printer_name", printer_name.unwrap_or_else(|| "POS-80".to_string())),
        ("business_type", business_type.unwrap_or_else(|| "standard".to_string())),
        ("fidelite_actif", fidelite_actif.unwrap_or_else(|| "true".to_string())),
        ("fidelite_dh_pour_1_point", fidelite_dh_pour_1_point.unwrap_or_else(|| "100".to_string())),
        ("fidelite_valeur_1_point", fidelite_valeur_1_point.unwrap_or_else(|| "1".to_string())),
        ("idle_timeout", idle_timeout.unwrap_or_else(|| "300".to_string())),
        ("logo_base64", logo_base64.unwrap_or_default()),
        ("receipt_header", receipt_header.unwrap_or_default()),
        ("doc_primary_color", doc_primary_color.unwrap_or_default()),
    ];
    for (key, value) in pairs {
        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            params![key, value],
        ).map_err(|e| e.to_string())?;
    }
    log_audit(&conn, None, "modifier_parametres", "Mise à jour des paramètres boutique", None, None);
    Ok(())
}

// ─── Print Receipt ───

#[tauri::command]
pub fn print_receipt(data: String) -> Result<(), String> {
    let path = std::env::temp_dir().join("ticket_impression.html");
    std::fs::write(&path, &data).map_err(|e| format!("Erreur écriture ticket: {}", e))?;

    if cfg!(target_os = "windows") {
        let ps = format!(
            "Start-Process -FilePath '{}' -WindowStyle Normal -Wait",
            path.to_string_lossy().replace("'", "''")
        );
        let _ = std::process::Command::new("powershell")
            .args(["-Command", &ps])
            .output();
    } else if cfg!(target_os = "macos") {
        let _ = std::process::Command::new("open")
            .arg(path.to_string_lossy().as_ref())
            .output();
    } else {
        let _ = std::process::Command::new("xdg-open")
            .arg(path.to_string_lossy().as_ref())
            .output();
    }
    Ok(())
}

#[tauri::command]
pub fn save_document_pdf(base64_data: String, filename: String) -> Result<String, String> {
    let bytes = general_purpose::STANDARD.decode(&base64_data)
        .map_err(|e| format!("Erreur de décodage base64: {}", e))?;

    let safe_name: String = filename
        .chars()
        .filter(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | '.'))
        .collect();
    let safe_name = if safe_name.is_empty() { "document.pdf".to_string() } else { safe_name };
    let safe_name = if safe_name.to_lowercase().ends_with(".pdf") { safe_name } else { format!("{}.pdf", safe_name) };

    let docs_dir = std::env::current_dir()
        .map_err(|e| e.to_string())?
        .join("documents");
    std::fs::create_dir_all(&docs_dir).map_err(|e| e.to_string())?;
    let doc_path = docs_dir.join(safe_name);
    std::fs::write(&doc_path, &bytes).map_err(|e| format!("Erreur d'écriture du PDF: {}", e))?;
    Ok(doc_path.to_string_lossy().to_string())
}

// ─── Backup / Export / Import ───

#[tauri::command]
pub fn backup_database(db: State<DbState>) -> Result<String, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let _db_path = conn.path().ok_or("Base de données non fichier")?.to_string();
    let backup_dir = std::env::current_dir()
        .map_err(|e| e.to_string())?
        .join("backups");
    std::fs::create_dir_all(&backup_dir).map_err(|e| e.to_string())?;
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let backup_path = backup_dir.join(format!("supercaisse_{}.db", timestamp));
    let mut dst = Connection::open(&backup_path).map_err(|e| e.to_string())?;
    let backup = Backup::new(&*conn, &mut dst).map_err(|e| e.to_string())?;
    backup.run_to_completion(5, Duration::from_millis(250), None).map_err(|e| e.to_string())?;
    Ok(backup_path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn export_database(db: State<DbState>) -> Result<String, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let _db_path = conn.path().ok_or("Base de données non fichier")?.to_string();
    let export_dir = std::env::current_dir()
        .map_err(|e| e.to_string())?
        .join("exports");
    std::fs::create_dir_all(&export_dir).map_err(|e| e.to_string())?;
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let export_path = export_dir.join(format!("supercaisse_export_{}.db", timestamp));
    let mut dst = Connection::open(&export_path).map_err(|e| e.to_string())?;
    let backup = Backup::new(&*conn, &mut dst).map_err(|e| e.to_string())?;
    backup.run_to_completion(5, Duration::from_millis(250), None).map_err(|e| e.to_string())?;
    Ok(export_path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn import_database(db: State<DbState>, path: String) -> Result<(), String> {
    let mut conn = db.conn.lock().map_err(|e| e.to_string())?;
    let src = Connection::open(&path).map_err(|e| e.to_string())?;
    let backup = Backup::new(&src, &mut *conn).map_err(|e| e.to_string())?;
    backup.run_to_completion(5, Duration::from_millis(250), None).map_err(|e| e.to_string())?;
    Ok(())
}

// ─── Fidélité ───

#[tauri::command]
pub fn get_mouvements_fidelite(db: State<DbState>, client_id: i64) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT mf.id, mf.client_id, mf.vente_id, mf.points, mf.mtype, mf.date, v.numero_facture
         FROM mouvements_fidelite mf
         LEFT JOIN ventes v ON v.id = mf.vente_id
         WHERE mf.client_id = ?1
         ORDER BY mf.date DESC
         LIMIT 100"
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map(params![client_id], |row| {
        Ok(serde_json::json!({
            "id": row.get::<_, i64>(0)?,
            "client_id": row.get::<_, i64>(1)?,
            "vente_id": row.get::<_, Option<i64>>(2)?,
            "points": row.get::<_, f64>(3)?,
            "mtype": row.get::<_, String>(4)?,
            "date": row.get::<_, String>(5)?,
            "numero_facture": row.get::<_, Option<String>>(6)?
        }))
    }).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

// ─── Rapport X (intermédiaire) ───

#[tauri::command]
pub fn get_rapport_x(db: State<DbState>, session_id: i64) -> Result<serde_json::Value, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let (date_ouverture, fond_initial): (String, f64) = conn.query_row(
        "SELECT date_ouverture, fond_initial FROM sessions_caisse WHERE id = ?1",
        params![session_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    ).map_err(|_| "Session introuvable".to_string())?;

    let nb_ventes: i64 = conn.query_row(
        "SELECT COUNT(*) FROM ventes WHERE session_id = ?1 AND statut != 'annulee'",
        params![session_id], |r| r.get(0),
    ).unwrap_or(0);

    let ca_total: f64 = conn.query_row(
        "SELECT COALESCE(SUM(montant_total - montant_remise), 0) FROM ventes WHERE session_id = ?1 AND statut != 'annulee'",
        params![session_id], |r| r.get(0),
    ).unwrap_or(0.0);

    let total_remises: f64 = conn.query_row(
        "SELECT COALESCE(SUM(montant_remise), 0) FROM ventes WHERE session_id = ?1 AND statut != 'annulee'",
        params![session_id], |r| r.get(0),
    ).unwrap_or(0.0);

    let nb_annulations: i64 = conn.query_row(
        "SELECT COUNT(*) FROM ventes WHERE session_id = ?1 AND statut = 'annulee'",
        params![session_id], |r| r.get(0),
    ).unwrap_or(0);

    let nb_articles_vendus: f64 = conn.query_row(
        "SELECT COALESCE(SUM(va.quantite), 0) FROM vente_articles va JOIN ventes v ON v.id = va.vente_id WHERE v.session_id = ?1 AND v.statut != 'annulee'",
        params![session_id], |r| r.get(0),
    ).unwrap_or(0.0);

    let mut stmt = conn.prepare(
        "SELECT mode_paiement, COALESCE(SUM(montant_total - montant_remise), 0) FROM ventes WHERE session_id = ?1 AND statut != 'annulee' GROUP BY mode_paiement"
    ).map_err(|e| e.to_string())?;
    let par_mode = stmt.query_map(params![session_id], |r| {
        Ok(serde_json::json!({
            "mode": r.get::<_, String>(0)?,
            "total": r.get::<_, f64>(1)?
        }))
    }).map_err(|e| e.to_string())?.filter_map(Result::ok).collect::<Vec<_>>();

    Ok(serde_json::json!({
        "session_id": session_id,
        "date_ouverture": date_ouverture,
        "fond_initial": fond_initial,
        "nb_ventes": nb_ventes,
        "ca_total": ca_total,
        "total_remises": total_remises,
        "nb_annulations": nb_annulations,
        "nb_articles_vendus": nb_articles_vendus,
        "par_mode": par_mode,
    }))
}

// ─── Relevé client ───

#[tauri::command]
pub fn get_releve_client(db: State<DbState>, client_id: i64) -> Result<serde_json::Value, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let (nom, credit_actuel, credit_plafond): (String, f64, f64) = conn.query_row(
        "SELECT nom, COALESCE(credit_actuel, 0), COALESCE(credit_plafond, 0) FROM clients WHERE id = ?1",
        params![client_id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    ).map_err(|_| "Client introuvable".to_string())?;

    let mut stmt = conn.prepare(
        "SELECT v.id, v.date, v.numero_facture, v.montant_total, v.montant_remise, v.mode_paiement, v.statut, v.dtype
         FROM ventes v WHERE v.client_id = ?1
         ORDER BY v.date DESC LIMIT 200"
    ).map_err(|e| e.to_string())?;
    let ventes = stmt.query_map(params![client_id], |r| {
        Ok(serde_json::json!({
            "id": r.get::<_, i64>(0)?,
            "date": r.get::<_, String>(1)?,
            "numero_facture": r.get::<_, Option<String>>(2)?,
            "montant_total": r.get::<_, f64>(3)?,
            "montant_remise": r.get::<_, f64>(4)?,
            "mode_paiement": r.get::<_, String>(5)?,
            "statut": r.get::<_, String>(6)?,
            "dtype": r.get::<_, Option<String>>(7)?
        }))
    }).map_err(|e| e.to_string())?.filter_map(Result::ok).collect::<Vec<_>>();

    let mut stmt2 = conn.prepare(
        "SELECT p.id, p.date, p.montant, p.type, p.reference
         FROM paiements p WHERE p.client_id = ?1
         ORDER BY p.date DESC LIMIT 200"
    ).map_err(|e| e.to_string())?;
    let paiements = stmt2.query_map(params![client_id], |r| {
        Ok(serde_json::json!({
            "id": r.get::<_, i64>(0)?,
            "date": r.get::<_, String>(1)?,
            "montant": r.get::<_, f64>(2)?,
            "type": r.get::<_, String>(3)?,
            "reference": r.get::<_, Option<String>>(4)?
        }))
    }).map_err(|e| e.to_string())?.filter_map(Result::ok).collect::<Vec<_>>();

    Ok(serde_json::json!({
        "client_id": client_id,
        "nom": nom,
        "credit_actuel": credit_actuel,
        "credit_plafond": credit_plafond,
        "ventes": ventes,
        "paiements": paiements,
    }))
}

// ─── Journal d'audit ───

#[tauri::command]
pub fn get_audit_log(db: State<DbState>, debut: Option<String>, fin: Option<String>, action_filter: Option<String>) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
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

// ─── PIN rapide (changement caissier) ───

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

// ─── Rapports détaillés ───

#[tauri::command]
pub fn get_rapport_detaille(db: State<DbState>, debut: Option<String>, fin: Option<String>) -> Result<serde_json::Value, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let date_filter = |col: &str| -> (String, Vec<String>) {
        let mut clause = String::new();
        let mut params = Vec::new();
        if let Some(d) = &debut { if !d.is_empty() { clause.push_str(&format!(" AND {} >= ?", col)); params.push(d.clone()); } }
        if let Some(f) = &fin { if !f.is_empty() { clause.push_str(&format!(" AND {} <= ?", col)); params.push(format!("{} 23:59:59", f)); } }
        (clause, params)
    };

    let (wc, wp) = date_filter("v.date");
    let base_sql = format!(
        "SELECT COALESCE(SUM(v.montant_total - v.montant_remise), 0),
                COALESCE(SUM(v.montant_remise), 0),
                COUNT(*)
         FROM ventes v WHERE v.statut != 'annulee' {}", wc
    );
    let params_ref: Vec<&dyn rusqlite::types::ToSql> = wp.iter().map(|s| s as &dyn rusqlite::types::ToSql).collect();
    let (ca_total, total_remises, nb_ventes): (f64, f64, i64) = conn.query_row(
        &base_sql, params_ref.as_slice(),
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    ).unwrap_or((0.0, 0.0, 0));

    let (wc2, wp2) = date_filter("v.date");
    let marge_sql = format!(
        "SELECT COALESCE(SUM(va.quantite * (va.prix_unitaire - a.prix_achat)), 0)
         FROM vente_articles va
         JOIN ventes v ON v.id = va.vente_id
         JOIN articles a ON a.id = va.article_id
         WHERE v.statut != 'annulee' {}", wc2
    );
    let params_ref2: Vec<&dyn rusqlite::types::ToSql> = wp2.iter().map(|s| s as &dyn rusqlite::types::ToSql).collect();
    let marge_brute: f64 = conn.query_row(&marge_sql, params_ref2.as_slice(), |r| r.get(0)).unwrap_or(0.0);

    let (wc3, wp3) = date_filter("v.date");
    let tva_sql = format!(
        "SELECT COALESCE(SUM(va.quantite * va.prix_unitaire * va.tva / 100.0), 0)
         FROM vente_articles va
         JOIN ventes v ON v.id = va.vente_id
         WHERE v.statut != 'annulee' {}", wc3
    );
    let params_ref3: Vec<&dyn rusqlite::types::ToSql> = wp3.iter().map(|s| s as &dyn rusqlite::types::ToSql).collect();
    let tva_collectee: f64 = conn.query_row(&tva_sql, params_ref3.as_slice(), |r| r.get(0)).unwrap_or(0.0);

    let (wc4, wp4) = date_filter("v.date");
    let top_sql = format!(
        "SELECT a.designation, SUM(va.quantite) as qty, SUM(va.total_ligne) as total
         FROM vente_articles va
         JOIN ventes v ON v.id = va.vente_id
         JOIN articles a ON a.id = va.article_id
         WHERE v.statut != 'annulee' {}
         GROUP BY va.article_id ORDER BY qty DESC LIMIT 10", wc4
    );
    let params_ref4: Vec<&dyn rusqlite::types::ToSql> = wp4.iter().map(|s| s as &dyn rusqlite::types::ToSql).collect();
    let mut stmt = conn.prepare(&top_sql).map_err(|e| e.to_string())?;
    let top_articles = stmt.query_map(params_ref4.as_slice(), |r| {
        Ok(serde_json::json!({
            "designation": r.get::<_, String>(0)?,
            "quantite": r.get::<_, f64>(1)?,
            "total": r.get::<_, f64>(2)?
        }))
    }).map_err(|e| e.to_string())?.filter_map(Result::ok).collect::<Vec<_>>();

    let (wc5, wp5) = date_filter("v.date");
    let rotation_sql = format!(
        "SELECT a.id, a.designation, a.stock, COALESCE(SUM(CASE WHEN v.id IS NOT NULL THEN va.quantite ELSE 0 END), 0) as vendu
         FROM articles a
         LEFT JOIN vente_articles va ON va.article_id = a.id
         LEFT JOIN ventes v ON v.id = va.vente_id AND v.statut != 'annulee' AND v.dtype IN ('facture', 'bl') {}
         WHERE a.actif = 1
         GROUP BY a.id ORDER BY vendu DESC LIMIT 20", wc5
    );
    let params_ref5: Vec<&dyn rusqlite::types::ToSql> = wp5.iter().map(|s| s as &dyn rusqlite::types::ToSql).collect();
    let mut stmt2 = conn.prepare(&rotation_sql).map_err(|e| e.to_string())?;
    let rotation_stock = stmt2.query_map(params_ref5.as_slice(), |r| {
        Ok(serde_json::json!({
            "id": r.get::<_, i64>(0)?,
            "designation": r.get::<_, String>(1)?,
            "stock": r.get::<_, f64>(2)?,
            "vendu": r.get::<_, f64>(3)?
        }))
    }).map_err(|e| e.to_string())?.filter_map(Result::ok).collect::<Vec<_>>();

    let (wc6, wp6) = date_filter("v.date");
    let daily_sql = format!(
        "SELECT date(v.date) as jour, COALESCE(SUM(v.montant_total - v.montant_remise), 0) as ca, COUNT(*) as nb
         FROM ventes v WHERE v.statut != 'annulee' {}
         GROUP BY jour ORDER BY jour", wc6
    );
    let params_ref6: Vec<&dyn rusqlite::types::ToSql> = wp6.iter().map(|s| s as &dyn rusqlite::types::ToSql).collect();
    let mut stmt3 = conn.prepare(&daily_sql).map_err(|e| e.to_string())?;
    let ventes_par_jour = stmt3.query_map(params_ref6.as_slice(), |r| {
        Ok(serde_json::json!({
            "jour": r.get::<_, String>(0)?,
            "ca": r.get::<_, f64>(1)?,
            "nb": r.get::<_, i64>(2)?
        }))
    }).map_err(|e| e.to_string())?.filter_map(Result::ok).collect::<Vec<_>>();

    let (wc7, wp7) = date_filter("v.date");
    let mode_sql = format!(
        "SELECT mode_paiement, COALESCE(SUM(montant_total - montant_remise), 0) as total, COUNT(*) as nb
         FROM ventes v WHERE v.statut != 'annulee' {}
         GROUP BY mode_paiement", wc7
    );
    let params_ref7: Vec<&dyn rusqlite::types::ToSql> = wp7.iter().map(|s| s as &dyn rusqlite::types::ToSql).collect();
    let mut stmt4 = conn.prepare(&mode_sql).map_err(|e| e.to_string())?;
    let par_mode = stmt4.query_map(params_ref7.as_slice(), |r| {
        Ok(serde_json::json!({
            "mode": r.get::<_, String>(0)?,
            "total": r.get::<_, f64>(1)?,
            "count": r.get::<_, i64>(2)?
        }))
    }).map_err(|e| e.to_string())?.filter_map(Result::ok).collect::<Vec<_>>();

    Ok(serde_json::json!({
        "ca_total": ca_total,
        "total_remises": total_remises,
        "nb_ventes": nb_ventes,
        "marge_brute": marge_brute,
        "tva_collectee": tva_collectee,
        "top_articles": top_articles,
        "rotation_stock": rotation_stock,
        "ventes_par_jour": ventes_par_jour,
        "par_mode": par_mode,
    }))
}

// ─── Inventaire physique ───

#[tauri::command]
pub fn create_inventaire(db: State<DbState>, magasin_id: i64, utilisateur_id: i64) -> Result<serde_json::Value, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
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
pub fn get_inventaire(db: State<DbState>, inventaire_id: i64) -> Result<serde_json::Value, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

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
pub fn get_inventaires(db: State<DbState>) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
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
pub fn update_inventaire_ligne(db: State<DbState>, ligne_id: i64, stock_compte: f64) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
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
pub fn valider_inventaire(db: State<DbState>, inventaire_id: i64, utilisateur_id: i64) -> Result<(), String> {
    let mut conn = db.conn.lock().map_err(|e| e.to_string())?;

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

// ─── Permissions ───

#[tauri::command]
pub fn get_permissions(db: State<DbState>, role: String) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT role, module, action, allowed FROM permissions WHERE role = ?1 ORDER BY module, action"
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map(params![role], |row| {
        Ok(serde_json::json!({
            "role": row.get::<_, String>(0)?,
            "module": row.get::<_, String>(1)?,
            "action": row.get::<_, String>(2)?,
            "allowed": row.get::<_, i32>(3)? != 0,
        }))
    }).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_permission(db: State<DbState>, role: String, module: String, action: String, allowed: bool) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO permissions (role, module, action, allowed) VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(role, module, action) DO UPDATE SET allowed = ?4",
        params![role, module, action, allowed as i32],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

// ─── Multi-caisse ───

#[tauri::command]
pub fn get_caisses(db: State<DbState>) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT c.id, c.nom, c.utilisateur_id, c.statut, c.ouverture_date, c.fermeture_date,
                c.fond_initial, c.recettes_especes, c.recettes_cb, c.recettes_cheque,
                c.recettes_virement, c.depenses, c.ecart, c.note, u.nom AS utilisateur_nom
         FROM caisses c
         LEFT JOIN utilisateurs u ON c.utilisateur_id = u.id
         ORDER BY c.id DESC"
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |row| {
        Ok(serde_json::json!({
            "id": row.get::<_, i64>(0)?,
            "nom": row.get::<_, String>(1)?,
            "utilisateur_id": row.get::<_, Option<i64>>(2)?,
            "statut": row.get::<_, String>(3)?,
            "ouverture_date": row.get::<_, Option<String>>(4)?,
            "fermeture_date": row.get::<_, Option<String>>(5)?,
            "fond_initial": row.get::<_, f64>(6)?,
            "recettes_especes": row.get::<_, f64>(7)?,
            "recettes_cb": row.get::<_, f64>(8)?,
            "recettes_cheque": row.get::<_, f64>(9)?,
            "recettes_virement": row.get::<_, f64>(10)?,
            "depenses": row.get::<_, f64>(11)?,
            "ecart": row.get::<_, f64>(12)?,
            "note": row.get::<_, Option<String>>(13)?,
            "utilisateur_nom": row.get::<_, Option<String>>(14)?,
        }))
    }).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_caisse(db: State<DbState>, nom: String, fond_initial: f64, utilisateur_id: Option<i64>) -> Result<i64, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    conn.execute(
        "INSERT INTO caisses (nom, utilisateur_id, statut, ouverture_date, fond_initial) VALUES (?1, ?2, 'ouverte', ?3, ?4)",
        params![nom, utilisateur_id, now, fond_initial],
    ).map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

#[tauri::command]
pub fn close_caisse(db: State<DbState>, id: i64, note: Option<String>) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let ouverture_date: String = conn.query_row(
        "SELECT ouverture_date FROM caisses WHERE id = ?1 AND statut = 'ouverte'",
        params![id],
        |row| row.get(0),
    ).map_err(|_| "Caisse introuvable ou déjà fermée".to_string())?;

    let recettes_especes: f64 = conn.query_row(
        "SELECT COALESCE(SUM(montant_total - montant_remise), 0) FROM ventes WHERE mode_paiement = 'especes' AND statut = 'validee' AND date >= ?1",
        params![ouverture_date],
        |row| row.get(0),
    ).unwrap_or(0.0);

    let recettes_cb: f64 = conn.query_row(
        "SELECT COALESCE(SUM(montant_total - montant_remise), 0) FROM ventes WHERE mode_paiement = 'cb' AND statut = 'validee' AND date >= ?1",
        params![ouverture_date],
        |row| row.get(0),
    ).unwrap_or(0.0);

    let recettes_cheque: f64 = conn.query_row(
        "SELECT COALESCE(SUM(montant_total - montant_remise), 0) FROM ventes WHERE mode_paiement = 'cheque' AND statut = 'validee' AND date >= ?1",
        params![ouverture_date],
        |row| row.get(0),
    ).unwrap_or(0.0);

    let recettes_virement: f64 = conn.query_row(
        "SELECT COALESCE(SUM(montant_total - montant_remise), 0) FROM ventes WHERE mode_paiement = 'virement' AND statut = 'validee' AND date >= ?1",
        params![ouverture_date],
        |row| row.get(0),
    ).unwrap_or(0.0);

    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    conn.execute(
        "UPDATE caisses SET statut = 'fermee', fermeture_date = ?1, recettes_especes = ?2, recettes_cb = ?3, recettes_cheque = ?4, recettes_virement = ?5, note = ?6 WHERE id = ?7",
        params![now, recettes_especes, recettes_cb, recettes_cheque, recettes_virement, note, id],
    ).map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn get_tresorerie(db: State<DbState>) -> Result<serde_json::Value, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let today_start = chrono::Local::now().format("%Y-%m-%d 00:00:00").to_string();

    let week_start = {
        use chrono::Datelike;
        let now = chrono::Local::now();
        let weekday = now.weekday().num_days_from_monday();
        let monday = now - chrono::Duration::days(weekday as i64);
        monday.format("%Y-%m-%d 00:00:00").to_string()
    };

    let month_start = chrono::Local::now().format("%Y-%m-01 00:00:00").to_string();

    let fetch_recettes = |since: &str| -> Result<serde_json::Value, String> {
        let especes: f64 = conn.query_row(
            "SELECT COALESCE(SUM(montant_total - montant_remise), 0) FROM ventes WHERE mode_paiement = 'especes' AND statut = 'validee' AND date >= ?1",
            params![since], |r| r.get(0),
        ).unwrap_or(0.0);
        let cb: f64 = conn.query_row(
            "SELECT COALESCE(SUM(montant_total - montant_remise), 0) FROM ventes WHERE mode_paiement = 'cb' AND statut = 'validee' AND date >= ?1",
            params![since], |r| r.get(0),
        ).unwrap_or(0.0);
        let cheque: f64 = conn.query_row(
            "SELECT COALESCE(SUM(montant_total - montant_remise), 0) FROM ventes WHERE mode_paiement = 'cheque' AND statut = 'validee' AND date >= ?1",
            params![since], |r| r.get(0),
        ).unwrap_or(0.0);
        let virement: f64 = conn.query_row(
            "SELECT COALESCE(SUM(montant_total - montant_remise), 0) FROM ventes WHERE mode_paiement = 'virement' AND statut = 'validee' AND date >= ?1",
            params![since], |r| r.get(0),
        ).unwrap_or(0.0);
        Ok(serde_json::json!({
            "especes": especes,
            "cb": cb,
            "cheque": cheque,
            "virement": virement,
            "total": especes + cb + cheque + virement,
        }))
    };

    let jour = fetch_recettes(&today_start)?;
    let semaine = fetch_recettes(&week_start)?;
    let mois = fetch_recettes(&month_start)?;

    Ok(serde_json::json!({
        "jour": jour,
        "semaine": semaine,
        "mois": mois,
    }))
}

// ─── Comparaison prix fournisseurs ───

#[tauri::command]
pub fn compare_fournisseur_prices(db: State<DbState>, article_id: Option<i64>) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let sql = "
        SELECT
            art.id AS article_id,
            art.designation,
            art.code_barre,
            f.id AS fournisseur_id,
            f.nom AS fournisseur_nom,
            aa.prix_unitaire,
            a.date
        FROM achat_articles aa
        JOIN achats a ON aa.achat_id = a.id
        JOIN fournisseurs f ON a.fournisseur_id = f.id
        JOIN articles art ON aa.article_id = art.id
        WHERE a.fournisseur_id IS NOT NULL
        ORDER BY art.designation, f.nom, a.date DESC
    ";

    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, f64>(5)?,
            row.get::<_, String>(6)?,
        ))
    }).map_err(|e| e.to_string())?;

    let all_rows: Vec<_> = rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;

    let mut articles_map: std::collections::BTreeMap<i64, serde_json::Value> = std::collections::BTreeMap::new();

    for (aid, designation, code_barre, fid, fournisseur_nom, prix, date) in all_rows {
        if let Some(filter_id) = article_id {
            if aid != filter_id {
                continue;
            }
        }

        let entry = articles_map.entry(aid).or_insert_with(|| {
            serde_json::json!({
                "article_id": aid,
                "designation": designation,
                "code_barre": code_barre,
                "fournisseurs": []
            })
        });

        let empty = vec![];
        let fournisseurs = entry["fournisseurs"].as_array().unwrap_or(&empty);
        let already_has = fournisseurs.iter().any(|f| f["fournisseur_id"].as_i64() == Some(fid));

        if !already_has {
            if let Some(arr) = entry["fournisseurs"].as_array_mut() {
                arr.push(serde_json::json!({
                    "fournisseur_id": fid,
                    "fournisseur_nom": fournisseur_nom,
                    "prix_unitaire": prix,
                    "date": date
                }));
            }
        }
    }

    Ok(articles_map.into_values().collect())
}
