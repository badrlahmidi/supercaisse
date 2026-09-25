use crate::db::*;
use rusqlite::{params, Connection};
use tauri::State;

use super::{default_magasin_id, adjust_article_stock, log_audit, document_prefixe, next_numero_document, annee_courante};

fn get_composants(tx: &Connection, article_id: i64) -> Result<Vec<(i64, f64)>, String> {
    let mut stmt = tx.prepare("SELECT composant_id, quantite FROM article_composants WHERE article_id = ?1")
        .map_err(|e| e.to_string())?;
    let rows = stmt.query_map(params![article_id], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, f64>(1)?))
    }).map_err(|e| e.to_string())?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_vente(db: State<DbState>, client_id: Option<i64>, caissier_id: Option<i64>,
    articles: Vec<serde_json::Value>, montant_remise: f64, mode_paiement: String,
    splits: Option<Vec<serde_json::Value>>, dtype: Option<String>,
    points_utilises: Option<f64>, points_gagnes: Option<f64>,
    magasin_id: Option<i64>
) -> Result<serde_json::Value, String> {
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

    document_prefixe(&document_type)?;

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

    let numero_facture = next_numero_document(&tx, &document_type, annee_courante())?;

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

    if let Some(ref split_list) = splits {
        if split_list.len() > 1 {
            for s in split_list {
                let mode = s["mode"].as_str().unwrap_or("inconnu").to_string();
                let montant = s["montant"].as_f64().unwrap_or(0.0);
                let desc = format!("Split vente #{}: {}", vente_id, mode);
                tx.execute(
                    "INSERT INTO journal_caisse (utilisateur_id, jtype, montant, description) VALUES (?1, 'encaissement', ?2, ?3)",
                    params![caissier_id, montant, desc],
                ).map_err(|e| e.to_string())?;
            }
        }
    }

    log_audit(&tx, caissier_id, "creer_vente",
        &format!("Vente #{} - {} DH ({}) - {}", vente_id, montant_total, document_type, mode_paiement),
        Some("vente"), Some(vente_id));

    tx.commit().map_err(|e| e.to_string())?;

    Ok(serde_json::json!({ "id": vente_id, "numero_facture": numero_facture }))
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

    let numero_facture = next_numero_document(&tx, &target_type, annee_courante())?;

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

    let net_amount = (montant_total - montant_remise).abs();
    if mode_paiement == "credit" && (target_type == "facture" || target_type == "bl") {
        if let Some(cid) = client_id {
            tx.execute(
                "UPDATE clients SET credit_actuel = credit_actuel + ?1 WHERE id = ?2",
                params![net_amount, cid],
            ).map_err(|e| e.to_string())?;
        }
    }
    if target_type == "avoir" {
        if let Some(cid) = client_id {
            if mode_paiement == "credit" {
                tx.execute(
                    "UPDATE clients SET credit_actuel = credit_actuel - ?1 WHERE id = ?2",
                    params![net_amount, cid],
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
