use crate::db::*;
use crate::session::{autoriser, Acces, AuthState};
use rusqlite::{params, Connection, OptionalExtension};
use tauri::State;

use super::calcul::{
    calculer_ligne, round2, totaliser, valider_pourcentage, LigneCalculee, TOLERANCE_MONTANT,
};
use super::mouvements::{inverser_lots, mouvement_ligne, proprietaire_lots, LigneStock, Sens};
use super::{
    annee_courante, default_magasin_id, document_prefixe, log_audit, next_numero_document,
};

const MODES_PAIEMENT: [&str; 7] = [
    "especes", "carte", "cb", "cheque", "virement", "credit", "fidelite",
];

pub(crate) fn normaliser_paiements(
    mode_paiement: &str,
    splits: Option<&[serde_json::Value]>,
    montant_par_defaut: f64,
) -> Result<Vec<(String, f64)>, String> {
    let paiements: Vec<(String, f64)> = match splits {
        Some(list) if !list.is_empty() => list
            .iter()
            .map(|s| {
                let mode = s["mode"]
                    .as_str()
                    .ok_or("Mode de paiement manquant")?
                    .to_string();
                let montant = s["montant"]
                    .as_f64()
                    .ok_or_else(|| format!("Montant manquant pour le paiement {}", mode))?;
                Ok((mode, montant))
            })
            .collect::<Result<_, String>>()?,
        _ => vec![(mode_paiement.to_string(), montant_par_defaut.max(0.0))],
    };
    for (mode, montant) in &paiements {
        if !MODES_PAIEMENT.contains(&mode.as_str()) {
            return Err(format!("Mode de paiement inconnu : {}", mode));
        }
        if !montant.is_finite() || *montant < 0.0 {
            return Err(format!(
                "Montant invalide pour le paiement {} : {}",
                mode, montant
            ));
        }
    }
    Ok(paiements)
}

struct LigneVente {
    article_id: i64,
    variante_id: Option<i64>,
    quantite: f64,
    prix_unitaire: f64,
    tva: f64,
    remise_ligne: f64,
    note: Option<String>,
    prix_type: String,
    calcul: LigneCalculee,
}

fn preparer_lignes(
    tx: &Connection,
    articles: &[serde_json::Value],
    remise_globale: f64,
) -> Result<Vec<LigneVente>, String> {
    if articles.is_empty() {
        return Err("Le document ne contient aucun article".to_string());
    }
    articles
        .iter()
        .map(|a| {
            let article_id = a["article_id"]
                .as_i64()
                .ok_or("article_id manquant ou invalide dans la ligne")?;
            let quantite = a["quantite"]
                .as_f64()
                .ok_or_else(|| format!("Quantité manquante (article {})", article_id))?;
            if !quantite.is_finite() || quantite <= 0.0 {
                return Err(format!(
                    "Quantité invalide (article {}) : {}",
                    article_id, quantite
                ));
            }
            let remise_ligne =
                valider_pourcentage("Remise ligne", a["remise_ligne"].as_f64().unwrap_or(0.0))?;
            let prix_type = match a["prix_type"].as_str().unwrap_or("public") {
                "grossiste" => "grossiste",
                _ => "public",
            }
            .to_string();
            let (prix_vente, prix_grossiste, tva): (f64, Option<f64>, f64) = tx
                .query_row(
                    "SELECT prix_vente, prix_grossiste, tva FROM articles WHERE id = ?1",
                    params![article_id],
                    |r| {
                        Ok((
                            r.get::<_, Option<f64>>(0)?.unwrap_or(0.0),
                            r.get(1)?,
                            r.get::<_, Option<f64>>(2)?.unwrap_or(0.0),
                        ))
                    },
                )
                .optional()
                .map_err(|e| e.to_string())?
                .ok_or_else(|| format!("Article {} introuvable", article_id))?;
            let prix_unitaire = if prix_type == "grossiste" {
                prix_grossiste.unwrap_or(prix_vente)
            } else {
                prix_vente
            };
            Ok(LigneVente {
                article_id,
                variante_id: a["variante_id"].as_i64(),
                quantite,
                prix_unitaire,
                tva,
                remise_ligne,
                note: a["note"].as_str().map(|s| s.to_string()),
                prix_type,
                calcul: calculer_ligne(quantite, prix_unitaire, tva, remise_ligne, remise_globale),
            })
        })
        .collect()
}

fn lire_setting(tx: &Connection, key: &str) -> Result<Option<String>, String> {
    tx.query_row(
        "SELECT value FROM settings WHERE key = ?1",
        params![key],
        |r| r.get(0),
    )
    .optional()
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_vente(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    client_id: Option<i64>,
    articles: Vec<serde_json::Value>,
    remise_globale_pct: Option<f64>,
    mode_paiement: String,
    splits: Option<Vec<serde_json::Value>>,
    dtype: Option<String>,
    points_utilises: Option<f64>,
    magasin_id: Option<i64>,
) -> Result<serde_json::Value, String> {
    let mut conn = db.conn.lock().map_err(|e| e.to_string())?;
    let me = autoriser(&auth, &conn, &token, Acces::Module("ventes", "creer"))?;
    create_vente_impl(
        &mut conn,
        client_id,
        Some(me.user_id),
        articles,
        remise_globale_pct,
        mode_paiement,
        splits,
        dtype,
        points_utilises,
        magasin_id,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn create_vente_impl(
    conn: &mut Connection,
    client_id: Option<i64>,
    caissier_id: Option<i64>,
    articles: Vec<serde_json::Value>,
    remise_globale_pct: Option<f64>,
    mode_paiement: String,
    splits: Option<Vec<serde_json::Value>>,
    dtype: Option<String>,
    points_utilises: Option<f64>,
    magasin_id: Option<i64>,
) -> Result<serde_json::Value, String> {
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

    let document_type = dtype.unwrap_or_else(|| "facture".to_string());
    document_prefixe(&document_type)?;
    let encaisse = matches!(document_type.as_str(), "facture" | "bl");

    let remise_globale = valider_pourcentage("Remise document", remise_globale_pct.unwrap_or(0.0))?;
    let lignes = preparer_lignes(&tx, &articles, remise_globale)?;
    let calculs: Vec<LigneCalculee> = lignes.iter().map(|l| l.calcul).collect();
    let totaux = totaliser(&calculs);

    let paiements = normaliser_paiements(&mode_paiement, splits.as_deref(), totaux.net_ttc)?;

    let fidelite_actif = lire_setting(&tx, "fidelite_actif")?.as_deref() == Some("true");
    let valeur_point: f64 = lire_setting(&tx, "fidelite_valeur_1_point")?
        .and_then(|v| v.parse().ok())
        .filter(|v: &f64| *v > 0.0)
        .unwrap_or(1.0);
    let dh_pour_1_point: f64 = lire_setting(&tx, "fidelite_dh_pour_1_point")?
        .and_then(|v| v.parse().ok())
        .filter(|v: &f64| *v > 0.0)
        .unwrap_or(100.0);
    let pts_utilises = if encaisse {
        points_utilises.unwrap_or(0.0)
    } else {
        0.0
    };
    let paiement_fidelite: f64 = paiements
        .iter()
        .filter(|(m, _)| m == "fidelite")
        .map(|(_, montant)| montant)
        .sum();

    if encaisse {
        let total_paye: f64 = paiements.iter().map(|(_, montant)| montant).sum();
        if (total_paye - totaux.net_ttc).abs() > TOLERANCE_MONTANT {
            return Err(format!(
                "Montant encaissé ({:.2} DH) différent du net à payer recalculé ({:.2} DH). Rechargez les articles et réessayez.",
                total_paye, totaux.net_ttc
            ));
        }
        if !pts_utilises.is_finite() || pts_utilises < 0.0 || pts_utilises.fract() != 0.0 {
            return Err(format!("Nombre de points invalide : {}", pts_utilises));
        }
        if (paiement_fidelite - round2(pts_utilises * valeur_point)).abs() > TOLERANCE_MONTANT {
            return Err(format!(
                "Paiement fidélité ({:.2} DH) incohérent avec les points utilisés ({} × {:.2} DH)",
                paiement_fidelite, pts_utilises, valeur_point
            ));
        }
        if pts_utilises > 0.0 {
            let cid =
                client_id.ok_or("Un client doit être sélectionné pour utiliser des points")?;
            let solde: f64 = tx
                .query_row(
                    "SELECT COALESCE(points_fidelite, 0) FROM clients WHERE id = ?1",
                    params![cid],
                    |r| r.get(0),
                )
                .map_err(|e| e.to_string())?;
            if pts_utilises > solde {
                return Err(format!(
                    "Points insuffisants : {} demandés, {} disponibles",
                    pts_utilises, solde
                ));
            }
        }
    }

    let credit_demandé: f64 = paiements
        .iter()
        .filter(|(m, _)| m == "credit")
        .map(|(_, montant)| montant)
        .sum();
    if credit_demandé > 0.0 {
        if let Some(cid) = client_id {
            let (actuel, plafond): (f64, Option<f64>) = tx
                .query_row(
                    "SELECT credit_actuel, credit_plafond FROM clients WHERE id = ?1",
                    params![cid],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .map_err(|e| e.to_string())?;

            if let Some(plaf) = plafond {
                if plaf > 0.0 && actuel + credit_demandé > plaf {
                    return Err(format!(
                        "Plafond de crédit dépassé. Crédit actuel: {}, Plafond: {}, Demandé: {}",
                        actuel, plaf, credit_demandé
                    ));
                }
            }

            if encaisse {
                tx.execute(
                    "UPDATE clients SET credit_actuel = credit_actuel + ?1 WHERE id = ?2",
                    params![credit_demandé, cid],
                )
                .map_err(|e| e.to_string())?;
            }
        } else {
            return Err("Un client doit être sélectionné pour payer à crédit.".to_string());
        }
    }

    let pts_gagnes = match client_id {
        Some(_) if encaisse && fidelite_actif => (round2(totaux.net_ttc - paiement_fidelite)
            / dh_pour_1_point)
            .floor()
            .max(0.0),
        _ => 0.0,
    };

    let numero_facture = next_numero_document(&tx, &document_type, annee_courante())?;

    let current_session_id: Option<i64> = if let Some(cid) = caissier_id {
        tx.query_row(
            "SELECT id FROM sessions_caisse WHERE caissier_id = ?1 AND statut = 'ouverte'",
            params![cid],
            |row| row.get(0),
        )
        .ok()
    } else {
        None
    };

    tx.execute(
        "INSERT INTO ventes (client_id, caissier_id, montant_total, montant_remise, montant_ht, montant_tva, mode_paiement, numero_facture, dtype, session_id, points_utilises, points_gagnes, magasin_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        params![client_id, caissier_id, totaux.montant_total, totaux.montant_remise, totaux.montant_ht, totaux.montant_tva, mode_paiement, numero_facture, document_type, current_session_id, pts_utilises, pts_gagnes, magasin_id],
    ).map_err(|e| e.to_string())?;
    let vente_id = tx.last_insert_rowid();

    if let Some(cid) = client_id {
        if pts_utilises > 0.0 {
            tx.execute(
                "UPDATE clients SET points_fidelite = points_fidelite - ?1 WHERE id = ?2",
                params![pts_utilises, cid],
            )
            .map_err(|e| e.to_string())?;
            tx.execute("INSERT INTO mouvements_fidelite (client_id, vente_id, points, mtype) VALUES (?1, ?2, ?3, 'depense')", params![cid, vente_id, pts_utilises]).map_err(|e| e.to_string())?;
        }
        if pts_gagnes > 0.0 {
            tx.execute("UPDATE clients SET points_fidelite = COALESCE(points_fidelite, 0) + ?1 WHERE id = ?2", params![pts_gagnes, cid]).map_err(|e| e.to_string())?;
            tx.execute("INSERT INTO mouvements_fidelite (client_id, vente_id, points, mtype) VALUES (?1, ?2, ?3, 'gain')", params![cid, vente_id, pts_gagnes]).map_err(|e| e.to_string())?;
        }
    }

    for l in &lignes {
        let article_id = l.article_id;
        let qte = l.quantite;
        tx.execute(
            "INSERT INTO vente_articles (vente_id, article_id, quantite, prix_unitaire, tva, total_ligne, montant_ht, montant_tva, remise_ligne, note, variante_id, prix_type) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![vente_id, article_id, qte, l.prix_unitaire, l.tva, l.calcul.total_ligne, l.calcul.montant_ht, l.calcul.montant_tva, l.remise_ligne, l.note, l.variante_id, l.prix_type],
        ).map_err(|e| e.to_string())?;

        if encaisse {
            mouvement_ligne(
                &tx,
                Sens::Sortie,
                &LigneStock {
                    article_id,
                    variante_id: l.variante_id,
                    quantite: qte,
                },
                magasin_id,
                vente_id,
                "vente",
                true,
            )?;
        }
    }

    if encaisse {
        for (mode, montant) in &paiements {
            tx.execute(
                "INSERT INTO vente_paiements (vente_id, session_id, mode, montant) VALUES (?1, ?2, ?3, ?4)",
                params![vente_id, current_session_id, mode, montant],
            ).map_err(|e| e.to_string())?;
        }
        if paiements.len() > 1 {
            for (mode, montant) in &paiements {
                tx.execute(
                    "INSERT INTO journal_caisse (utilisateur_id, jtype, montant, description, session_id) VALUES (?1, 'encaissement', ?2, ?3, ?4)",
                    params![caissier_id, montant, format!("Split vente #{}: {}", vente_id, mode), current_session_id],
                ).map_err(|e| e.to_string())?;
            }
        }
    }

    log_audit(
        &tx,
        caissier_id,
        "creer_vente",
        &format!(
            "Vente #{} - {:.2} DH TTC ({}) - {}",
            vente_id, totaux.net_ttc, document_type, mode_paiement
        ),
        Some("vente"),
        Some(vente_id),
    );

    tx.commit().map_err(|e| e.to_string())?;

    Ok(serde_json::json!({
        "id": vente_id,
        "numero_facture": numero_facture,
        "montant_total": totaux.montant_total,
        "montant_remise": totaux.montant_remise,
        "montant_ht": totaux.montant_ht,
        "montant_tva": totaux.montant_tva,
        "net_ttc": totaux.net_ttc,
        "points_gagnes": pts_gagnes,
    }))
}

fn credit_propre(tx: &Connection, vente_id: i64) -> Result<f64, String> {
    let (mode_paiement, montant_total, montant_remise): (String, f64, f64) = tx
        .query_row(
            "SELECT mode_paiement, montant_total, montant_remise FROM ventes WHERE id = ?1",
            params![vente_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .map_err(|e| e.to_string())?;
    let (nb_paiements, credit_paye): (i64, f64) = tx.query_row(
        "SELECT COUNT(*), COALESCE(SUM(CASE WHEN mode = 'credit' THEN montant END), 0) FROM vente_paiements WHERE vente_id = ?1",
        params![vente_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    ).map_err(|e| e.to_string())?;
    if nb_paiements > 0 {
        return Ok(credit_paye);
    }
    Ok(if mode_paiement == "credit" {
        round2((montant_total - montant_remise).abs())
    } else {
        0.0
    })
}

pub(crate) fn credit_porte(tx: &Connection, vente_id: i64) -> Result<f64, String> {
    let (dtype, source): (String, Option<i64>) = tx
        .query_row(
            "SELECT COALESCE(dtype, 'facture'), source_vente_id FROM ventes WHERE id = ?1",
            params![vente_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|e| e.to_string())?;
    if !matches!(dtype.as_str(), "facture" | "bl") {
        return Ok(0.0);
    }
    if let Some(src) = source {
        let src_dtype: String = tx
            .query_row(
                "SELECT COALESCE(dtype, 'facture') FROM ventes WHERE id = ?1",
                params![src],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        if matches!(src_dtype.as_str(), "facture" | "bl") {
            return credit_porte(tx, src);
        }
    }
    credit_propre(tx, vente_id)
}

fn credit_repris_par_avoir(tx: &Connection, avoir_id: i64) -> Result<f64, String> {
    let (source, montant_total, montant_remise): (Option<i64>, f64, f64) = tx
        .query_row(
            "SELECT source_vente_id, montant_total, montant_remise FROM ventes WHERE id = ?1",
            params![avoir_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .map_err(|e| e.to_string())?;
    let Some(src) = source else { return Ok(0.0) };
    Ok(credit_porte(tx, src)?.min(round2((montant_total - montant_remise).abs())))
}

fn verifier_plafond(tx: &Connection, client_id: i64, montant: f64) -> Result<(), String> {
    let (actuel, plafond): (f64, Option<f64>) = tx
        .query_row(
            "SELECT COALESCE(credit_actuel, 0), credit_plafond FROM clients WHERE id = ?1",
            params![client_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|e| e.to_string())?;
    if let Some(plaf) = plafond {
        if plaf > 0.0 && actuel + montant > plaf + TOLERANCE_MONTANT {
            return Err(format!(
                "Plafond de crédit dépassé. Crédit actuel: {}, Plafond: {}, Demandé: {}",
                actuel, plaf, montant
            ));
        }
    }
    Ok(())
}

fn ajuster_credit(tx: &Connection, client_id: i64, delta: f64) -> Result<(), String> {
    if delta != 0.0 {
        tx.execute(
            "UPDATE clients SET credit_actuel = COALESCE(credit_actuel, 0) + ?1 WHERE id = ?2",
            params![round2(delta), client_id],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn annuler_vente(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    vente_id: i64,
    motif: Option<String>,
) -> Result<(), String> {
    let mut conn = db.conn.lock().map_err(|e| e.to_string())?;
    let me = autoriser(&auth, &conn, &token, Acces::Module("ventes", "modifier"))?;
    annuler_vente_impl(&mut conn, vente_id, Some(me.user_id), motif.as_deref())
}

pub(crate) fn annuler_vente_impl(
    conn: &mut Connection,
    vente_id: i64,
    auteur: Option<i64>,
    motif: Option<&str>,
) -> Result<(), String> {
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    let (statut, dtype, vente_magasin_id, client_id): (String, String, Option<i64>, Option<i64>) = tx.query_row(
        "SELECT statut, COALESCE(dtype, 'facture'), magasin_id, client_id FROM ventes WHERE id = ?1",
        params![vente_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    ).optional().map_err(|e| e.to_string())?.ok_or("Document introuvable")?;

    match statut.as_str() {
        "annulee" => return Err("Cette vente est déjà annulée".to_string()),
        "convertie" => return Err("Ce document a déjà été converti : annulez le document issu de la conversion ou émettez un avoir".to_string()),
        _ => {}
    }

    if let Some(cid) = client_id {
        match dtype.as_str() {
            "facture" | "bl" => ajuster_credit(&tx, cid, -credit_porte(&tx, vente_id)?)?,
            "avoir" => ajuster_credit(&tx, cid, credit_repris_par_avoir(&tx, vente_id)?)?,
            _ => {}
        }
        let mouvements: Vec<(String, f64)> = {
            let mut stmt = tx.prepare(
                "SELECT mtype, COALESCE(SUM(points), 0) FROM mouvements_fidelite WHERE vente_id = ?1 AND mtype IN ('gain', 'depense') GROUP BY mtype"
            ).map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map(params![vente_id], |r| Ok((r.get(0)?, r.get(1)?)))
                .map_err(|e| e.to_string())?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
                .map_err(|e| e.to_string())?
        };
        for (mtype, points) in mouvements {
            let (delta, annulation) = if mtype == "gain" {
                (-points, "annulation_gain")
            } else {
                (points, "annulation_depense")
            };
            tx.execute("UPDATE clients SET points_fidelite = COALESCE(points_fidelite, 0) + ?1 WHERE id = ?2", params![delta, cid])
                .map_err(|e| e.to_string())?;
            tx.execute("INSERT INTO mouvements_fidelite (client_id, vente_id, points, mtype) VALUES (?1, ?2, ?3, ?4)", params![cid, vente_id, points, annulation])
                .map_err(|e| e.to_string())?;
        }
    }

    let stock_was_deducted = dtype == "facture" || dtype == "bl";
    let is_avoir = dtype == "avoir";

    if stock_was_deducted || is_avoir {
        let magasin_id = vente_magasin_id.map_or_else(|| default_magasin_id(&tx), Ok)?;
        let mut stmt = tx
            .prepare(
                "SELECT article_id, quantite, variante_id FROM vente_articles WHERE vente_id = ?1",
            )
            .map_err(|e| e.to_string())?;
        let lignes = stmt
            .query_map(params![vente_id], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, f64>(1)?,
                    row.get::<_, Option<i64>>(2)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        let lignes: Vec<(i64, f64, Option<i64>)> = lignes
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| e.to_string())?;
        drop(stmt);

        let (sens, reference) = if is_avoir {
            (Sens::Sortie, "annulation_avoir")
        } else {
            (Sens::Entree, "annulation_vente")
        };
        for (article_id, quantite, variante_id) in lignes {
            mouvement_ligne(
                &tx,
                sens,
                &LigneStock {
                    article_id,
                    variante_id,
                    quantite,
                },
                magasin_id,
                vente_id,
                reference,
                false,
            )?;
        }
        let proprietaire = if is_avoir {
            vente_id
        } else {
            proprietaire_lots(&tx, vente_id)?
        };
        inverser_lots(&tx, proprietaire, None)?;
    }

    let numero_facture: Option<String> = tx
        .query_row(
            "SELECT numero_facture FROM ventes WHERE id = ?1",
            params![vente_id],
            |r| r.get(0),
        )
        .ok();
    let montant: f64 = tx
        .query_row(
            "SELECT montant_total - montant_remise FROM ventes WHERE id = ?1",
            params![vente_id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    tx.execute(
        "UPDATE ventes SET statut = 'annulee' WHERE id = ?1",
        params![vente_id],
    )
    .map_err(|e| e.to_string())?;
    let motif = motif
        .map(str::trim)
        .filter(|m| !m.is_empty())
        .map(|m| format!(" - Motif: {}", m))
        .unwrap_or_default();
    log_audit(
        &tx,
        auteur,
        "annuler_vente",
        &format!(
            "Annulation {} #{} ({}) - Montant: {:.2}{}",
            dtype,
            vente_id,
            numero_facture.unwrap_or_default(),
            montant,
            motif
        ),
        Some("vente"),
        Some(vente_id),
    );
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_ventes(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    debut: Option<String>,
    fin: Option<String>,
) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let _me = autoriser(&auth, &conn, &token, Acces::Module("ventes", "voir"))?;
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
    let params_refs: Vec<&dyn rusqlite::types::ToSql> =
        query_params.iter().map(|p| p.as_ref()).collect();
    let rows = stmt
        .query_map(params_refs.as_slice(), |row| {
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
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_vente_details(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    vente_id: i64,
) -> Result<serde_json::Value, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let _me = autoriser(&auth, &conn, &token, Acces::Module("ventes", "voir"))?;
    let vente = conn.query_row(
        "SELECT v.id, v.date, v.montant_total, v.montant_remise, v.mode_paiement, v.statut, v.numero_facture,
                c.nom as client_nom, c.telephone as client_tel, u.nom as caissier_nom, v.dtype, c.ice as client_ice,
                v.source_vente_id, src.dtype as source_dtype, src.numero_facture as source_numero,
                v.montant_ht, v.montant_tva
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
                "montant_ht": row.get::<_, Option<f64>>(15)?,
                "montant_tva": row.get::<_, Option<f64>>(16)?,
            }))
        }
    ).map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT va.id, va.article_id, a.designation, va.quantite, va.prix_unitaire, va.tva, va.total_ligne, va.remise_ligne, va.montant_ht, va.montant_tva
         FROM vente_articles va
         JOIN articles a ON va.article_id = a.id
         WHERE va.vente_id = ?1"
    ).map_err(|e| e.to_string())?;
    let lignes = stmt
        .query_map(params![vente_id], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, i64>(0)?,
                "article_id": row.get::<_, i64>(1)?,
                "designation": row.get::<_, String>(2)?,
                "quantite": row.get::<_, f64>(3)?,
                "prix_unitaire": row.get::<_, f64>(4)?,
                "tva": row.get::<_, f64>(5)?,
                "total_ligne": row.get::<_, f64>(6)?,
                "remise_ligne": row.get::<_, Option<f64>>(7)?,
                "montant_ht": row.get::<_, Option<f64>>(8)?,
                "montant_tva": row.get::<_, Option<f64>>(9)?,
            }))
        })
        .map_err(|e| e.to_string())?;
    let lignes: Vec<_> = lignes
        .collect::<Result<_, _>>()
        .map_err(|e| e.to_string())?;
    Ok(serde_json::json!({ "vente": vente, "lignes": lignes }))
}

#[tauri::command]
pub fn convert_document(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    vente_id: i64,
    target_type: String,
) -> Result<i64, String> {
    let mut conn = db.conn.lock().map_err(|e| e.to_string())?;
    let _me = autoriser(&auth, &conn, &token, Acces::Module("ventes", "modifier"))?;
    convert_document_impl(&mut conn, vente_id, target_type)
}

pub(crate) fn convert_document_impl(
    conn: &mut Connection,
    vente_id: i64,
    target_type: String,
) -> Result<i64, String> {
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    #[allow(clippy::type_complexity)]
    let (source_dtype, client_id, caissier_id, montant_total, montant_remise, mode_paiement, statut, numero_facture_src, source_magasin_id, montant_ht, montant_tva): (String, Option<i64>, Option<i64>, f64, f64, String, String, Option<String>, Option<i64>, Option<f64>, Option<f64>) = tx.query_row(
        "SELECT dtype, client_id, caissier_id, montant_total, montant_remise, mode_paiement, statut, numero_facture, magasin_id, montant_ht, montant_tva FROM ventes WHERE id = ?1",
        params![vente_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?, row.get(7)?, row.get(8)?, row.get(9)?, row.get(10)?)),
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
        return Err(format!(
            "Ce document a déjà été converti en {}",
            target_type
        ));
    }

    let allowed = match source_dtype.as_str() {
        "devis" => target_type == "facture" || target_type == "bl",
        "bl" => target_type == "facture",
        "facture" => target_type == "avoir",
        _ => false,
    };
    if !allowed {
        return Err(format!(
            "Conversion {} → {} non autorisée",
            source_dtype, target_type
        ));
    }

    let mut stmt = tx.prepare(
        "SELECT article_id, quantite, prix_unitaire, tva, total_ligne, remise_ligne, note, variante_id, prix_type, montant_ht, montant_tva FROM vente_articles WHERE vente_id = ?1"
    ).map_err(|e| e.to_string())?;
    #[allow(clippy::type_complexity)]
    let lignes: Vec<(
        i64,
        f64,
        f64,
        f64,
        f64,
        Option<f64>,
        Option<String>,
        Option<i64>,
        Option<String>,
        Option<f64>,
        Option<f64>,
    )> = stmt
        .query_map(params![vente_id], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
                row.get(7)?,
                row.get(8)?,
                row.get(9)?,
                row.get(10)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| e.to_string())?;
    drop(stmt);

    let numero_facture = next_numero_document(&tx, &target_type, annee_courante())?;

    let new_montant_total = if target_type == "avoir" {
        -montant_total.abs()
    } else {
        montant_total
    };
    let new_montant_remise = if target_type == "avoir" {
        -montant_remise.abs()
    } else {
        montant_remise
    };
    let signe = |v: Option<f64>| v.map(|x| if target_type == "avoir" { -x.abs() } else { x });
    let (new_montant_ht, new_montant_tva) = (signe(montant_ht), signe(montant_tva));

    let magasin_id = source_magasin_id.map_or_else(|| default_magasin_id(&tx), Ok)?;

    tx.execute(
        "INSERT INTO ventes (client_id, caissier_id, montant_total, montant_remise, montant_ht, montant_tva, mode_paiement, numero_facture, dtype, source_vente_id, magasin_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![client_id, caissier_id, new_montant_total, new_montant_remise, new_montant_ht, new_montant_tva, mode_paiement, numero_facture, target_type, vente_id, magasin_id],
    ).map_err(|e| e.to_string())?;
    let new_vente_id = tx.last_insert_rowid();

    for (
        article_id,
        qte,
        pu,
        tva,
        total_ligne,
        remise_ligne,
        note,
        variante_id,
        prix_type,
        ligne_ht,
        ligne_tva,
    ) in &lignes
    {
        let new_total_ligne = if target_type == "avoir" {
            -total_ligne.abs()
        } else {
            *total_ligne
        };
        let (new_ligne_ht, new_ligne_tva) = (signe(*ligne_ht), signe(*ligne_tva));
        tx.execute(
            "INSERT INTO vente_articles (vente_id, article_id, quantite, prix_unitaire, tva, total_ligne, montant_ht, montant_tva, remise_ligne, note, variante_id, prix_type) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![new_vente_id, article_id, qte, pu, tva, new_total_ligne, new_ligne_ht, new_ligne_tva, remise_ligne, note, variante_id, prix_type],
        ).map_err(|e| e.to_string())?;

        let ligne_stock = LigneStock {
            article_id: *article_id,
            variante_id: *variante_id,
            quantite: *qte,
        };
        if target_type == "avoir" {
            mouvement_ligne(
                &tx,
                Sens::Entree,
                &ligne_stock,
                magasin_id,
                new_vente_id,
                "avoir",
                false,
            )?;
        }
        if (target_type == "facture" || target_type == "bl") && source_dtype == "devis" {
            mouvement_ligne(
                &tx,
                Sens::Sortie,
                &ligne_stock,
                magasin_id,
                new_vente_id,
                "vente",
                true,
            )?;
        }
    }

    if target_type == "avoir" {
        inverser_lots(&tx, proprietaire_lots(&tx, vente_id)?, Some(new_vente_id))?;
    }
    tx.execute(
        "UPDATE ventes SET statut = 'convertie' WHERE id = ?1",
        params![vente_id],
    )
    .map_err(|e| e.to_string())?;

    if let Some(cid) = client_id {
        match target_type.as_str() {
            "facture" | "bl" if !matches!(source_dtype.as_str(), "facture" | "bl") => {
                let credit = credit_porte(&tx, new_vente_id)?;
                if credit > 0.0 {
                    verifier_plafond(&tx, cid, credit)?;
                    ajuster_credit(&tx, cid, credit)?;
                }
            }
            "avoir" => ajuster_credit(&tx, cid, -credit_repris_par_avoir(&tx, new_vente_id)?)?,
            _ => {}
        }
    }

    let source_ref = numero_facture_src.unwrap_or_else(|| format!("#{}", vente_id));
    log_audit(
        &tx,
        caissier_id,
        "convertir_document",
        &format!(
            "Conversion {} {} → {} {}",
            source_dtype, source_ref, target_type, numero_facture
        ),
        Some("vente"),
        Some(new_vente_id),
    );

    tx.commit().map_err(|e| e.to_string())?;
    Ok(new_vente_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn setup() -> Connection {
        let conn = crate::db::init_db(":memory:").unwrap();
        conn.execute_batch("
            INSERT INTO articles (id, designation, prix_vente, prix_grossiste, tva) VALUES (1, 'Huile', 100, 80, 20);
            INSERT INTO articles (id, designation, prix_vente, tva) VALUES (2, 'Pain', 10, 0);
            INSERT INTO clients (id, nom, points_fidelite) VALUES (1, 'Client', 50);
            UPDATE settings SET value = '1' WHERE key = 'fidelite_valeur_1_point';
            UPDATE settings SET value = '100' WHERE key = 'fidelite_dh_pour_1_point';
            UPDATE settings SET value = 'true' WHERE key = 'fidelite_actif';
            INSERT INTO article_stocks (article_id, magasin_id, quantite) SELECT id, 1, 100 FROM articles; UPDATE articles SET stock = 100;
        ").unwrap();
        conn
    }

    fn vendre(
        conn: &mut Connection,
        client: Option<i64>,
        lignes: serde_json::Value,
        remise: Option<f64>,
        paiements: serde_json::Value,
        dtype: &str,
        points: Option<f64>,
    ) -> Result<serde_json::Value, String> {
        create_vente_impl(
            conn,
            client,
            Some(1),
            lignes.as_array().unwrap().clone(),
            remise,
            "especes".into(),
            Some(paiements.as_array().unwrap().clone()),
            Some(dtype.into()),
            points,
            None,
        )
    }

    fn montants(conn: &Connection, id: i64) -> (f64, f64, f64, f64) {
        conn.query_row("SELECT montant_total, montant_remise, montant_ht, montant_tva FROM ventes WHERE id = ?1", params![id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))).unwrap()
    }

    #[test]
    fn test_prix_et_tva_lus_depuis_le_catalogue() {
        let mut conn = setup();
        let r = vendre(
            &mut conn,
            None,
            json!([{ "article_id": 1, "quantite": 2, "prix_unitaire": 1, "tva": 0 }]),
            None,
            json!([{ "mode": "especes", "montant": 240 }]),
            "facture",
            None,
        )
        .unwrap();
        assert_eq!(r["net_ttc"], json!(240.0));
        let id = r["id"].as_i64().unwrap();
        assert_eq!(montants(&conn, id), (240.0, 0.0, 200.0, 40.0));
        let (pu, tva, total): (f64, f64, f64) = conn
            .query_row(
                "SELECT prix_unitaire, tva, total_ligne FROM vente_articles WHERE vente_id = ?1",
                params![id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!((pu, tva, total), (100.0, 20.0, 240.0));
    }

    #[test]
    fn test_remises_ligne_et_document() {
        let mut conn = setup();
        let r = vendre(
            &mut conn,
            None,
            json!([
                { "article_id": 1, "quantite": 1, "remise_ligne": 10 },
                { "article_id": 2, "quantite": 3 }
            ]),
            Some(10.0),
            json!([{ "mode": "especes", "montant": 124.2 }]),
            "facture",
            None,
        )
        .unwrap();
        let id = r["id"].as_i64().unwrap();
        let (total, remise, ht, tva) = montants(&conn, id);
        assert_eq!((total, ht, tva), (138.0, 108.0, 16.2));
        assert_eq!(remise, 13.8);
        assert_eq!(round2(total - remise), round2(ht + tva));
        let somme_tva: f64 = conn
            .query_row(
                "SELECT SUM(montant_tva) FROM vente_articles WHERE vente_id = ?1",
                params![id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(somme_tva, tva);
    }

    #[test]
    fn test_montant_encaisse_incoherent_refuse() {
        let mut conn = setup();
        let err = vendre(
            &mut conn,
            None,
            json!([{ "article_id": 1, "quantite": 1 }]),
            None,
            json!([{ "mode": "especes", "montant": 100 }]),
            "facture",
            None,
        )
        .unwrap_err();
        assert!(err.contains("net à payer"));
        let nb: i64 = conn
            .query_row("SELECT COUNT(*) FROM ventes", [], |r| r.get(0))
            .unwrap();
        assert_eq!(nb, 0);
    }

    #[test]
    fn test_prix_grossiste() {
        let mut conn = setup();
        let r = vendre(
            &mut conn,
            None,
            json!([{ "article_id": 1, "quantite": 1, "prix_type": "grossiste" }]),
            None,
            json!([{ "mode": "carte", "montant": 96 }]),
            "facture",
            None,
        )
        .unwrap();
        assert_eq!(r["net_ttc"], json!(96.0));
    }

    #[test]
    fn test_devis_sans_controle_d_encaissement() {
        let mut conn = setup();
        let r = vendre(
            &mut conn,
            Some(1),
            json!([{ "article_id": 1, "quantite": 1 }]),
            None,
            json!([{ "mode": "especes", "montant": 1 }]),
            "devis",
            None,
        )
        .unwrap();
        assert_eq!(r["points_gagnes"], json!(0.0));
        let nb: i64 = conn
            .query_row("SELECT COUNT(*) FROM vente_paiements", [], |r| r.get(0))
            .unwrap();
        assert_eq!(nb, 0);
    }

    #[test]
    fn test_points_fidelite() {
        let mut conn = setup();
        let r = vendre(
            &mut conn,
            Some(1),
            json!([{ "article_id": 1, "quantite": 1 }]),
            None,
            json!([{ "mode": "especes", "montant": 100 }, { "mode": "fidelite", "montant": 20 }]),
            "facture",
            Some(20.0),
        )
        .unwrap();
        assert_eq!(r["points_gagnes"], json!(1.0));
        let points: f64 = conn
            .query_row(
                "SELECT points_fidelite FROM clients WHERE id = 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(points, 31.0);
        let tva: f64 = conn
            .query_row(
                "SELECT montant_tva FROM ventes WHERE id = ?1",
                params![r["id"].as_i64().unwrap()],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(tva, 20.0);
    }

    #[test]
    fn test_points_fidelite_controles() {
        let mut conn = setup();
        let trop = vendre(
            &mut conn,
            Some(1),
            json!([{ "article_id": 1, "quantite": 1 }]),
            None,
            json!([{ "mode": "especes", "montant": 60 }, { "mode": "fidelite", "montant": 60 }]),
            "facture",
            Some(60.0),
        );
        assert!(trop.unwrap_err().contains("Points insuffisants"));
        let incoherent = vendre(
            &mut conn,
            Some(1),
            json!([{ "article_id": 1, "quantite": 1 }]),
            None,
            json!([{ "mode": "especes", "montant": 90 }, { "mode": "fidelite", "montant": 30 }]),
            "facture",
            Some(10.0),
        );
        assert!(incoherent.unwrap_err().contains("fidélité"));
        let sans_points = vendre(
            &mut conn,
            Some(1),
            json!([{ "article_id": 1, "quantite": 1 }]),
            None,
            json!([{ "mode": "especes", "montant": 100 }, { "mode": "fidelite", "montant": 20 }]),
            "facture",
            None,
        );
        assert!(sans_points.is_err());
        let sans_client = vendre(
            &mut conn,
            None,
            json!([{ "article_id": 1, "quantite": 1 }]),
            None,
            json!([{ "mode": "especes", "montant": 110 }, { "mode": "fidelite", "montant": 10 }]),
            "facture",
            Some(10.0),
        );
        assert!(sans_client.is_err());
    }

    #[test]
    fn test_lignes_invalides() {
        let mut conn = setup();
        let paiement = json!([{ "mode": "especes", "montant": 120 }]);
        assert!(vendre(
            &mut conn,
            None,
            json!([{ "article_id": 1, "quantite": 0 }]),
            None,
            paiement.clone(),
            "facture",
            None
        )
        .is_err());
        assert!(vendre(
            &mut conn,
            None,
            json!([{ "article_id": 1, "quantite": 1, "remise_ligne": 150 }]),
            None,
            paiement.clone(),
            "facture",
            None
        )
        .is_err());
        assert!(vendre(
            &mut conn,
            None,
            json!([{ "article_id": 1, "quantite": 1 }]),
            Some(-5.0),
            paiement.clone(),
            "facture",
            None
        )
        .is_err());
        assert!(vendre(
            &mut conn,
            None,
            json!([{ "article_id": 99, "quantite": 1 }]),
            None,
            paiement.clone(),
            "facture",
            None
        )
        .is_err());
        assert!(vendre(&mut conn, None, json!([]), None, paiement, "facture", None).is_err());
    }

    #[test]
    fn test_avoir_reprend_les_montants_en_negatif() {
        let mut conn = setup();
        let r = vendre(
            &mut conn,
            None,
            json!([{ "article_id": 1, "quantite": 1 }]),
            Some(10.0),
            json!([{ "mode": "especes", "montant": 108 }]),
            "facture",
            None,
        )
        .unwrap();
        let facture = r["id"].as_i64().unwrap();
        let avoir = convert_document_impl(&mut conn, facture, "avoir".into()).unwrap();
        assert_eq!(montants(&conn, avoir), (-120.0, -12.0, -90.0, -18.0));
        let (ht, tva): (f64, f64) = conn
            .query_row(
                "SELECT montant_ht, montant_tva FROM vente_articles WHERE vente_id = ?1",
                params![avoir],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!((ht, tva), (-90.0, -18.0));
        assert!(avoir > facture);
    }

    fn credit(conn: &Connection) -> f64 {
        conn.query_row("SELECT credit_actuel FROM clients WHERE id = 1", [], |r| {
            r.get(0)
        })
        .unwrap()
    }

    fn points(conn: &Connection) -> f64 {
        conn.query_row(
            "SELECT points_fidelite FROM clients WHERE id = 1",
            [],
            |r| r.get(0),
        )
        .unwrap()
    }

    fn document_credit(conn: &mut Connection, dtype: &str) -> i64 {
        create_vente_impl(
            conn,
            Some(1),
            Some(1),
            vec![json!({ "article_id": 1, "quantite": 1 })],
            None,
            "credit".into(),
            None,
            Some(dtype.into()),
            None,
            None,
        )
        .unwrap()["id"]
            .as_i64()
            .unwrap()
    }

    #[test]
    fn test_bl_converti_en_facture_credit_compte_une_fois() {
        let mut conn = setup();
        let bl = document_credit(&mut conn, "bl");
        assert_eq!(credit(&conn), 120.0);
        let facture = convert_document_impl(&mut conn, bl, "facture".into()).unwrap();
        assert_eq!(credit(&conn), 120.0);
        let avoir = convert_document_impl(&mut conn, facture, "avoir".into()).unwrap();
        assert_eq!(credit(&conn), 0.0);
        annuler_vente_impl(&mut conn, avoir, None, None).unwrap();
        assert_eq!(credit(&conn), 120.0);
    }

    #[test]
    fn test_devis_a_credit_converti_verifie_le_plafond() {
        let mut conn = setup();
        conn.execute("UPDATE clients SET credit_plafond = 100 WHERE id = 1", [])
            .unwrap();
        let devis = create_vente_impl(
            &mut conn,
            Some(1),
            Some(1),
            vec![json!({ "article_id": 2, "quantite": 1 })],
            None,
            "credit".into(),
            None,
            Some("devis".into()),
            None,
            None,
        )
        .unwrap()["id"]
            .as_i64()
            .unwrap();
        conn.execute("UPDATE vente_articles SET quantite = 12, total_ligne = 120, montant_ht = 120 WHERE vente_id = ?1", params![devis]).unwrap();
        conn.execute(
            "UPDATE ventes SET montant_total = 120, montant_ht = 120 WHERE id = ?1",
            params![devis],
        )
        .unwrap();
        assert!(convert_document_impl(&mut conn, devis, "facture".into())
            .unwrap_err()
            .contains("Plafond"));
        assert_eq!(credit(&conn), 0.0);
        conn.execute("UPDATE clients SET credit_plafond = 500 WHERE id = 1", [])
            .unwrap();
        convert_document_impl(&mut conn, devis, "facture".into()).unwrap();
        assert_eq!(credit(&conn), 120.0);
    }

    #[test]
    fn test_annulation_reverse_le_credit_et_les_points() {
        let mut conn = setup();
        let r = vendre(
            &mut conn,
            Some(1),
            json!([{ "article_id": 1, "quantite": 1 }]),
            None,
            json!([{ "mode": "credit", "montant": 90 }, { "mode": "fidelite", "montant": 30 }]),
            "facture",
            Some(30.0),
        )
        .unwrap();
        assert_eq!(credit(&conn), 90.0);
        assert_eq!(points(&conn), 50.0 - 30.0 + 0.0);
        let facture = r["id"].as_i64().unwrap();
        annuler_vente_impl(&mut conn, facture, Some(1), Some("Erreur de saisie")).unwrap();
        assert_eq!(credit(&conn), 0.0);
        assert_eq!(points(&conn), 50.0);
        let detail: String = conn
            .query_row(
                "SELECT detail FROM audit_log WHERE action = 'annuler_vente'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(detail.contains("Motif: Erreur de saisie"));
        assert!(annuler_vente_impl(&mut conn, facture, None, None)
            .unwrap_err()
            .contains("déjà annulée"));
    }

    #[test]
    fn test_annulation_reverse_les_points_gagnes() {
        let mut conn = setup();
        let r = vendre(
            &mut conn,
            Some(1),
            json!([{ "article_id": 1, "quantite": 5 }]),
            None,
            json!([{ "mode": "especes", "montant": 600 }]),
            "facture",
            None,
        )
        .unwrap();
        assert_eq!(points(&conn), 56.0);
        annuler_vente_impl(&mut conn, r["id"].as_i64().unwrap(), None, None).unwrap();
        assert_eq!(points(&conn), 50.0);
    }

    #[test]
    fn test_document_converti_non_annulable() {
        let mut conn = setup();
        let bl = document_credit(&mut conn, "bl");
        let facture = convert_document_impl(&mut conn, bl, "facture".into()).unwrap();
        assert!(annuler_vente_impl(&mut conn, bl, None, None)
            .unwrap_err()
            .contains("converti"));
        let stock_avant: f64 = conn
            .query_row("SELECT stock FROM articles WHERE id = 1", [], |r| r.get(0))
            .unwrap();
        annuler_vente_impl(&mut conn, facture, None, None).unwrap();
        let stock_apres: f64 = conn
            .query_row("SELECT stock FROM articles WHERE id = 1", [], |r| r.get(0))
            .unwrap();
        assert_eq!(stock_apres, stock_avant + 1.0);
        assert_eq!(credit(&conn), 0.0);
    }

    #[test]
    fn test_migration_des_ventes_historiques() {
        let path = std::env::temp_dir().join(format!(
            "supercaisse_test_c8_{}_{}.db",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path_str = path.to_string_lossy().to_string();
        {
            let conn = crate::db::init_db(&path_str).unwrap();
            conn.execute_batch("
                INSERT INTO articles (id, designation, prix_vente, tva) VALUES (1, 'Huile', 100, 20);
                INSERT INTO ventes (id, montant_total, montant_remise, mode_paiement, dtype) VALUES (1, 100, 12, 'especes', 'facture');
                INSERT INTO vente_articles (vente_id, article_id, quantite, prix_unitaire, tva, total_ligne) VALUES (1, 1, 1, 100, 20, 120);
                INSERT INTO ventes (id, montant_total, montant_remise, mode_paiement, dtype) VALUES (2, -100, -12, 'especes', 'avoir');
                INSERT INTO vente_articles (vente_id, article_id, quantite, prix_unitaire, tva, total_ligne) VALUES (2, 1, 1, 100, 20, -120);
                PRAGMA user_version = 0;
            ").unwrap();
        }
        let conn = crate::db::init_db(&path_str).unwrap();
        assert_eq!(montants(&conn, 1), (120.0, 12.0, 90.0, 18.0));
        assert_eq!(montants(&conn, 2), (-120.0, -12.0, -90.0, -18.0));
        drop(conn);
        let conn = crate::db::init_db(&path_str).unwrap();
        assert_eq!(montants(&conn, 1), (120.0, 12.0, 90.0, 18.0));
        drop(conn);
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{}", path_str, suffix));
        }
    }
}
