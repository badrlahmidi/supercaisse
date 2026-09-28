use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{Map, Value};

pub(super) const TABLES_CLE_COMPOSITE: &[&str] =
    &["article_stocks", "article_variante_stocks", "transfert_lignes"];

const COLONNES_BOOLEENNES: &[&str] = &["actif", "suivi_lot", "est_kit"];
const COLONNES_TRACABILITE_EXCLUES: &[&str] =
    &["created_at", "created_by", "updated_at", "updated_by"];

pub(super) fn cle_conflit_cloud(table: &str) -> &'static str {
    match table {
        "article_stocks" => "article_id,magasin_id",
        "article_variante_stocks" => "variante_id,magasin_id",
        "transfert_lignes" => "transfert_id,article_id",
        _ => "id",
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OutboxEntry {
    pub id: i64,
    pub table_name: String,
    pub row_uuid: String,
    pub operation: String,
    pub cle1_locale: Option<i64>,
    pub cle2_locale: Option<i64>,
}

pub fn lire_outbox_en_attente(
    conn: &Connection,
    limite: i64,
) -> rusqlite::Result<Vec<OutboxEntry>> {
    conn.prepare(
        "SELECT id, table_name, row_uuid, operation, cle1_locale, cle2_locale
         FROM sync_outbox ORDER BY queued_at LIMIT ?1",
    )?
    .query_map(params![limite], |r| {
        Ok(OutboxEntry {
            id: r.get(0)?,
            table_name: r.get(1)?,
            row_uuid: r.get(2)?,
            operation: r.get(3)?,
            cle1_locale: r.get(4)?,
            cle2_locale: r.get(5)?,
        })
    })?
    .collect()
}

pub fn cle_suppression_cloud(
    conn: &Connection,
    entree: &OutboxEntry,
) -> Result<Vec<(String, String)>, String> {
    let (colonne1, table1, colonne2, table2) = match entree.table_name.as_str() {
        "article_stocks" => ("article_id", "articles", "magasin_id", "magasins"),
        "article_variante_stocks" => {
            ("variante_id", "article_variantes", "magasin_id", "magasins")
        }
        "transfert_lignes" => ("transfert_id", "transferts_stock", "article_id", "articles"),
        _ => return Ok(vec![("id".to_string(), entree.row_uuid.clone())]),
    };
    let id1 = entree
        .cle1_locale
        .ok_or_else(|| format!("{} : première clé locale manquante", entree.table_name))?;
    let id2 = entree
        .cle2_locale
        .ok_or_else(|| format!("{} : seconde clé locale manquante", entree.table_name))?;
    let uuid1 = traduire_reference(conn, table1, id1)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("{} #{} introuvable au cloud", table1, id1))?;
    let uuid2 = traduire_reference(conn, table2, id2)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("{} #{} introuvable au cloud", table2, id2))?;
    Ok(vec![(colonne1.to_string(), uuid1), (colonne2.to_string(), uuid2)])
}

pub fn marquer_synchronise(conn: &Connection, id: i64) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM sync_outbox WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn marquer_echec(conn: &Connection, id: i64, erreur: &str) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE sync_outbox SET attempts = attempts + 1, last_error = ?2 WHERE id = ?1",
        params![id, erreur],
    )?;
    Ok(())
}

pub(super) fn colonnes_table(conn: &Connection, table: &str) -> rusqlite::Result<Vec<String>> {
    conn.prepare(&format!("PRAGMA table_info({})", table))?
        .query_map([], |r| r.get::<_, String>(1))?
        .collect()
}

pub(super) fn cles_etrangeres(
    conn: &Connection,
    table: &str,
) -> rusqlite::Result<Vec<(String, String)>> {
    conn.prepare(&format!("PRAGMA foreign_key_list({})", table))?
        .query_map([], |r| Ok((r.get::<_, String>(3)?, r.get::<_, String>(2)?)))?
        .collect()
}

fn traduire_reference(
    conn: &Connection,
    table_cible: &str,
    id_local: i64,
) -> rusqlite::Result<Option<String>> {
    let requete = match table_cible {
        "utilisateurs" => "SELECT cloud_profile_id FROM utilisateurs WHERE id = ?1".to_string(),
        "magasins" => "SELECT cloud_magasin_id FROM magasins WHERE id = ?1".to_string(),
        _ => format!("SELECT uuid FROM {} WHERE id = ?1", table_cible),
    };
    conn.query_row(&requete, params![id_local], |r| r.get(0))
        .optional()
}

pub fn ligne_pour_cloud(conn: &Connection, table: &str, row_uuid: &str) -> Result<Value, String> {
    let colonnes = colonnes_table(conn, table).map_err(|e| e.to_string())?;
    let cles = cles_etrangeres(conn, table).map_err(|e| e.to_string())?;
    let requete = format!(
        "SELECT {} FROM {} WHERE uuid = ?1",
        colonnes.join(", "),
        table
    );

    let mut objet = Map::new();
    let trouve = conn
        .query_row(&requete, params![row_uuid], |ligne| {
            for (index, nom) in colonnes.iter().enumerate() {
                let valeur = match ligne.get_ref(index)? {
                    rusqlite::types::ValueRef::Null => Value::Null,
                    rusqlite::types::ValueRef::Integer(i) if COLONNES_BOOLEENNES.contains(&nom.as_str()) => {
                        Value::Bool(i != 0)
                    }
                    rusqlite::types::ValueRef::Integer(i) => Value::from(i),
                    rusqlite::types::ValueRef::Real(f) => serde_json::Number::from_f64(f)
                        .map(Value::Number)
                        .unwrap_or(Value::Null),
                    rusqlite::types::ValueRef::Text(t) => {
                        Value::String(String::from_utf8_lossy(t).into_owned())
                    }
                    rusqlite::types::ValueRef::Blob(_) => Value::Null,
                };
                objet.insert(nom.clone(), valeur);
            }
            Ok(())
        })
        .optional()
        .map_err(|e| e.to_string())?;
    if trouve.is_none() {
        return Err(format!("ligne introuvable : {}.{}", table, row_uuid));
    }

    objet.remove("uuid");
    for colonne in COLONNES_TRACABILITE_EXCLUES {
        objet.remove(*colonne);
    }
    if !TABLES_CLE_COMPOSITE.contains(&table) {
        objet.insert("id".into(), Value::String(row_uuid.to_string()));
    }

    for (colonne_locale, table_cible) in cles {
        let id_local = match objet.get(&colonne_locale) {
            Some(Value::Number(n)) => n.as_i64(),
            _ => None,
        };
        let uuid_cloud = match id_local {
            None => None,
            Some(id_local) => {
                traduire_reference(conn, &table_cible, id_local).map_err(|e| e.to_string())?
            }
        };
        objet.insert(
            colonne_locale,
            uuid_cloud.map(Value::String).unwrap_or(Value::Null),
        );
    }

    Ok(Value::Object(objet))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_db;

    #[test]
    fn test_lire_et_marquer_outbox() {
        let conn = init_db(":memory:").unwrap();
        conn.execute("INSERT INTO categories (nom) VALUES ('Boissons')", [])
            .unwrap();
        let en_attente = lire_outbox_en_attente(&conn, 10).unwrap();
        assert_eq!(en_attente.len(), 1);
        assert_eq!(en_attente[0].table_name, "categories");
        assert_eq!(en_attente[0].operation, "upsert");

        marquer_echec(&conn, en_attente[0].id, "réseau indisponible").unwrap();
        let (tentatives, erreur): (i64, String) = conn
            .query_row(
                "SELECT attempts, last_error FROM sync_outbox WHERE id = ?1",
                params![en_attente[0].id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(tentatives, 1);
        assert_eq!(erreur, "réseau indisponible");

        marquer_synchronise(&conn, en_attente[0].id).unwrap();
        assert_eq!(lire_outbox_en_attente(&conn, 10).unwrap().len(), 0);
    }

    #[test]
    fn test_ligne_pour_cloud_traduit_les_cles_etrangeres() {
        let conn = init_db(":memory:").unwrap();
        conn.execute("INSERT INTO categories (nom) VALUES ('Boissons')", [])
            .unwrap();
        conn.execute(
            "INSERT INTO articles (designation, categorie_id) VALUES ('Coca', 1)",
            [],
        )
        .unwrap();
        let uuid_categorie: String = conn
            .query_row("SELECT uuid FROM categories WHERE id = 1", [], |r| r.get(0))
            .unwrap();
        let uuid_article: String = conn
            .query_row("SELECT uuid FROM articles WHERE id = 1", [], |r| r.get(0))
            .unwrap();

        let ligne = ligne_pour_cloud(&conn, "articles", &uuid_article).unwrap();
        let objet = ligne.as_object().unwrap();
        assert_eq!(objet["id"], Value::String(uuid_article));
        assert_eq!(objet["categorie_id"], Value::String(uuid_categorie));
        assert_eq!(objet["fournisseur_id"], Value::Null);
        assert!(!objet.contains_key("uuid"));
    }

    #[test]
    fn test_ligne_pour_cloud_traduit_utilisateur_et_magasin() {
        let conn = init_db(":memory:").unwrap();
        conn.execute(
            "UPDATE utilisateurs SET cloud_profile_id = 'profil-cloud-1' WHERE id = 1",
            [],
        )
        .unwrap();
        conn.execute(
            "UPDATE magasins SET cloud_magasin_id = 'magasin-cloud-1' WHERE id = 1",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO ventes (caissier_id, magasin_id, montant_total) VALUES (1, 1, 10)",
            [],
        )
        .unwrap();
        let uuid_vente: String = conn
            .query_row("SELECT uuid FROM ventes WHERE id = 1", [], |r| r.get(0))
            .unwrap();

        let ligne = ligne_pour_cloud(&conn, "ventes", &uuid_vente).unwrap();
        let objet = ligne.as_object().unwrap();
        assert_eq!(
            objet["caissier_id"],
            Value::String("profil-cloud-1".to_string())
        );
        assert_eq!(
            objet["magasin_id"],
            Value::String("magasin-cloud-1".to_string())
        );
    }

    #[test]
    fn test_ligne_pour_cloud_ligne_absente() {
        let conn = init_db(":memory:").unwrap();
        assert!(ligne_pour_cloud(&conn, "categories", "inexistant").is_err());
    }
}
