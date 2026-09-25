use crate::db::*;
use crate::session::{autoriser, Acces, AuthState};
use rusqlite::{params, Connection, OptionalExtension};
use tauri::State;

use super::{adjust_article_stock, log_audit};

#[tauri::command]
pub fn create_inventaire(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    magasin_id: i64,
) -> Result<serde_json::Value, String> {
    let mut conn = db.conn.lock().map_err(|e| e.to_string())?;
    let me = autoriser(&auth, &conn, &token, Acces::Module("inventaire", "creer"))?;
    create_inventaire_impl(&mut conn, magasin_id, me.user_id)
}

pub(crate) fn create_inventaire_impl(
    conn: &mut Connection,
    magasin_id: i64,
    utilisateur_id: i64,
) -> Result<serde_json::Value, String> {
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let en_cours: Option<i64> = tx
        .query_row(
            "SELECT id FROM inventaires WHERE magasin_id = ?1 AND statut = 'en_cours' LIMIT 1",
            params![magasin_id],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    if let Some(id) = en_cours {
        return Err(format!("L'inventaire #{} est déjà en cours pour ce magasin : validez-le avant d'en créer un autre", id));
    }
    tx.execute(
        "INSERT INTO inventaires (magasin_id, utilisateur_id) VALUES (?1, ?2)",
        params![magasin_id, utilisateur_id],
    )
    .map_err(|e| e.to_string())?;
    let inv_id = tx.last_insert_rowid();
    let nb = tx
        .execute(
            "INSERT INTO inventaire_lignes (inventaire_id, article_id, stock_theorique)
         SELECT ?1, a.id, COALESCE(s.quantite, 0)
         FROM articles a
         LEFT JOIN article_stocks s ON s.article_id = a.id AND s.magasin_id = ?2
         WHERE a.actif = 1
         ORDER BY a.designation",
            params![inv_id, magasin_id],
        )
        .map_err(|e| e.to_string())?;
    log_audit(
        &tx,
        Some(utilisateur_id),
        "creer_inventaire",
        &format!("Inventaire #{} créé ({} articles)", inv_id, nb),
        Some("inventaire"),
        Some(inv_id),
    );
    tx.commit().map_err(|e| e.to_string())?;
    Ok(serde_json::json!({ "id": inv_id, "nb_articles": nb }))
}

#[tauri::command]
pub fn get_inventaire(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    inventaire_id: i64,
) -> Result<serde_json::Value, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let _me = autoriser(&auth, &conn, &token, Acces::Module("inventaire", "voir"))?;

    let (date_debut, statut, magasin_id): (String, String, i64) = conn
        .query_row(
            "SELECT date_debut, statut, magasin_id FROM inventaires WHERE id = ?1",
            params![inventaire_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .map_err(|_| "Inventaire introuvable".to_string())?;

    let mut stmt = conn.prepare(
        "SELECT il.id, il.article_id, a.designation, a.code_barre, il.stock_theorique, il.stock_compte, il.ecart
         FROM inventaire_lignes il
         JOIN articles a ON a.id = il.article_id
         WHERE il.inventaire_id = ?1
         ORDER BY a.designation"
    ).map_err(|e| e.to_string())?;
    let lignes = stmt
        .query_map(params![inventaire_id], |r| {
            Ok(serde_json::json!({
                "id": r.get::<_, i64>(0)?,
                "article_id": r.get::<_, i64>(1)?,
                "designation": r.get::<_, String>(2)?,
                "code_barre": r.get::<_, Option<String>>(3)?,
                "stock_theorique": r.get::<_, f64>(4)?,
                "stock_compte": r.get::<_, Option<f64>>(5)?,
                "ecart": r.get::<_, Option<f64>>(6)?
            }))
        })
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect::<Vec<_>>();

    Ok(serde_json::json!({
        "id": inventaire_id,
        "date_debut": date_debut,
        "statut": statut,
        "magasin_id": magasin_id,
        "lignes": lignes,
    }))
}

#[tauri::command]
pub fn get_inventaires(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
) -> Result<Vec<serde_json::Value>, String> {
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
    let rows = stmt
        .query_map([], |r| {
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
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_inventaire_ligne(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    ligne_id: i64,
    stock_compte: f64,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let _me = autoriser(
        &auth,
        &conn,
        &token,
        Acces::Module("inventaire", "modifier"),
    )?;
    update_inventaire_ligne_impl(&conn, ligne_id, stock_compte)
}

pub(crate) fn update_inventaire_ligne_impl(
    conn: &Connection,
    ligne_id: i64,
    stock_compte: f64,
) -> Result<(), String> {
    if !stock_compte.is_finite() || stock_compte < 0.0 {
        return Err(format!("Quantité comptée invalide : {}", stock_compte));
    }
    let (article_id, magasin_id, statut): (i64, i64, String) = conn.query_row(
        "SELECT il.article_id, i.magasin_id, i.statut FROM inventaire_lignes il JOIN inventaires i ON i.id = il.inventaire_id WHERE il.id = ?1",
        params![ligne_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    ).optional().map_err(|e| e.to_string())?.ok_or("Ligne introuvable")?;
    if statut != "en_cours" {
        return Err(
            "Cet inventaire est déjà validé : la ligne ne peut plus être modifiée".to_string(),
        );
    }
    let stock_actuel: f64 = conn.query_row(
        "SELECT COALESCE((SELECT quantite FROM article_stocks WHERE article_id = ?1 AND magasin_id = ?2), 0)",
        params![article_id, magasin_id], |r| r.get(0),
    ).map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE inventaire_lignes SET stock_theorique = ?1, stock_compte = ?2, ecart = ?3 WHERE id = ?4",
        params![stock_actuel, stock_compte, stock_compte - stock_actuel, ligne_id],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn valider_inventaire(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    inventaire_id: i64,
) -> Result<(), String> {
    let mut conn = db.conn.lock().map_err(|e| e.to_string())?;
    let me = autoriser(
        &auth,
        &conn,
        &token,
        Acces::Module("inventaire", "modifier"),
    )?;
    valider_inventaire_impl(&mut conn, inventaire_id, me.user_id)
}

pub(crate) fn valider_inventaire_impl(
    conn: &mut Connection,
    inventaire_id: i64,
    utilisateur_id: i64,
) -> Result<(), String> {
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let (statut, magasin_id): (String, i64) = tx
        .query_row(
            "SELECT statut, magasin_id FROM inventaires WHERE id = ?1",
            params![inventaire_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()
        .map_err(|e| e.to_string())?
        .ok_or("Inventaire introuvable")?;
    if statut != "en_cours" {
        return Err("Cet inventaire est déjà validé".to_string());
    }

    let ajustements: Vec<(i64, f64)> = {
        let mut stmt = tx.prepare(
            "SELECT article_id, ecart FROM inventaire_lignes WHERE inventaire_id = ?1 AND stock_compte IS NOT NULL AND ecart != 0"
        ).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![inventaire_id], |r| Ok((r.get(0)?, r.get(1)?)))
            .map_err(|e| e.to_string())?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| e.to_string())?
    };

    for (article_id, ecart) in &ajustements {
        adjust_article_stock(&tx, *article_id, magasin_id, *ecart)?;
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

    log_audit(
        &tx,
        Some(utilisateur_id),
        "valider_inventaire",
        &format!(
            "Inventaire #{} validé ({} écarts appliqués)",
            inventaire_id,
            ajustements.len()
        ),
        Some("inventaire"),
        Some(inventaire_id),
    );

    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::ventes::create_vente_impl;
    use serde_json::json;

    fn setup() -> Connection {
        let conn = crate::db::init_db(":memory:").unwrap();
        conn.execute(
            "INSERT INTO articles (id, designation, prix_vente, tva) VALUES (1, 'Huile', 10, 0)",
            [],
        )
        .unwrap();
        adjust_article_stock(&conn, 1, 1, 50.0).unwrap();
        conn
    }

    fn stock(conn: &Connection) -> f64 {
        conn.query_row(
            "SELECT quantite FROM article_stocks WHERE article_id = 1 AND magasin_id = 1",
            [],
            |r| r.get(0),
        )
        .unwrap()
    }

    fn vendre(conn: &mut Connection, qte: f64) {
        create_vente_impl(
            conn,
            None,
            Some(1),
            vec![json!({ "article_id": 1, "quantite": qte })],
            None,
            "especes".into(),
            Some(vec![json!({ "mode": "especes", "montant": qte * 10.0 })]),
            Some("facture".into()),
            None,
            Some(1),
        )
        .unwrap();
    }

    fn ligne(conn: &Connection, inv: i64) -> i64 {
        conn.query_row(
            "SELECT id FROM inventaire_lignes WHERE inventaire_id = ?1 AND article_id = 1",
            params![inv],
            |r| r.get(0),
        )
        .unwrap()
    }

    #[test]
    fn test_ventes_pendant_l_inventaire_conservees() {
        let mut conn = setup();
        let inv = create_inventaire_impl(&mut conn, 1, 1).unwrap()["id"]
            .as_i64()
            .unwrap();
        vendre(&mut conn, 5.0);
        update_inventaire_ligne_impl(&conn, ligne(&conn, inv), 43.0).unwrap();
        vendre(&mut conn, 3.0);
        valider_inventaire_impl(&mut conn, inv, 1).unwrap();
        assert_eq!(stock(&conn), 40.0);
        let total: f64 = conn
            .query_row("SELECT stock FROM articles WHERE id = 1", [], |r| r.get(0))
            .unwrap();
        assert_eq!(total, 40.0);
        let ecart: f64 = conn
            .query_row(
                "SELECT quantite FROM mouvements_stock WHERE mtype = 'inventaire'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(ecart, -2.0);
    }

    #[test]
    fn test_inventaire_valide_verrouille() {
        let mut conn = setup();
        let inv = create_inventaire_impl(&mut conn, 1, 1).unwrap()["id"]
            .as_i64()
            .unwrap();
        let l = ligne(&conn, inv);
        update_inventaire_ligne_impl(&conn, l, 48.0).unwrap();
        valider_inventaire_impl(&mut conn, inv, 1).unwrap();
        assert!(update_inventaire_ligne_impl(&conn, l, 10.0).is_err());
        assert!(valider_inventaire_impl(&mut conn, inv, 1).is_err());
        assert_eq!(stock(&conn), 48.0);
    }

    #[test]
    fn test_controles() {
        let mut conn = setup();
        let inv = create_inventaire_impl(&mut conn, 1, 1).unwrap()["id"]
            .as_i64()
            .unwrap();
        assert!(create_inventaire_impl(&mut conn, 1, 1)
            .unwrap_err()
            .contains("déjà en cours"));
        assert!(update_inventaire_ligne_impl(&conn, ligne(&conn, inv), -1.0).is_err());
        assert!(update_inventaire_ligne_impl(&conn, ligne(&conn, inv), f64::NAN).is_err());
        assert!(update_inventaire_ligne_impl(&conn, 999, 1.0).is_err());
        valider_inventaire_impl(&mut conn, inv, 1).unwrap();
        assert_eq!(stock(&conn), 50.0);
        assert!(create_inventaire_impl(&mut conn, 1, 1).is_ok());
    }
}
