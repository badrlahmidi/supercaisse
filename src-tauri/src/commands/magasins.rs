use crate::db::*;
use crate::session::{autoriser, Acces, AuthState};
use rusqlite::{params, Connection};
use tauri::State;

use super::mouvements::retirer_stock;
use super::{adjust_article_stock, log_audit};

#[tauri::command(async)]
pub fn get_magasins(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
) -> Result<Vec<super::contrats::Magasin>, String> {
    let conn = db.lecture()?;
    let _me = autoriser(&auth, &conn, &token, Acces::Connecte)?;
    let mut stmt = conn
        .prepare("SELECT id, nom, adresse FROM magasins ORDER BY id")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok(super::contrats::Magasin {
                id: row.get(0)?,
                nom: row.get(1)?,
                adresse: row.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

#[tauri::command(async)]
pub fn add_magasin(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    nom: String,
    adresse: Option<String>,
) -> Result<i64, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let me = autoriser(&auth, &conn, &token, Acces::Module("magasins", "creer"))?;
    conn.execute(
        "INSERT INTO magasins (nom, adresse) VALUES (?1, ?2)",
        params![nom, adresse],
    )
    .map_err(|e| e.to_string())?;
    let id = conn.last_insert_rowid();
    super::tracer_creation(&conn, "magasins", id, Some(me.user_id))?;
    Ok(id)
}

#[tauri::command(async)]
pub fn update_magasin(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    id: i64,
    nom: String,
    adresse: Option<String>,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let me = autoriser(&auth, &conn, &token, Acces::Module("magasins", "modifier"))?;
    conn.execute(
        "UPDATE magasins SET nom=?1, adresse=?2 WHERE id=?3",
        params![nom, adresse, id],
    )
    .map_err(|e| e.to_string())?;
    super::tracer_modification(&conn, "magasins", id, Some(me.user_id))?;
    Ok(())
}

#[tauri::command(async)]
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
        .map_err(|e| e.to_string())?;
    if count <= 1 {
        return Err("Impossible de supprimer le dernier magasin".to_string());
    }
    let has_sessions: bool = conn
        .query_row(
            "SELECT count(*) > 0 FROM sessions_caisse WHERE magasin_id = ?1",
            params![id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
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
        .map_err(|e| super::erreur_suppression(e, "ce magasin"))?;
    tx.execute(
        "UPDATE articles SET stock = COALESCE((SELECT SUM(quantite) FROM article_stocks WHERE article_id = articles.id), 0)",
        [],
    ).map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command(async)]
pub fn get_stats_magasins(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
) -> Result<Vec<super::contrats::StatsMagasin>, String> {
    let conn = db.lecture()?;
    let _me = autoriser(&auth, &conn, &token, Acces::Admin)?;
    get_stats_magasins_impl(&conn)
}

pub(crate) fn get_stats_magasins_impl(
    conn: &Connection,
) -> Result<Vec<super::contrats::StatsMagasin>, String> {
    let mut stmt = conn.prepare(
        "SELECT
             m.id,
             m.nom,
             m.adresse,
             COALESCE(v.ca_mois, 0.0) AS ca_mois,
             COALESCE(v.nb_ventes, 0) AS nb_ventes,
             COALESCE(v.nb_clients_actifs, 0) AS nb_clients_actifs,
             COALESCE(s.valeur_stock, 0.0) AS valeur_stock
         FROM magasins m
         LEFT JOIN (
             SELECT
                 magasin_id,
                 SUM(montant_total - montant_remise) AS ca_mois,
                 COUNT(CASE WHEN dtype != 'avoir' THEN 1 END) AS nb_ventes,
                 COUNT(DISTINCT CASE WHEN dtype != 'avoir' AND client_id IS NOT NULL THEN client_id END) AS nb_clients_actifs
             FROM ventes
             WHERE statut != 'annulee'
               AND (COALESCE(dtype, 'facture') IN ('facture', 'avoir')
                    OR (dtype = 'bl' AND NOT EXISTS (
                            SELECT 1 FROM ventes vf
                            WHERE vf.source_vente_id = ventes.id AND vf.dtype = 'facture')))
               AND strftime('%Y-%m', date) = strftime('%Y-%m', 'now', 'localtime')
             GROUP BY magasin_id
         ) v ON v.magasin_id = m.id
         LEFT JOIN (
             SELECT
                 as2.magasin_id,
                 SUM(as2.quantite * a.prix_achat) AS valeur_stock
             FROM article_stocks as2
             JOIN articles a ON a.id = as2.article_id
             WHERE as2.quantite > 0 AND a.actif = 1
             GROUP BY as2.magasin_id
         ) s ON s.magasin_id = m.id
         ORDER BY m.id",
    )
    .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok(super::contrats::StatsMagasin {
                id: row.get(0)?,
                nom: row.get(1)?,
                adresse: row.get(2)?,
                ca_mois: row.get(3)?,
                nb_ventes: row.get(4)?,
                nb_clients_actifs: row.get(5)?,
                valeur_stock: row.get(6)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> Connection {
        let conn = crate::db::init_db(":memory:").unwrap();
        conn.execute_batch(
            "UPDATE magasins SET nom = 'Centre', adresse = 'Rue A' WHERE id = 1;
             INSERT INTO magasins (nom, adresse) VALUES ('Nord', NULL);
             INSERT INTO articles (designation, prix_vente, prix_achat, tva) VALUES ('Café', 50.0, 20.0, 20);
             INSERT INTO articles (designation, prix_vente, prix_achat, tva) VALUES ('Thé',  30.0, 10.0, 20);
             INSERT INTO clients (nom) VALUES ('Alice');
             INSERT INTO clients (nom) VALUES ('Bob');",
        )
        .unwrap();
        conn
    }

    fn magasin_ids(conn: &Connection) -> (i64, i64) {
        let id1: i64 = conn
            .query_row("SELECT id FROM magasins WHERE nom = 'Centre'", [], |r| {
                r.get(0)
            })
            .unwrap();
        let id2: i64 = conn
            .query_row("SELECT id FROM magasins WHERE nom = 'Nord'", [], |r| {
                r.get(0)
            })
            .unwrap();
        (id1, id2)
    }

    fn article_ids(conn: &Connection) -> (i64, i64) {
        let a1: i64 = conn
            .query_row(
                "SELECT id FROM articles WHERE designation = 'Café'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let a2: i64 = conn
            .query_row(
                "SELECT id FROM articles WHERE designation = 'Thé'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        (a1, a2)
    }

    fn client_ids(conn: &Connection) -> (i64, i64) {
        let c1: i64 = conn
            .query_row(
                "SELECT id FROM clients WHERE nom = 'Alice'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let c2: i64 = conn
            .query_row(
                "SELECT id FROM clients WHERE nom = 'Bob'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        (c1, c2)
    }

    #[test]
    fn test_stats_magasin_sans_ventes_renvoie_zeros() {
        let conn = setup();
        let stats = get_stats_magasins_impl(&conn).unwrap();
        assert_eq!(stats.len(), 2);
        let centre = stats.iter().find(|s| s.nom == "Centre").unwrap();
        assert_eq!(centre.adresse, Some("Rue A".into()));
        assert_eq!(centre.ca_mois, 0.0);
        assert_eq!(centre.nb_ventes, 0);
        assert_eq!(centre.nb_clients_actifs, 0);
        assert_eq!(centre.valeur_stock, 0.0);
    }

    #[test]
    fn test_stats_magasin_ventes_et_stock() {
        let conn = setup();
        let (m1, _) = magasin_ids(&conn);
        let (a1, a2) = article_ids(&conn);
        let (c1, c2) = client_ids(&conn);
        conn.execute_batch(&format!(
            "INSERT INTO ventes (magasin_id, montant_total, montant_remise, dtype, statut, client_id,
                                 date, mode_paiement)
             VALUES ({m1}, 200.0, 0.0, 'facture', 'validee', {c1}, strftime('%Y-%m-%dT%H:%M:%S', 'now', 'localtime'), 'especes');
             INSERT INTO ventes (magasin_id, montant_total, montant_remise, dtype, statut, client_id,
                                 date, mode_paiement)
             VALUES ({m1}, 100.0, 0.0, 'facture', 'validee', {c2}, strftime('%Y-%m-%dT%H:%M:%S', 'now', 'localtime'), 'especes');
             INSERT INTO article_stocks (article_id, magasin_id, quantite) VALUES ({a1}, {m1}, 10);
             INSERT INTO article_stocks (article_id, magasin_id, quantite) VALUES ({a2}, {m1},  5);",
        ))
        .unwrap();
        let stats = get_stats_magasins_impl(&conn).unwrap();
        let centre = stats.iter().find(|s| s.nom == "Centre").unwrap();
        assert_eq!(centre.ca_mois, 300.0);
        assert_eq!(centre.nb_ventes, 2);
        assert_eq!(centre.nb_clients_actifs, 2);
        assert_eq!(centre.valeur_stock, 10.0 * 20.0 + 5.0 * 10.0);
        let nord = stats.iter().find(|s| s.nom == "Nord").unwrap();
        assert_eq!(nord.ca_mois, 0.0);
        assert_eq!(nord.valeur_stock, 0.0);
    }

    #[test]
    fn test_avoir_deduit_et_remise_appliquee() {
        let conn = setup();
        let (m1, _) = magasin_ids(&conn);
        conn.execute_batch(&format!(
            "INSERT INTO ventes (magasin_id, montant_total, montant_remise, dtype, statut,
                                 date, mode_paiement)
             VALUES ({m1}, 500.0, 50.0, 'facture', 'validee', strftime('%Y-%m-%dT%H:%M:%S', 'now', 'localtime'), 'especes');
             INSERT INTO ventes (magasin_id, montant_total, montant_remise, dtype, statut,
                                 date, mode_paiement)
             VALUES ({m1}, -100.0, -10.0, 'avoir', 'validee', strftime('%Y-%m-%dT%H:%M:%S', 'now', 'localtime'), 'especes');",
        ))
        .unwrap();
        let stats = get_stats_magasins_impl(&conn).unwrap();
        let centre = stats.iter().find(|s| s.nom == "Centre").unwrap();
        assert_eq!(centre.ca_mois, (500.0 - 50.0) + (-100.0 - (-10.0)));
        assert_eq!(centre.nb_ventes, 1);
    }

    #[test]
    fn test_ventes_annulees_et_devis_exclus() {
        let conn = setup();
        let (m1, _) = magasin_ids(&conn);
        let (c1, _) = client_ids(&conn);
        conn.execute_batch(&format!(
            "INSERT INTO ventes (magasin_id, montant_total, dtype, statut, client_id,
                                 date, mode_paiement)
             VALUES ({m1}, 300.0, 'facture', 'annulee', {c1}, strftime('%Y-%m-%dT%H:%M:%S', 'now', 'localtime'), 'especes');
             INSERT INTO ventes (magasin_id, montant_total, dtype, statut, client_id,
                                 date, mode_paiement)
             VALUES ({m1}, 200.0, 'devis', 'validee', {c1}, strftime('%Y-%m-%dT%H:%M:%S', 'now', 'localtime'), 'especes');
             INSERT INTO ventes (magasin_id, montant_total, dtype, statut, client_id,
                                 date, mode_paiement)
             VALUES ({m1}, 150.0, 'commande', 'validee', {c1}, strftime('%Y-%m-%dT%H:%M:%S', 'now', 'localtime'), 'especes');",
        ))
        .unwrap();
        let stats = get_stats_magasins_impl(&conn).unwrap();
        let centre = stats.iter().find(|s| s.nom == "Centre").unwrap();
        assert_eq!(centre.ca_mois, 0.0);
        assert_eq!(centre.nb_ventes, 0);
        assert_eq!(centre.nb_clients_actifs, 0);
    }
}

#[tauri::command(async)]
pub fn get_transferts(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
) -> Result<Vec<super::contrats::TransfertResume>, String> {
    let conn = db.lecture()?;
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
            Ok(super::contrats::TransfertResume {
                id: row.get(0)?,
                date: row.get(1)?,
                statut: row.get(2)?,
                source_nom: row.get(3)?,
                dest_nom: row.get(4)?,
                utilisateur_nom: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

#[tauri::command(async)]
pub fn get_stock_par_magasin(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    magasin_id: i64,
) -> Result<Vec<super::contrats::StockMagasin>, String> {
    let conn = db.lecture()?;
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
            Ok(super::contrats::StockMagasin {
                id: row.get(0)?,
                designation: row.get(1)?,
                code_barre: row.get(2)?,
                stock: row.get(3)?,
                stock_alerte: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

#[tauri::command(async)]
pub fn create_transfert(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    source_id: i64,
    dest_id: i64,
    articles: Vec<super::contrats::LigneTransfertSaisie>,
) -> Result<i64, String> {
    let mut conn = db.conn.lock().map_err(|e| e.to_string())?;
    let me = autoriser(&auth, &conn, &token, Acces::Module("stock", "creer"))?;
    create_transfert_impl(&mut conn, source_id, dest_id, Some(me.user_id), articles)
}

pub(crate) fn create_transfert_impl(
    conn: &mut Connection,
    source_id: i64,
    dest_id: i64,
    utilisateur_id: Option<i64>,
    articles: Vec<super::contrats::LigneTransfertSaisie>,
) -> Result<i64, String> {
    if source_id == dest_id {
        return Err(
            "Le magasin source et le magasin destination doivent être différents".to_string(),
        );
    }
    if articles.is_empty() {
        return Err("Le transfert ne contient aucun article".to_string());
    }
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    for magasin in [source_id, dest_id] {
        let existe: bool = tx
            .query_row(
                "SELECT COUNT(*) > 0 FROM magasins WHERE id = ?1",
                params![magasin],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        if !existe {
            return Err(format!("Magasin {} introuvable", magasin));
        }
    }

    tx.execute(
        "INSERT INTO transferts_stock (source_id, dest_id, utilisateur_id, statut) VALUES (?1, ?2, ?3, 'en_attente')",
        params![source_id, dest_id, utilisateur_id]
    ).map_err(|e| e.to_string())?;

    let transfert_id = tx.last_insert_rowid();

    for a in articles {
        let article_id = a.article_id;
        let quantite = a.quantite;
        if !quantite.is_finite() || quantite <= 0.0 {
            return Err(format!(
                "Quantité invalide pour l'article {} : {}",
                article_id, quantite
            ));
        }
        let existe: bool = tx
            .query_row(
                "SELECT COUNT(*) > 0 FROM articles WHERE id = ?1",
                params![article_id],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        if !existe {
            return Err(format!("Article {} introuvable", article_id));
        }
        tx.execute(
            "INSERT INTO transfert_lignes (transfert_id, article_id, quantite) VALUES (?1, ?2, ?3)",
            params![transfert_id, article_id, quantite],
        )
        .map_err(|e| e.to_string())?;
    }

    tx.commit().map_err(|e| e.to_string())?;
    Ok(transfert_id)
}

#[tauri::command(async)]
pub fn validate_transfert(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    transfert_id: i64,
) -> Result<(), String> {
    let mut conn = db.conn.lock().map_err(|e| e.to_string())?;
    let me = autoriser(&auth, &conn, &token, Acces::Module("stock", "modifier"))?;
    super::tracer(
        &format!("Validation du transfert {}", transfert_id),
        validate_transfert_impl(&mut conn, transfert_id, Some(me.user_id)),
    )
}

pub(crate) fn validate_transfert_impl(
    conn: &mut Connection,
    transfert_id: i64,
    auteur: Option<i64>,
) -> Result<(), String> {
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

        retirer_stock(&tx, article_id, source_id, qte)?;
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
        auteur,
        "valider_transfert",
        &format!(
            "Validation transfert #{} (magasin {} → {})",
            transfert_id, source_id, dest_id
        ),
        Some("transfert"),
        Some(transfert_id),
    )?;

    drop(stmt);
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}
