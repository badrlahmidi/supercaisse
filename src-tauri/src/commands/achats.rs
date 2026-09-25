use crate::db::*;
use crate::session::{autoriser, Acces, AuthState};
use rusqlite::params;
use tauri::State;

use super::{adjust_article_stock, default_magasin_id};

#[tauri::command]
pub fn create_achat(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    fournisseur_id: Option<i64>,
    reference: Option<String>,
    articles: Vec<serde_json::Value>,
    statut_livraison: Option<String>,
    statut_paiement: Option<String>,
) -> Result<i64, String> {
    let mut conn = db.conn.lock().map_err(|e| e.to_string())?;
    let _me = autoriser(&auth, &conn, &token, Acces::Module("achats", "creer"))?;
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
        let article_id = a["article_id"]
            .as_i64()
            .ok_or("article_id manquant ou invalide dans la ligne")?;
        let qte = a["quantite"].as_f64().unwrap_or(0.0);
        let pu = a["prix_unitaire"].as_f64().unwrap_or(0.0);
        let total_ligne = qte * pu;
        tx.execute(
            "INSERT INTO achat_articles (achat_id, article_id, quantite, prix_unitaire, total_ligne) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![achat_id, article_id, qte, pu, total_ligne],
        ).map_err(|e| e.to_string())?;

        if sl == "recu" {
            tx.execute(
                "UPDATE articles SET prix_achat = ?1 WHERE id = ?2",
                params![pu, article_id],
            )
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
pub fn get_achats(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let _me = autoriser(&auth, &conn, &token, Acces::Module("achats", "voir"))?;
    let mut stmt = conn.prepare(
        "SELECT a.id, a.date, a.fournisseur_id, a.reference, a.montant_total, a.statut, f.nom as fournisseur_nom, a.statut_livraison, a.statut_paiement
         FROM achats a LEFT JOIN fournisseurs f ON a.fournisseur_id = f.id
         ORDER BY a.date DESC LIMIT 200"
    ).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
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
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_achat_status(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    achat_id: i64,
    statut_livraison: String,
    statut_paiement: String,
) -> Result<(), String> {
    let mut conn = db.conn.lock().map_err(|e| e.to_string())?;
    let _me = autoriser(&auth, &conn, &token, Acces::Module("achats", "modifier"))?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    let old_sl: String = tx
        .query_row(
            "SELECT statut_livraison FROM achats WHERE id = ?1",
            params![achat_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    tx.execute(
        "UPDATE achats SET statut_livraison = ?1, statut_paiement = ?2 WHERE id = ?3",
        params![statut_livraison, statut_paiement, achat_id],
    )
    .map_err(|e| e.to_string())?;

    if old_sl != "recu" && statut_livraison == "recu" {
        let magasin_id = default_magasin_id(&tx)?;
        let mut stmt = tx.prepare("SELECT article_id, quantite, prix_unitaire FROM achat_articles WHERE achat_id = ?1").map_err(|e| e.to_string())?;
        let lignes = stmt
            .query_map(params![achat_id], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, f64>(1)?,
                    row.get::<_, f64>(2)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        let lignes: Vec<(i64, f64, f64)> = lignes
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| e.to_string())?;
        drop(stmt);

        for (article_id, qte, pu) in lignes {
            tx.execute(
                "UPDATE articles SET prix_achat = ?1 WHERE id = ?2",
                params![pu, article_id],
            )
            .map_err(|e| e.to_string())?;
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

#[tauri::command]
pub fn compare_fournisseur_prices(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    article_id: Option<i64>,
) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let _me = autoriser(&auth, &conn, &token, Acces::Module("achats", "voir"))?;

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
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, f64>(5)?,
                row.get::<_, String>(6)?,
            ))
        })
        .map_err(|e| e.to_string())?;

    let all_rows: Vec<_> = rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let mut articles_map: std::collections::BTreeMap<i64, serde_json::Value> =
        std::collections::BTreeMap::new();

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
        let already_has = fournisseurs
            .iter()
            .any(|f| f["fournisseur_id"].as_i64() == Some(fid));

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
