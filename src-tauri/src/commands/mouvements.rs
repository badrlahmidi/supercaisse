use rusqlite::{params, Connection, OptionalExtension};

use super::adjust_article_stock;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Sens {
    Sortie,
    Entree,
}

pub(crate) fn stock_negatif_autorise(conn: &Connection) -> Result<bool, String> {
    let valeur: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'autoriser_stock_negatif'",
            [],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    Ok(valeur.as_deref() == Some("true"))
}

fn designation(conn: &Connection, article_id: i64) -> String {
    conn.query_row(
        "SELECT designation FROM articles WHERE id = ?1",
        params![article_id],
        |r| r.get(0),
    )
    .unwrap_or_else(|_| format!("article {}", article_id))
}

pub(crate) fn retirer_stock(
    conn: &Connection,
    article_id: i64,
    magasin_id: i64,
    quantite: f64,
) -> Result<(), String> {
    if !stock_negatif_autorise(conn)? {
        let disponible: f64 = conn
            .query_row(
                "SELECT COALESCE((SELECT quantite FROM article_stocks WHERE article_id = ?1 AND magasin_id = ?2), 0)",
                params![article_id, magasin_id],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        if disponible - quantite < -1e-9 {
            return Err(format!(
                "Stock insuffisant pour {} : {} disponible(s), {} demandé(s)",
                designation(conn, article_id),
                disponible,
                quantite
            ));
        }
    }
    adjust_article_stock(conn, article_id, magasin_id, -quantite)
}

pub(crate) fn ajuster_stock_variante(
    conn: &Connection,
    variante_id: i64,
    magasin_id: i64,
    delta: f64,
) -> Result<(), String> {
    if delta < 0.0 && !stock_negatif_autorise(conn)? {
        let (disponible, libelle): (f64, String) = conn
            .query_row(
                "SELECT COALESCE((SELECT quantite FROM article_variante_stocks WHERE variante_id = v.id AND magasin_id = ?2), 0),
                        a.designation || ' ' || COALESCE(v.taille, '') || ' ' || COALESCE(v.couleur, '')
                 FROM article_variantes v JOIN articles a ON a.id = v.article_id WHERE v.id = ?1",
                params![variante_id, magasin_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
            .map_err(|e| e.to_string())?
            .ok_or("Variante introuvable")?;
        if disponible + delta < -1e-9 {
            return Err(format!(
                "Stock insuffisant pour {} : {} disponible(s), {} demandé(s)",
                libelle.trim(),
                disponible,
                -delta
            ));
        }
    }
    conn.execute(
        "INSERT INTO article_variante_stocks (variante_id, magasin_id, quantite) VALUES (?1, ?2, ?3)
         ON CONFLICT(variante_id, magasin_id) DO UPDATE SET quantite = quantite + ?3",
        params![variante_id, magasin_id, delta],
    )
    .map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE article_variantes SET stock_dedie = (SELECT COALESCE(SUM(quantite), 0) FROM article_variante_stocks WHERE variante_id = ?1) WHERE id = ?1",
        params![variante_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub(crate) fn consommer_lots(
    conn: &Connection,
    article_id: i64,
    magasin_id: i64,
    quantite: f64,
    vente_id: i64,
) -> Result<(), String> {
    let suivi_lot: bool = conn
        .query_row(
            "SELECT COALESCE(suivi_lot, 0) FROM articles WHERE id = ?1",
            params![article_id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if !suivi_lot {
        return Ok(());
    }
    let lots: Vec<(i64, f64)> = {
        let mut stmt = conn
            .prepare(
                "SELECT id, quantite FROM article_lots
                 WHERE article_id = ?1 AND magasin_id = ?2 AND quantite > 0
                   AND (date_peremption IS NULL OR date(date_peremption) >= date('now', 'localtime'))
                 ORDER BY (date_peremption IS NULL), date(date_peremption), date_reception, id",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![article_id, magasin_id], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| e.to_string())?
    };
    let mut reste = quantite;
    for (lot_id, disponible) in lots {
        if reste <= 1e-9 {
            break;
        }
        let pris = reste.min(disponible);
        conn.execute(
            "UPDATE article_lots SET quantite = quantite - ?1 WHERE id = ?2",
            params![pris, lot_id],
        )
        .map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO vente_lots (vente_id, lot_id, quantite) VALUES (?1, ?2, ?3)",
            params![vente_id, lot_id, pris],
        )
        .map_err(|e| e.to_string())?;
        reste -= pris;
    }
    Ok(())
}

pub(crate) fn proprietaire_lots(conn: &Connection, vente_id: i64) -> Result<i64, String> {
    let a_des_lots: bool = conn
        .query_row(
            "SELECT COUNT(*) > 0 FROM vente_lots WHERE vente_id = ?1",
            params![vente_id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if a_des_lots {
        return Ok(vente_id);
    }
    let source: Option<(i64, String)> = conn
        .query_row(
            "SELECT s.id, COALESCE(s.dtype, 'facture') FROM ventes v JOIN ventes s ON s.id = v.source_vente_id WHERE v.id = ?1",
            params![vente_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    match source {
        Some((id, dtype)) if matches!(dtype.as_str(), "bl" | "facture") => {
            proprietaire_lots(conn, id)
        }
        _ => Ok(vente_id),
    }
}

pub(crate) fn inverser_lots(
    conn: &Connection,
    proprietaire: i64,
    trace: Option<i64>,
) -> Result<(), String> {
    let lignes: Vec<(i64, f64)> = {
        let mut stmt = conn
            .prepare(
                "SELECT lot_id, SUM(quantite) FROM vente_lots WHERE vente_id = ?1 GROUP BY lot_id",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![proprietaire], |r| Ok((r.get(0)?, r.get(1)?)))
            .map_err(|e| e.to_string())?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| e.to_string())?
    };
    for (lot_id, quantite) in lignes {
        conn.execute(
            "UPDATE article_lots SET quantite = quantite + ?1 WHERE id = ?2",
            params![quantite, lot_id],
        )
        .map_err(|e| e.to_string())?;
        if let Some(id) = trace {
            conn.execute(
                "INSERT INTO vente_lots (vente_id, lot_id, quantite) VALUES (?1, ?2, ?3)",
                params![id, lot_id, -quantite],
            )
            .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn composants(conn: &Connection, article_id: i64) -> Result<Vec<(i64, f64)>, String> {
    let mut stmt = conn
        .prepare("SELECT composant_id, quantite FROM article_composants WHERE article_id = ?1")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![article_id], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|e| e.to_string())?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| e.to_string())
}

fn journaliser(
    conn: &Connection,
    article_id: i64,
    quantite: f64,
    sens: Sens,
    vente_id: i64,
    reference: &str,
    magasin_id: i64,
) -> Result<(), String> {
    let mtype = match sens {
        Sens::Sortie => "sortie",
        Sens::Entree => "entree",
    };
    conn.execute(
        "INSERT INTO mouvements_stock (article_id, quantite, mtype, reference_id, reference_type, magasin_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![article_id, quantite, mtype, vente_id, reference, magasin_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub(crate) struct LigneStock {
    pub article_id: i64,
    pub variante_id: Option<i64>,
    pub quantite: f64,
}

pub(crate) fn mouvement_ligne(
    conn: &Connection,
    sens: Sens,
    ligne: &LigneStock,
    magasin_id: i64,
    vente_id: i64,
    reference: &str,
    lots_fefo: bool,
) -> Result<(), String> {
    let signe = if sens == Sens::Sortie { -1.0 } else { 1.0 };
    if let Some(vid) = ligne.variante_id {
        ajuster_stock_variante(conn, vid, magasin_id, signe * ligne.quantite)?;
        return journaliser(
            conn,
            ligne.article_id,
            ligne.quantite,
            sens,
            vente_id,
            &format!("{}_variante", reference),
            magasin_id,
        );
    }
    let kit = composants(conn, ligne.article_id)?;
    let (cibles, reference) = if kit.is_empty() {
        (
            vec![(ligne.article_id, ligne.quantite)],
            reference.to_string(),
        )
    } else {
        (
            kit.into_iter()
                .map(|(c, q)| (c, q * ligne.quantite))
                .collect(),
            format!("{}_kit", reference),
        )
    };
    for (article_id, quantite) in cibles {
        match sens {
            Sens::Sortie => {
                retirer_stock(conn, article_id, magasin_id, quantite)?;
                if lots_fefo {
                    consommer_lots(conn, article_id, magasin_id, quantite, vente_id)?;
                }
            }
            Sens::Entree => adjust_article_stock(conn, article_id, magasin_id, quantite)?,
        }
        journaliser(
            conn, article_id, quantite, sens, vente_id, &reference, magasin_id,
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::magasins::{create_transfert_impl, validate_transfert_impl};
    use crate::commands::ventes::{annuler_vente_impl, convert_document_impl, create_vente_impl};
    use serde_json::json;

    fn setup() -> Connection {
        let conn = crate::db::init_db(":memory:").unwrap();
        conn.execute_batch(
            "
            INSERT INTO magasins (id, nom) VALUES (2, 'Annexe');
            INSERT INTO articles (id, designation, prix_vente, tva) VALUES (1, 'Riz', 10, 0);
            INSERT INTO articles (id, designation, prix_vente, tva, suivi_lot) VALUES (2, 'Yaourt', 10, 0, 1);
            INSERT INTO articles (id, designation, prix_vente, tva, est_kit) VALUES (3, 'Panier', 30, 0, 1);
            INSERT INTO article_composants (article_id, composant_id, quantite) VALUES (3, 1, 2);
            UPDATE settings SET value = 'false' WHERE key = 'fidelite_actif';
        ",
        )
        .unwrap();
        adjust_article_stock(&conn, 1, 1, 3.0).unwrap();
        conn
    }

    fn vendre(
        conn: &mut Connection,
        article: i64,
        variante: Option<i64>,
        qte: f64,
        magasin: i64,
        dtype: &str,
    ) -> Result<i64, String> {
        let prix: f64 = conn
            .query_row(
                "SELECT prix_vente FROM articles WHERE id = ?1",
                params![article],
                |r| r.get(0),
            )
            .unwrap();
        let r = create_vente_impl(
            conn,
            None,
            Some(1),
            vec![json!({ "article_id": article, "variante_id": variante, "quantite": qte })],
            None,
            "especes".into(),
            Some(vec![json!({ "mode": "especes", "montant": prix * qte })]),
            Some(dtype.into()),
            None,
            Some(magasin),
        )?;
        Ok(r["id"].as_i64().unwrap())
    }

    fn stock(conn: &Connection, article: i64, magasin: i64) -> f64 {
        conn.query_row(
            "SELECT COALESCE((SELECT quantite FROM article_stocks WHERE article_id = ?1 AND magasin_id = ?2), 0)",
            params![article, magasin],
            |r| r.get(0),
        )
        .unwrap()
    }

    fn lot(conn: &Connection, id: i64) -> f64 {
        conn.query_row(
            "SELECT quantite FROM article_lots WHERE id = ?1",
            params![id],
            |r| r.get(0),
        )
        .unwrap()
    }

    #[test]
    fn test_stock_negatif_refuse_par_defaut() {
        let mut conn = setup();
        let err = vendre(&mut conn, 1, None, 5.0, 1, "facture").unwrap_err();
        assert!(err.contains("Stock insuffisant pour Riz"), "{err}");
        let nb: i64 = conn
            .query_row("SELECT COUNT(*) FROM ventes", [], |r| r.get(0))
            .unwrap();
        assert_eq!(nb, 0);
        assert_eq!(stock(&conn, 1, 1), 3.0);
        vendre(&mut conn, 1, None, 3.0, 1, "facture").unwrap();
        assert_eq!(stock(&conn, 1, 1), 0.0);
    }

    #[test]
    fn test_stock_negatif_autorise_par_parametre() {
        let mut conn = setup();
        conn.execute(
            "UPDATE settings SET value = 'true' WHERE key = 'autoriser_stock_negatif'",
            [],
        )
        .unwrap();
        vendre(&mut conn, 1, None, 5.0, 1, "facture").unwrap();
        assert_eq!(stock(&conn, 1, 1), -2.0);
    }

    #[test]
    fn test_stock_controle_par_magasin_et_devis_libres() {
        let mut conn = setup();
        assert!(vendre(&mut conn, 1, None, 1.0, 2, "facture").is_err());
        vendre(&mut conn, 1, None, 50.0, 2, "devis").unwrap();
    }

    #[test]
    fn test_kit_controle_les_composants() {
        let mut conn = setup();
        assert!(vendre(&mut conn, 3, None, 2.0, 1, "facture")
            .unwrap_err()
            .contains("Riz"));
        vendre(&mut conn, 3, None, 1.0, 1, "facture").unwrap();
        assert_eq!(stock(&conn, 1, 1), 1.0);
    }

    #[test]
    fn test_variantes_par_magasin() {
        let mut conn = setup();
        conn.execute("INSERT INTO article_variantes (id, article_id, taille, stock_dedie) VALUES (10, 1, 'M', 0)", [])
            .unwrap();
        ajuster_stock_variante(&conn, 10, 1, 5.0).unwrap();
        let v = vendre(&mut conn, 1, Some(10), 2.0, 1, "facture").unwrap();
        let dedie: f64 = conn
            .query_row(
                "SELECT stock_dedie FROM article_variantes WHERE id = 10",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(dedie, 3.0);
        assert!(vendre(&mut conn, 1, Some(10), 1.0, 2, "facture").is_err());
        assert!(vendre(&mut conn, 1, Some(10), 4.0, 1, "facture").is_err());
        annuler_vente_impl(&mut conn, v, None, None).unwrap();
        let dedie: f64 = conn
            .query_row(
                "SELECT stock_dedie FROM article_variantes WHERE id = 10",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(dedie, 5.0);
        assert_eq!(stock(&conn, 1, 1), 3.0);
    }

    fn lots_yaourt(conn: &Connection) {
        conn.execute_batch(
            "
            INSERT INTO article_lots (id, article_id, magasin_id, numero_lot, date_peremption, quantite, date_reception)
            VALUES (1, 2, 1, 'A', '2099-01-01', 3, '2026-01-01'),
                   (2, 2, 1, 'B', '2098-06-01', 2, '2026-01-02'),
                   (3, 2, 1, 'C', '2000-01-01', 5, '2025-01-01'),
                   (4, 2, 1, 'D', NULL, 4, '2025-06-01');
        ",
        )
        .unwrap();
        adjust_article_stock(conn, 2, 1, 14.0).unwrap();
    }

    #[test]
    fn test_fefo_et_annulation() {
        let mut conn = setup();
        lots_yaourt(&conn);
        let v = vendre(&mut conn, 2, None, 4.0, 1, "facture").unwrap();
        assert_eq!(
            (lot(&conn, 2), lot(&conn, 1), lot(&conn, 3), lot(&conn, 4)),
            (0.0, 1.0, 5.0, 4.0)
        );
        vendre(&mut conn, 2, None, 3.0, 1, "facture").unwrap();
        assert_eq!((lot(&conn, 1), lot(&conn, 4)), (0.0, 2.0));
        annuler_vente_impl(&mut conn, v, None, None).unwrap();
        assert_eq!(
            (lot(&conn, 2), lot(&conn, 1), lot(&conn, 4)),
            (2.0, 2.0, 2.0)
        );
    }

    #[test]
    fn test_avoir_restitue_les_lots() {
        let mut conn = setup();
        lots_yaourt(&conn);
        let f = vendre(&mut conn, 2, None, 3.0, 1, "facture").unwrap();
        assert_eq!((lot(&conn, 2), lot(&conn, 1)), (0.0, 2.0));
        let avoir = convert_document_impl(&mut conn, f, "avoir".into()).unwrap();
        assert_eq!((lot(&conn, 2), lot(&conn, 1)), (2.0, 3.0));
        annuler_vente_impl(&mut conn, avoir, None, None).unwrap();
        assert_eq!((lot(&conn, 2), lot(&conn, 1)), (0.0, 2.0));
    }

    #[test]
    fn test_bl_converti_puis_facture_annulee_restitue_les_lots() {
        let mut conn = setup();
        lots_yaourt(&conn);
        let bl = vendre(&mut conn, 2, None, 2.0, 1, "bl").unwrap();
        assert_eq!(lot(&conn, 2), 0.0);
        let f = convert_document_impl(&mut conn, bl, "facture".into()).unwrap();
        assert_eq!(lot(&conn, 2), 0.0);
        annuler_vente_impl(&mut conn, f, None, None).unwrap();
        assert_eq!(lot(&conn, 2), 2.0);
        assert_eq!(stock(&conn, 2, 1), 14.0);
    }

    #[test]
    fn test_transferts() {
        let mut conn = setup();
        assert!(create_transfert_impl(
            &mut conn,
            1,
            1,
            None,
            vec![json!({ "article_id": 1, "quantite": 1 })]
        )
        .is_err());
        assert!(create_transfert_impl(
            &mut conn,
            1,
            2,
            None,
            vec![json!({ "article_id": 1, "quantite": 0 })]
        )
        .is_err());
        assert!(create_transfert_impl(
            &mut conn,
            1,
            2,
            None,
            vec![json!({ "article_id": 99, "quantite": 1 })]
        )
        .is_err());
        assert!(create_transfert_impl(
            &mut conn,
            1,
            9,
            None,
            vec![json!({ "article_id": 1, "quantite": 1 })]
        )
        .is_err());
        let trop = create_transfert_impl(
            &mut conn,
            1,
            2,
            None,
            vec![json!({ "article_id": 1, "quantite": 5 })],
        )
        .unwrap();
        assert!(validate_transfert_impl(&mut conn, trop, None)
            .unwrap_err()
            .contains("Stock insuffisant"));
        let ok = create_transfert_impl(
            &mut conn,
            1,
            2,
            None,
            vec![json!({ "article_id": 1, "quantite": 2 })],
        )
        .unwrap();
        validate_transfert_impl(&mut conn, ok, None).unwrap();
        assert_eq!((stock(&conn, 1, 1), stock(&conn, 1, 2)), (1.0, 2.0));
    }

    #[test]
    fn test_migration_stock_des_variantes() {
        let path = std::env::temp_dir().join(format!(
            "supercaisse_test_variantes_{}_{}.db",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path_str = path.to_string_lossy().to_string();
        {
            let conn = crate::db::init_db(&path_str).unwrap();
            conn.execute_batch(
                "
                INSERT INTO articles (id, designation) VALUES (1, 'T-shirt');
                INSERT INTO article_variantes (id, article_id, taille, stock_dedie) VALUES (1, 1, 'M', 7);
                DROP TABLE article_variante_stocks;
            ",
            )
            .unwrap();
        }
        let conn = crate::db::init_db(&path_str).unwrap();
        let (magasin, quantite): (i64, f64) = conn
            .query_row(
                "SELECT magasin_id, quantite FROM article_variante_stocks WHERE variante_id = 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!((magasin, quantite), (1, 7.0));
        drop(conn);
        let conn = crate::db::init_db(&path_str).unwrap();
        let nb: i64 = conn
            .query_row("SELECT COUNT(*) FROM article_variante_stocks", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(nb, 1);
        drop(conn);
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{}", path_str, suffix));
        }
    }
}
