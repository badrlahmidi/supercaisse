use crate::db::*;
use crate::session::{autoriser, Acces, AuthState};
use rusqlite::{params, Connection, OptionalExtension};
use tauri::State;

use super::calcul::{
    calculer_ligne, en_dh, round2, somme_dh, totaliser, valider_pourcentage, vers_centimes,
    LigneCalculee, TOLERANCE_MONTANT,
};
use super::contrats::{
    LigneVenteDetail, LigneVenteSaisie, ListeVentes, ModePaiement, PaiementSaisi, TypeDocument,
    VenteCreee, VenteDetail, VenteEntete, VenteResume,
};
use super::mouvements::{inverser_lots, mouvement_ligne, proprietaire_lots, LigneStock, Sens};
use super::{
    annee_courante, default_magasin_id, document_prefixe, log_audit, next_numero_document,
};

pub(crate) fn mode_de_vente(paiements: &[(String, f64)]) -> String {
    let mut modes: Vec<&str> = paiements.iter().map(|(m, _)| m.as_str()).collect();
    modes.dedup();
    match modes.as_slice() {
        [mode] => mode.to_string(),
        _ => "mixte".to_string(),
    }
}

pub(crate) fn normaliser_paiements(
    mode_paiement: ModePaiement,
    splits: Option<&[PaiementSaisi]>,
    montant_par_defaut: f64,
) -> Result<Vec<(String, f64)>, String> {
    let paiements: Vec<(ModePaiement, f64)> = match splits {
        Some(list) if !list.is_empty() => list.iter().map(|s| (s.mode, s.montant)).collect(),
        _ => vec![(mode_paiement, montant_par_defaut.max(0.0))],
    };
    for (mode, montant) in &paiements {
        if *mode == ModePaiement::Mixte {
            return Err(
                "« mixte » n'est pas un mode de règlement : détaillez chaque paiement".to_string(),
            );
        }
        if !montant.is_finite() || *montant < 0.0 {
            return Err(format!(
                "Montant invalide pour le paiement {} : {}",
                mode.code(),
                montant
            ));
        }
    }
    Ok(paiements
        .into_iter()
        .map(|(mode, montant)| (mode.code().to_string(), round2(montant)))
        .collect())
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
    articles: &[LigneVenteSaisie],
    remise_globale: f64,
) -> Result<Vec<LigneVente>, String> {
    if articles.is_empty() {
        return Err("Le document ne contient aucun article".to_string());
    }
    articles
        .iter()
        .map(|a| {
            let article_id = a.article_id;
            let quantite = a.quantite;
            if !quantite.is_finite() || quantite <= 0.0 {
                return Err(format!(
                    "Quantité invalide (article {}) : {}",
                    article_id, quantite
                ));
            }
            let remise_ligne = valider_pourcentage("Remise ligne", a.remise_ligne.unwrap_or(0.0))?;
            let prix_type = a.prix_type.unwrap_or_default().code().to_string();
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
                variante_id: a.variante_id,
                quantite,
                prix_unitaire,
                tva,
                remise_ligne,
                note: a.note.clone(),
                prix_type,
                calcul: calculer_ligne(quantite, prix_unitaire, tva, remise_ligne, remise_globale),
            })
        })
        .collect()
}

pub(crate) fn verifier_plafond_remise(
    tx: &Connection,
    caissier_id: Option<i64>,
    articles: &[LigneVenteSaisie],
    remise_globale: f64,
) -> Result<(), String> {
    let Some(id) = caissier_id else {
        return Ok(());
    };
    let role: Option<String> = tx
        .query_row(
            "SELECT role FROM utilisateurs WHERE id = ?1",
            params![id],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    let cle = match role.as_deref() {
        Some("caissier") => "remise_max_caissier",
        Some("manager") => "remise_max_manager",
        _ => return Ok(()),
    };
    let plafond = lire_setting(tx, cle)?
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(if cle == "remise_max_caissier" {
            10.0
        } else {
            100.0
        });
    let effective = |remise_ligne: f64| {
        100.0 * (1.0 - (1.0 - remise_ligne / 100.0) * (1.0 - remise_globale / 100.0))
    };
    let max = articles
        .iter()
        .map(|a| effective(a.remise_ligne.unwrap_or(0.0)))
        .fold(effective(0.0), f64::max);
    if max > plafond + 1e-9 {
        return Err(format!(
            "Remise de {:.2} % supérieure au plafond de {:.2} % autorisé pour le rôle {} : faites valider la vente par un responsable",
            max,
            plafond,
            role.unwrap_or_default()
        ));
    }
    Ok(())
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

#[tauri::command(async)]
pub fn create_vente(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    client_id: Option<i64>,
    articles: Vec<LigneVenteSaisie>,
    remise_globale_pct: Option<f64>,
    mode_paiement: ModePaiement,
    splits: Option<Vec<PaiementSaisi>>,
    dtype: Option<TypeDocument>,
    points_utilises: Option<f64>,
    magasin_id: Option<i64>,
) -> Result<VenteCreee, String> {
    let mut conn = db.conn.lock().map_err(|e| e.to_string())?;
    let me = autoriser(&auth, &conn, &token, Acces::Module("ventes", "creer"))?;
    if super::fiscal::est_fiscal(dtype.unwrap_or(TypeDocument::Facture).code()) {
        super::fiscal::verifier_mentions_vendeur(&conn)?;
    }
    let resultat = create_vente_impl(
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
    );
    super::tracer(&format!("Vente par l'utilisateur {}", me.user_id), resultat)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn create_vente_impl(
    conn: &mut Connection,
    client_id: Option<i64>,
    caissier_id: Option<i64>,
    articles: Vec<LigneVenteSaisie>,
    remise_globale_pct: Option<f64>,
    mode_paiement: ModePaiement,
    splits: Option<Vec<PaiementSaisi>>,
    dtype: Option<TypeDocument>,
    points_utilises: Option<f64>,
    magasin_id: Option<i64>,
) -> Result<VenteCreee, String> {
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let magasin_id = match magasin_id {
        Some(id) => id,
        None => {
            if let Some(cid) = caissier_id {
                tx.query_row(
                    "SELECT magasin_id FROM sessions_caisse WHERE caissier_id = ?1 AND statut = 'ouverte' ORDER BY id DESC LIMIT 1",
                    params![cid], |r| r.get::<_, Option<i64>>(0)
                ).optional().map_err(|e| e.to_string())?.flatten().map_or_else(|| default_magasin_id(&tx), Ok)?
            } else {
                default_magasin_id(&tx)?
            }
        }
    };

    let document_type = dtype.unwrap_or(TypeDocument::Facture).code().to_string();
    document_prefixe(&document_type)?;
    let encaisse = matches!(document_type.as_str(), "facture" | "bl");
    if document_type == "facture" {
        super::fiscal::verifier_ice_client(&tx, client_id)?;
    }

    let remise_globale = valider_pourcentage("Remise document", remise_globale_pct.unwrap_or(0.0))?;
    let lignes = preparer_lignes(&tx, &articles, remise_globale)?;
    verifier_plafond_remise(&tx, caissier_id, &articles, remise_globale)?;
    let calculs: Vec<LigneCalculee> = lignes.iter().map(|l| l.calcul).collect();
    let totaux = totaliser(&calculs);

    let paiements = normaliser_paiements(mode_paiement, splits.as_deref(), totaux.net_ttc)?;
    let mode_vente = mode_de_vente(&paiements);

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
    let paiement_fidelite = somme_dh(
        paiements
            .iter()
            .filter(|(m, _)| m == "fidelite")
            .map(|(_, montant)| *montant),
    );

    if encaisse {
        let total_paye = somme_dh(paiements.iter().map(|(_, montant)| *montant));
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

    let credit_demandé = somme_dh(
        paiements
            .iter()
            .filter(|(m, _)| m == "credit")
            .map(|(_, montant)| *montant),
    );
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
                if plaf > 0.0 && somme_dh([actuel, credit_demandé]) > plaf {
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
        .optional()
        .map_err(|e| e.to_string())?
    } else {
        None
    };

    tx.execute(
        "INSERT INTO ventes (client_id, caissier_id, montant_total, montant_remise, montant_ht, montant_tva, mode_paiement, numero_facture, dtype, session_id, points_utilises, points_gagnes, magasin_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        params![client_id, caissier_id, totaux.montant_total, totaux.montant_remise, totaux.montant_ht, totaux.montant_tva, mode_vente, numero_facture, document_type, current_session_id, pts_utilises, pts_gagnes, magasin_id],
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
            vente_id, totaux.net_ttc, document_type, mode_vente
        ),
        Some("vente"),
        Some(vente_id),
    )?;

    tx.commit().map_err(|e| e.to_string())?;

    Ok(VenteCreee {
        id: vente_id,
        numero_facture,
        montant_total: totaux.montant_total,
        montant_remise: totaux.montant_remise,
        montant_ht: totaux.montant_ht,
        montant_tva: totaux.montant_tva,
        net_ttc: totaux.net_ttc,
        points_gagnes: pts_gagnes,
    })
}

fn documents_payes(tx: &Connection, facture_id: i64) -> Result<Vec<i64>, String> {
    let source: Option<i64> = tx
        .query_row(
            "SELECT s.id FROM ventes v JOIN ventes s ON s.id = v.source_vente_id
             WHERE v.id = ?1 AND COALESCE(s.dtype, 'facture') IN ('bl', 'facture')",
            params![facture_id],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    Ok(std::iter::once(facture_id).chain(source).collect())
}

fn inverser_fidelite(
    tx: &Connection,
    client_id: i64,
    documents: &[i64],
    rattachement: i64,
    inversions: &[(&str, f64, &str)],
) -> Result<(), String> {
    let mut a_inverser = Vec::new();
    for document in documents {
        for (mtype, signe, inverse) in inversions {
            let points: f64 = tx
                .query_row(
                    "SELECT COALESCE(SUM(points), 0) FROM mouvements_fidelite WHERE vente_id = ?1 AND mtype = ?2",
                    params![document, mtype],
                    |r| r.get(0),
                )
                .map_err(|e| e.to_string())?;
            if points.abs() >= 1e-9 {
                a_inverser.push((points, *signe, *inverse));
            }
        }
    }
    for (points, signe, inverse) in a_inverser {
        tx.execute(
            "UPDATE clients SET points_fidelite = COALESCE(points_fidelite, 0) + ?1 WHERE id = ?2",
            params![signe * points, client_id],
        )
        .map_err(|e| e.to_string())?;
        tx.execute(
            "INSERT INTO mouvements_fidelite (client_id, vente_id, points, mtype) VALUES (?1, ?2, ?3, ?4)",
            params![client_id, rattachement, points, inverse],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn rembourser_avoir(
    tx: &Connection,
    facture_id: i64,
    avoir_id: i64,
    auteur: i64,
) -> Result<(), String> {
    let documents = documents_payes(tx, facture_id)?;
    let mut paiements: Vec<(String, i64)> = Vec::new();
    for document in &documents {
        let mut stmt = tx
            .prepare(
                "SELECT mode, SUM(ROUND(montant * 100)) FROM vente_paiements WHERE vente_id = ?1 GROUP BY mode",
            )
            .map_err(|e| e.to_string())?;
        let lignes = stmt
            .query_map(params![document], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, f64>(1)?.round() as i64))
            })
            .map_err(|e| e.to_string())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| e.to_string())?;
        paiements.extend(lignes);
    }
    if paiements.is_empty() {
        let (mode, net): (String, f64) = tx
            .query_row(
                "SELECT mode_paiement, montant_total - montant_remise FROM ventes WHERE id = ?1",
                params![facture_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map_err(|e| e.to_string())?;
        paiements.push((mode, vers_centimes(net.abs())));
    }
    let remboursements: Vec<(String, i64)> = paiements
        .into_iter()
        .filter(|(mode, montant)| {
            *montant > 0 && !matches!(mode.as_str(), "credit" | "fidelite" | "mixte")
        })
        .collect();
    if remboursements.is_empty() {
        return Ok(());
    }
    let session: Option<i64> = tx
        .query_row(
            "SELECT id FROM sessions_caisse WHERE caissier_id = ?1 AND statut = 'ouverte' ORDER BY id DESC LIMIT 1",
            params![auteur],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    let especes: i64 = remboursements
        .iter()
        .filter(|(mode, _)| mode == "especes")
        .map(|(_, m)| m)
        .sum();
    if especes > 0 && session.is_none() {
        return Err(format!(
            "L'avoir rembourse {:.2} DH en espèces : ouvrez votre session de caisse avant de l'émettre.",
            en_dh(especes)
        ));
    }
    for (mode, montant) in remboursements {
        tx.execute(
            "INSERT INTO vente_paiements (vente_id, session_id, mode, montant) VALUES (?1, ?2, ?3, ?4)",
            params![avoir_id, session, mode, en_dh(montant)],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
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
        "SELECT COUNT(*), COALESCE(SUM(ROUND((CASE WHEN mode = 'credit' THEN montant END) * 100)) / 100.0, 0) FROM vente_paiements WHERE vente_id = ?1",
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

#[tauri::command(async)]
pub fn annuler_vente(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    vente_id: i64,
    motif: Option<String>,
) -> Result<(), String> {
    let mut conn = db.conn.lock().map_err(|e| e.to_string())?;
    let me = autoriser(&auth, &conn, &token, Acces::Module("ventes", "modifier"))?;
    super::tracer(
        &format!("Annulation de la vente {}", vente_id),
        annuler_vente_impl(&mut conn, vente_id, Some(me.user_id), motif.as_deref()),
    )
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
    super::fiscal::verifier_annulation_directe(&tx, vente_id, &dtype, motif)?;

    if let Some(cid) = client_id {
        match dtype.as_str() {
            "facture" | "bl" => ajuster_credit(&tx, cid, -credit_porte(&tx, vente_id)?)?,
            "avoir" => ajuster_credit(&tx, cid, credit_repris_par_avoir(&tx, vente_id)?)?,
            _ => {}
        }
        inverser_fidelite(
            &tx,
            cid,
            &[vente_id],
            vente_id,
            &[
                ("gain", -1.0, "annulation_gain"),
                ("depense", 1.0, "annulation_depense"),
                ("annulation_gain", 1.0, "retablissement_gain"),
                ("annulation_depense", -1.0, "retablissement_depense"),
            ],
        )?;
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
        .map_err(|e| e.to_string())?;
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
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command(async)]
pub fn get_ventes(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    debut: Option<String>,
    fin: Option<String>,
    recherche: Option<String>,
    page: Option<i64>,
    par_page: Option<i64>,
) -> Result<ListeVentes, String> {
    let conn = db.lecture()?;
    let _me = autoriser(&auth, &conn, &token, Acces::Module("ventes", "voir"))?;
    lister_ventes(&conn, &debut, &fin, &recherche, page, par_page)
}

const SOURCE_VENTES: &str = "ventes v
         LEFT JOIN clients c ON v.client_id = c.id
         LEFT JOIN utilisateurs u ON v.caissier_id = u.id";

pub(crate) fn lister_ventes(
    conn: &Connection,
    debut: &Option<String>,
    fin: &Option<String>,
    recherche: &Option<String>,
    page: Option<i64>,
    par_page: Option<i64>,
) -> Result<ListeVentes, String> {
    let mut filtre = super::Filtre::new();
    filtre.periode("v.date", debut, fin);
    if let Some(r) = recherche
        .as_ref()
        .map(|r| r.trim())
        .filter(|r| !r.is_empty())
    {
        let n = filtre.valeurs.len() + 1;
        filtre.ajouter(
            &format!("(c.nom LIKE ?{n} OR u.nom LIKE ?{n} OR v.numero_facture LIKE ?{n} OR v.mode_paiement LIKE ?{n} OR CAST(v.id AS TEXT) LIKE ?{n})"),
            format!("%{}%", r),
        );
    }
    let valeurs: Vec<&dyn rusqlite::types::ToSql> =
        filtre.valeurs.iter().map(|v| v.as_ref()).collect();
    let ca_centimes: i64 = conn
        .query_row(
            &format!(
                "SELECT COALESCE(SUM(ROUND((v.montant_total - v.montant_remise) * 100)), 0) FROM {} WHERE {} {}",
                SOURCE_VENTES,
                filtre_ca!(),
                filtre.clause
            ),
            valeurs.as_slice(),
            |r| r.get::<_, f64>(0),
        )
        .map_err(|e| e.to_string())? as i64;
    let page = super::paginer(
        conn,
        "v.id, v.date, v.client_id, v.caissier_id, v.montant_total, v.montant_remise, v.mode_paiement, v.statut, v.numero_facture,
         c.nom as client_nom, u.nom as caissier_nom, v.dtype, c.telephone as client_telephone, c.email as client_email",
        SOURCE_VENTES,
        &filtre,
        "v.date DESC, v.id DESC",
        page,
        par_page,
        |row| {
            Ok(VenteResume {
                id: row.get(0)?,
                date: row.get(1)?,
                client_id: row.get(2)?,
                caissier_id: row.get(3)?,
                montant_total: row.get(4)?,
                montant_remise: row.get(5)?,
                mode_paiement: row.get(6)?,
                statut: row.get(7)?,
                numero_facture: row.get(8)?,
                client_nom: row.get(9)?,
                caissier_nom: row.get(10)?,
                dtype: row.get(11)?,
                client_telephone: row.get(12)?,
                client_email: row.get(13)?,
            })
        },
    )?;
    Ok(ListeVentes {
        page,
        chiffre_affaires: en_dh(ca_centimes),
    })
}

#[tauri::command(async)]
pub fn get_vente_details(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    vente_id: i64,
) -> Result<VenteDetail, String> {
    let conn = db.lecture()?;
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
            Ok(VenteEntete {
                id: row.get(0)?,
                date: row.get(1)?,
                montant_total: row.get(2)?,
                montant_remise: row.get(3)?,
                mode_paiement: row.get(4)?,
                statut: row.get(5)?,
                numero_facture: row.get(6)?,
                client_nom: row.get(7)?,
                client_tel: row.get(8)?,
                caissier_nom: row.get(9)?,
                dtype: row.get(10)?,
                client_ice: row.get(11)?,
                source_vente_id: row.get(12)?,
                source_dtype: row.get(13)?,
                source_numero: row.get(14)?,
                montant_ht: row.get(15)?,
                montant_tva: row.get(16)?,
            })
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
            Ok(LigneVenteDetail {
                id: row.get(0)?,
                article_id: row.get(1)?,
                designation: row.get(2)?,
                quantite: row.get(3)?,
                prix_unitaire: row.get(4)?,
                tva: row.get(5)?,
                total_ligne: row.get(6)?,
                remise_ligne: row.get(7)?,
                montant_ht: row.get(8)?,
                montant_tva: row.get(9)?,
            })
        })
        .map_err(|e| e.to_string())?;
    let lignes: Vec<_> = lignes
        .collect::<Result<_, _>>()
        .map_err(|e| e.to_string())?;
    Ok(VenteDetail { vente, lignes })
}

#[tauri::command(async)]
pub fn convert_document(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    vente_id: i64,
    target_type: String,
) -> Result<i64, String> {
    let mut conn = db.conn.lock().map_err(|e| e.to_string())?;
    let me = autoriser(&auth, &conn, &token, Acces::Module("ventes", "modifier"))?;
    if super::fiscal::est_fiscal(&target_type) {
        super::fiscal::verifier_mentions_vendeur(&conn)?;
    }
    super::tracer(
        &format!("Conversion du document {}", vente_id),
        convert_document_impl(&mut conn, vente_id, target_type, me.user_id),
    )
}

pub(crate) fn convert_document_impl(
    conn: &mut Connection,
    vente_id: i64,
    target_type: String,
    auteur: i64,
) -> Result<i64, String> {
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    #[allow(clippy::type_complexity)]
    let (source_dtype, client_id, caissier_id, montant_total, montant_remise, mode_paiement, statut, numero_facture_src, source_magasin_id, montant_ht, montant_tva): (String, Option<i64>, Option<i64>, f64, f64, String, String, Option<String>, Option<i64>, Option<f64>, Option<f64>) = tx.query_row(
        "SELECT dtype, client_id, caissier_id, montant_total, montant_remise, mode_paiement, statut, numero_facture, magasin_id, montant_ht, montant_tva FROM ventes WHERE id = ?1",
        params![vente_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?, row.get(7)?, row.get(8)?, row.get(9)?, row.get(10)?)),
    ).map_err(|_| "Document source introuvable".to_string())?;
    if target_type == "facture" {
        super::fiscal::verifier_ice_client(&tx, client_id)?;
    }

    if statut == "annulee" {
        return Err("Impossible de convertir un document annulé".to_string());
    }

    let already_converted: bool = tx.query_row(
        "SELECT COUNT(*) > 0 FROM ventes WHERE source_vente_id = ?1 AND dtype = ?2 AND statut != 'annulee'",
        params![vente_id, target_type],
        |r| r.get(0),
    ).map_err(|e| e.to_string())?;
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
        if target_type == "avoir" {
            inverser_fidelite(
                &tx,
                cid,
                &documents_payes(&tx, vente_id)?,
                new_vente_id,
                &[
                    ("gain", -1.0, "annulation_gain"),
                    ("depense", 1.0, "annulation_depense"),
                ],
            )?;
        }
    }
    if target_type == "avoir" {
        rembourser_avoir(&tx, vente_id, new_vente_id, auteur)?;
    }

    let source_ref = numero_facture_src.unwrap_or_else(|| format!("#{}", vente_id));
    log_audit(
        &tx,
        Some(auteur),
        "convertir_document",
        &format!(
            "Conversion {} {} → {} {}",
            source_dtype, source_ref, target_type, numero_facture
        ),
        Some("vente"),
        Some(new_vente_id),
    )?;

    tx.commit().map_err(|e| e.to_string())?;
    Ok(new_vente_id)
}

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
pub(crate) fn vendre_json(
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
) -> Result<VenteCreee, String> {
    create_vente_impl(
        conn,
        client_id,
        caissier_id,
        serde_json::from_value(serde_json::Value::Array(articles))
            .map_err(|e| format!("Contrat : {}", e))?,
        remise_globale_pct,
        serde_json::from_value(serde_json::json!(mode_paiement))
            .map_err(|e| format!("Contrat : {}", e))?,
        splits
            .map(|s| serde_json::from_value(serde_json::Value::Array(s)))
            .transpose()
            .map_err(|e| format!("Contrat : {}", e))?,
        dtype
            .map(|d| serde_json::from_value(serde_json::json!(d)))
            .transpose()
            .map_err(|e| format!("Contrat : {}", e))?,
        points_utilises,
        magasin_id,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plafond_de_remise_par_role() {
        let mut conn = setup();
        conn.execute_batch(
            "INSERT INTO utilisateurs (id, login, password_hash, nom, role) VALUES
                (2, 'caissier', 'x', 'Caissier', 'caissier'),
                (3, 'gerant', 'x', 'Gérant', 'manager');
             UPDATE settings SET value = '20' WHERE key = 'remise_max_manager';",
        )
        .unwrap();
        let vendre_par = |conn: &mut Connection, caissier: i64, remise_ligne: f64, remise: f64| {
            let net = round2(240.0 * (1.0 - remise_ligne / 100.0) * (1.0 - remise / 100.0));
            vendre_json(
                conn,
                None,
                Some(caissier),
                vec![json!({ "article_id": 1, "quantite": 2, "remise_ligne": remise_ligne })],
                Some(remise),
                "especes".into(),
                Some(vec![json!({ "mode": "especes", "montant": net })]),
                Some("facture".into()),
                None,
                Some(1),
            )
        };
        vendre_par(&mut conn, 2, 10.0, 0.0).unwrap();
        vendre_par(&mut conn, 2, 0.0, 10.0).unwrap();
        let err = vendre_par(&mut conn, 2, 5.0, 6.0).unwrap_err();
        assert!(err.contains("plafond de 10.00 %"), "{err}");
        assert!(vendre_par(&mut conn, 2, 15.0, 0.0).is_err());
        vendre_par(&mut conn, 3, 20.0, 0.0).unwrap();
        assert!(vendre_par(&mut conn, 3, 0.0, 25.0).is_err());
        vendre_par(&mut conn, 1, 50.0, 50.0).unwrap();
    }

    #[test]
    fn test_mode_de_vente_valide() {
        let p = |m: &[(&str, f64)]| {
            mode_de_vente(
                &m.iter()
                    .map(|(a, b)| (a.to_string(), *b))
                    .collect::<Vec<_>>(),
            )
        };
        assert_eq!(p(&[("especes", 10.0)]), "especes");
        assert_eq!(p(&[("especes", 5.0), ("cb", 5.0)]), "mixte");
        assert_eq!(p(&[("especes", 5.0), ("especes", 5.0)]), "especes");
        assert_eq!(p(&[("cb", 5.0), ("fidelite", 1.0)]), "mixte");
    }
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
    ) -> Result<VenteCreee, String> {
        crate::commands::ventes::vendre_json(
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
    fn test_liste_des_ventes_paginee() {
        let mut conn = setup();
        for _ in 0..5 {
            vendre(
                &mut conn,
                None,
                json!([{ "article_id": 2, "quantite": 1 }]),
                None,
                json!([{ "mode": "especes", "montant": 10 }]),
                "bl",
                None,
            )
            .unwrap();
        }
        conn.execute(
            "UPDATE ventes SET date = '2026-01-10 09:00:00' WHERE id IN (1, 2)",
            [],
        )
        .unwrap();

        let p0 = lister_ventes(&conn, &None, &None, &None, Some(0), Some(2))
            .unwrap()
            .page;
        assert_eq!(
            (p0.total, p0.page, p0.par_page, p0.lignes.len()),
            (5, 0, 2, 2)
        );
        assert_eq!(p0.lignes[0].id, 5);
        let p2 = lister_ventes(&conn, &None, &None, &None, Some(2), Some(2)).unwrap();
        assert_eq!(
            p2.page.lignes.iter().map(|v| v.id).collect::<Vec<_>>(),
            vec![1]
        );

        let janvier = lister_ventes(
            &conn,
            &Some("2026-01-10".into()),
            &Some("2026-01-10".into()),
            &None,
            None,
            None,
        )
        .unwrap();
        assert_eq!(janvier.page.total, 2);
        assert_eq!(janvier.chiffre_affaires, 20.0);
        conn.execute("UPDATE ventes SET statut = 'annulee' WHERE id = 3", [])
            .unwrap();
        let tout = lister_ventes(&conn, &None, &None, &None, Some(0), Some(2)).unwrap();
        assert_eq!((tout.page.total, tout.chiffre_affaires), (5, 40.0));
        let recherche =
            lister_ventes(&conn, &None, &None, &Some(" 4 ".into()), None, None).unwrap();
        assert_eq!(
            recherche
                .page
                .lignes
                .iter()
                .map(|v| v.id)
                .collect::<Vec<_>>(),
            vec![4]
        );
        assert_eq!(janvier.page.par_page, crate::commands::PAR_PAGE_DEFAUT);

        let borne = lister_ventes(&conn, &None, &None, &None, Some(-3), Some(100_000)).unwrap();
        assert_eq!(
            (borne.page.page, borne.page.par_page),
            (0, crate::commands::PAR_PAGE_MAX)
        );
    }

    #[test]
    fn test_prix_et_tva_lus_depuis_le_catalogue() {
        let mut conn = setup();
        let refus = vendre(
            &mut conn,
            None,
            json!([{ "article_id": 1, "quantite": 2, "prix_unitaire": 1, "tva": 0 }]),
            None,
            json!([{ "mode": "especes", "montant": 2 }]),
            "facture",
            None,
        )
        .unwrap_err();
        assert!(refus.contains("unknown field `prix_unitaire`"), "{refus}");
        let r = vendre(
            &mut conn,
            None,
            json!([{ "article_id": 1, "quantite": 2 }]),
            None,
            json!([{ "mode": "especes", "montant": 240 }]),
            "facture",
            None,
        )
        .unwrap();
        assert_eq!(r.net_ttc, 240.0);
        let id = r.id;
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
        let id = r.id;
        let (total, remise, ht, tva) = montants(&conn, id);
        assert_eq!((total, ht, tva), (138.0, 108.0, 16.2));
        assert_eq!(remise, 13.8);
        assert_eq!(round2(total - remise), round2(ht + tva));
        let somme_tva: f64 = conn
            .query_row(
                "SELECT SUM(ROUND((montant_tva) * 100)) / 100.0 FROM vente_articles WHERE vente_id = ?1",
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
        assert_eq!(r.net_ttc, 96.0);
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
        assert_eq!(r.points_gagnes, 0.0);
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
        assert_eq!(r.points_gagnes, 1.0);
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
                params![r.id],
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
    fn test_avoir_rend_les_points_et_rembourse_la_session() {
        let mut conn = setup();
        conn.execute_batch(
            "INSERT INTO utilisateurs (id, login, password_hash, nom, role) VALUES (2, 'gerant2', 'x', 'Gérant', 'manager');
             INSERT INTO sessions_caisse (id, caissier_id, fond_initial, statut, magasin_id) VALUES (1, 1, 0, 'ouverte', 1);",
        )
        .unwrap();
        let facture = vendre(
            &mut conn,
            Some(1),
            json!([{ "article_id": 1, "quantite": 5 }]),
            None,
            json!([{ "mode": "especes", "montant": 570 }, { "mode": "fidelite", "montant": 30 }]),
            "facture",
            Some(30.0),
        )
        .unwrap()
        .id;
        let apres_vente = points(&conn);
        assert_eq!(apres_vente, 50.0 - 30.0 + 5.0);
        let especes = |conn: &Connection| {
            crate::commands::sessions::totaux_especes_session(conn, 1)
                .unwrap()
                .ventes_especes
        };
        assert_eq!(especes(&conn), 570.0);

        let refus = convert_document_impl(&mut conn, facture, "avoir".into(), 2).unwrap_err();
        assert!(refus.contains("ouvrez votre session"), "{}", refus);

        let avoir = convert_document_impl(&mut conn, facture, "avoir".into(), 1).unwrap();
        assert_eq!(points(&conn), 50.0);
        assert_eq!(especes(&conn), 0.0);
        let auteur: Option<i64> = conn
            .query_row(
                "SELECT utilisateur_id FROM audit_log WHERE action = 'convertir_document' ORDER BY id DESC LIMIT 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(auteur, Some(1));

        annuler_vente_impl(&mut conn, avoir, Some(1), Some("Erreur")).unwrap();
        assert_eq!(points(&conn), apres_vente);
        assert_eq!(especes(&conn), 570.0);
    }

    #[test]
    fn test_conversion_attribuee_a_son_auteur() {
        let mut conn = setup();
        conn.execute(
            "INSERT INTO utilisateurs (id, login, password_hash, nom, role) VALUES (2, 'gerant2', 'x', 'Gérant', 'manager')",
            [],
        )
        .unwrap();
        let bl = document_credit(&mut conn, "bl");
        convert_document_impl(&mut conn, bl, "facture".into(), 2).unwrap();
        let auteur: Option<i64> = conn
            .query_row(
                "SELECT utilisateur_id FROM audit_log WHERE action = 'convertir_document'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(auteur, Some(2));
    }

    #[test]
    fn test_avoir_reprend_les_montants_en_negatif() {
        let mut conn = setup();
        conn.execute(
            "INSERT INTO sessions_caisse (caissier_id, fond_initial, statut, magasin_id) VALUES (1, 0, 'ouverte', 1)",
            [],
        )
        .unwrap();
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
        let facture = r.id;
        let avoir = convert_document_impl(&mut conn, facture, "avoir".into(), 1).unwrap();
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
        crate::commands::ventes::vendre_json(
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
        .unwrap()
        .id
    }

    #[test]
    fn test_bl_converti_en_facture_credit_compte_une_fois() {
        let mut conn = setup();
        let bl = document_credit(&mut conn, "bl");
        assert_eq!(credit(&conn), 120.0);
        let facture = convert_document_impl(&mut conn, bl, "facture".into(), 1).unwrap();
        assert_eq!(credit(&conn), 120.0);
        let avoir = convert_document_impl(&mut conn, facture, "avoir".into(), 1).unwrap();
        assert_eq!(credit(&conn), 0.0);
        annuler_vente_impl(&mut conn, avoir, None, Some("Test")).unwrap();
        assert_eq!(credit(&conn), 120.0);
    }

    #[test]
    fn test_devis_a_credit_converti_verifie_le_plafond() {
        let mut conn = setup();
        conn.execute("UPDATE clients SET credit_plafond = 100 WHERE id = 1", [])
            .unwrap();
        let devis = crate::commands::ventes::vendre_json(
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
        .unwrap()
        .id;
        conn.execute("UPDATE vente_articles SET quantite = 12, total_ligne = 120, montant_ht = 120 WHERE vente_id = ?1", params![devis]).unwrap();
        conn.execute(
            "UPDATE ventes SET montant_total = 120, montant_ht = 120 WHERE id = ?1",
            params![devis],
        )
        .unwrap();
        assert!(convert_document_impl(&mut conn, devis, "facture".into(), 1)
            .unwrap_err()
            .contains("Plafond"));
        assert_eq!(credit(&conn), 0.0);
        conn.execute("UPDATE clients SET credit_plafond = 500 WHERE id = 1", [])
            .unwrap();
        convert_document_impl(&mut conn, devis, "facture".into(), 1).unwrap();
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
        let facture = r.id;
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
        assert!(annuler_vente_impl(&mut conn, facture, None, Some("Test"))
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
        annuler_vente_impl(&mut conn, r.id, None, Some("Test")).unwrap();
        assert_eq!(points(&conn), 50.0);
    }

    #[test]
    fn test_document_converti_non_annulable() {
        let mut conn = setup();
        let bl = document_credit(&mut conn, "bl");
        let facture = convert_document_impl(&mut conn, bl, "facture".into(), 1).unwrap();
        assert!(annuler_vente_impl(&mut conn, bl, None, Some("Test"))
            .unwrap_err()
            .contains("converti"));
        let stock_avant: f64 = conn
            .query_row("SELECT stock FROM articles WHERE id = 1", [], |r| r.get(0))
            .unwrap();
        annuler_vente_impl(&mut conn, facture, None, Some("Test")).unwrap();
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
