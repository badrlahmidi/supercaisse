use super::outbox::{cles_etrangeres, colonnes_table};
use super::{SupabaseClient, SyncCredentials};
use rusqlite::types::ToSql;
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;
use std::sync::{Arc, Mutex};

pub(super) const PULL_ORDER: &[&str] = &[
    "categories",
    "fournisseurs",
    "clients",
    "tables_resto",
    "caisses",
    "sessions_caisse",
    "articles",
    "cheques",
    "paiements",
    "achats",
    "article_variantes",
    "article_composants",
    "article_stocks",
    "article_lots",
    "achat_articles",
    "journal_caisse",
    "ventes",
    "article_variante_stocks",
    "vente_articles",
    "vente_paiements",
    "vente_lots",
    "mouvements_fidelite",
    "mouvements_stock",
    "inventaires",
    "transferts_stock",
    "inventaire_lignes",
    "transfert_lignes",
];

const TABLES_CLE_COMPOSITE_CLOUD: &[(&str, &str, &str)] = &[
    ("article_stocks", "article_id", "magasin_id"),
    ("article_variante_stocks", "variante_id", "magasin_id"),
    ("transfert_lignes", "transfert_id", "article_id"),
];

const TAILLE_PAGE: i64 = 200;

fn lire_curseur(conn: &Connection, table: &str) -> rusqlite::Result<Option<String>> {
    conn.query_row(
        "SELECT last_pulled_at FROM sync_pull_state WHERE table_name = ?1",
        params![table],
        |r| r.get(0),
    )
    .optional()
    .map(Option::flatten)
}

fn ecrire_curseur(conn: &Connection, table: &str, valeur: &str) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO sync_pull_state (table_name, last_pulled_at) VALUES (?1, ?2)
         ON CONFLICT(table_name) DO UPDATE SET last_pulled_at = excluded.last_pulled_at",
        params![table, valeur],
    )?;
    Ok(())
}

fn colonnes_non_nulles(conn: &Connection, table: &str) -> rusqlite::Result<Vec<(String, bool)>> {
    conn.prepare(&format!("PRAGMA table_info({})", table))?
        .query_map([], |r| Ok((r.get::<_, String>(1)?, r.get::<_, bool>(3)?)))?
        .collect()
}

fn traduire_reference_inverse(
    conn: &Connection,
    table_cible: &str,
    valeur_cloud: &str,
) -> rusqlite::Result<Option<i64>> {
    let requete = match table_cible {
        "utilisateurs" => "SELECT id FROM utilisateurs WHERE cloud_profile_id = ?1".to_string(),
        "magasins" => "SELECT id FROM magasins WHERE cloud_magasin_id = ?1".to_string(),
        _ => format!("SELECT id FROM {} WHERE uuid = ?1", table_cible),
    };
    conn.query_row(&requete, params![valeur_cloud], |r| r.get(0))
        .optional()
}

fn valeur_sql(v: &Value) -> Box<dyn ToSql> {
    match v {
        Value::Bool(b) => Box::new(*b),
        Value::Number(n) => match n.as_i64() {
            Some(i) => Box::new(i),
            None => Box::new(n.as_f64().unwrap_or(0.0)),
        },
        Value::String(s) => Box::new(s.clone()),
        _ => Box::new(Option::<i64>::None),
    }
}

fn executer_upsert(
    conn: &Connection,
    table: &str,
    colonnes: &[String],
    valeurs: &[Box<dyn ToSql>],
) -> Result<(), String> {
    let placeholders = (1..=colonnes.len())
        .map(|i| format!("?{}", i))
        .collect::<Vec<_>>()
        .join(", ");
    let maj = colonnes
        .iter()
        .enumerate()
        .filter(|(_, nom)| nom.as_str() != "uuid")
        .map(|(i, nom)| format!("{} = ?{}", nom, i + 1))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "INSERT INTO {t} ({cols}) VALUES ({ph})
         ON CONFLICT(uuid) WHERE uuid IS NOT NULL DO UPDATE SET {maj}",
        t = table,
        cols = colonnes.join(", "),
        ph = placeholders,
        maj = maj
    );
    let refs: Vec<&dyn ToSql> = valeurs.iter().map(|v| v.as_ref()).collect();
    conn.execute(&sql, refs.as_slice())
        .map(|_| ())
        .map_err(|e| e.to_string())
}

fn appliquer_ligne_uuid(conn: &Connection, table: &str, ligne: &Value) -> Result<bool, String> {
    let objet = ligne
        .as_object()
        .ok_or_else(|| "ligne cloud invalide".to_string())?;
    let id_cloud = objet
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| "id manquant".to_string())?;

    let colonnes_locales = colonnes_table(conn, table).map_err(|e| e.to_string())?;
    let cles = cles_etrangeres(conn, table).map_err(|e| e.to_string())?;
    let obligatoires = colonnes_non_nulles(conn, table).map_err(|e| e.to_string())?;

    let est_obligatoire = |nom: &str| {
        obligatoires
            .iter()
            .any(|(n, notnull)| n == nom && *notnull)
    };

    let mut colonnes = Vec::new();
    let mut valeurs: Vec<Box<dyn ToSql>> = Vec::new();
    for colonne in &colonnes_locales {
        if colonne == "uuid" {
            continue;
        }
        let Some(valeur_cloud) = objet.get(colonne) else {
            continue;
        };
        if let Some((_, table_cible)) = cles.iter().find(|(c, _)| c == colonne) {
            let id_local = match valeur_cloud.as_str() {
                None => None,
                Some(s) => traduire_reference_inverse(conn, table_cible, s)
                    .map_err(|e| e.to_string())?,
            };
            match id_local {
                Some(id) => {
                    colonnes.push(colonne.clone());
                    valeurs.push(Box::new(id));
                }
                None if est_obligatoire(colonne) => return Ok(false),
                None => {}
            }
        } else {
            colonnes.push(colonne.clone());
            valeurs.push(valeur_sql(valeur_cloud));
        }
    }
    colonnes.push("uuid".to_string());
    valeurs.push(Box::new(id_cloud.to_string()));

    executer_upsert(conn, table, &colonnes, &valeurs)?;
    Ok(true)
}

fn appliquer_ligne_composite(conn: &Connection, table: &str, ligne: &Value) -> Result<bool, String> {
    let objet = ligne
        .as_object()
        .ok_or_else(|| "ligne cloud invalide".to_string())?;
    let (colonne1, table1, colonne2, table2) = TABLES_CLE_COMPOSITE_CLOUD
        .iter()
        .find(|(t, _, _)| *t == table)
        .map(|(_, c1, c2)| {
            let cible = |c: &str| match table {
                "article_stocks" if c == "article_id" => "articles",
                "article_stocks" => "magasins",
                "article_variante_stocks" if c == "variante_id" => "article_variantes",
                "article_variante_stocks" => "magasins",
                "transfert_lignes" if c == "transfert_id" => "transferts_stock",
                _ => "articles",
            };
            (*c1, cible(c1), *c2, cible(c2))
        })
        .ok_or_else(|| format!("{} n'est pas une table à clé composite", table))?;

    let uuid1 = objet
        .get(colonne1)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{} manquant", colonne1))?;
    let uuid2 = objet
        .get(colonne2)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{} manquant", colonne2))?;
    let id1 = traduire_reference_inverse(conn, table1, uuid1).map_err(|e| e.to_string())?;
    let id2 = traduire_reference_inverse(conn, table2, uuid2).map_err(|e| e.to_string())?;
    let (Some(id1), Some(id2)) = (id1, id2) else {
        return Ok(false);
    };

    let colonnes_locales = colonnes_table(conn, table).map_err(|e| e.to_string())?;
    let mut colonnes = vec![colonne1.to_string(), colonne2.to_string()];
    let mut valeurs: Vec<Box<dyn ToSql>> = vec![Box::new(id1), Box::new(id2)];
    for colonne in &colonnes_locales {
        if colonne == colonne1 || colonne == colonne2 || colonne == "uuid" {
            continue;
        }
        if let Some(valeur) = objet.get(colonne) {
            colonnes.push(colonne.clone());
            valeurs.push(valeur_sql(valeur));
        }
    }

    let placeholders = (1..=colonnes.len())
        .map(|i| format!("?{}", i))
        .collect::<Vec<_>>()
        .join(", ");
    let maj = colonnes
        .iter()
        .enumerate()
        .filter(|(_, nom)| nom.as_str() != colonne1 && nom.as_str() != colonne2)
        .map(|(i, nom)| format!("{} = ?{}", nom, i + 1))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = if maj.is_empty() {
        format!(
            "INSERT OR IGNORE INTO {t} ({cols}) VALUES ({ph})",
            t = table,
            cols = colonnes.join(", "),
            ph = placeholders
        )
    } else {
        format!(
            "INSERT INTO {t} ({cols}) VALUES ({ph}) ON CONFLICT({c1}, {c2}) DO UPDATE SET {maj}",
            t = table,
            cols = colonnes.join(", "),
            ph = placeholders,
            c1 = colonne1,
            c2 = colonne2,
            maj = maj
        )
    };
    let refs: Vec<&dyn ToSql> = valeurs.iter().map(|v| v.as_ref()).collect();
    conn.execute(&sql, refs.as_slice())
        .map_err(|e| e.to_string())?;
    Ok(true)
}

pub async fn executer_pull_table<C: SupabaseClient>(
    conn: &Arc<Mutex<Connection>>,
    client: &C,
    creds: &SyncCredentials,
    table: &str,
) -> Result<(), String> {
    let curseur = {
        let verrou = conn.lock().map_err(|e| e.to_string())?;
        lire_curseur(&verrou, table).map_err(|e| e.to_string())?
    };
    let lignes = client
        .recuperer(creds, table, curseur.as_deref(), TAILLE_PAGE)
        .await?;

    let verrou = conn.lock().map_err(|e| e.to_string())?;
    let composite = TABLES_CLE_COMPOSITE_CLOUD.iter().any(|(t, _, _)| *t == table);
    let mut dernier_horodatage: Option<String> = None;
    for ligne in &lignes {
        let horodatage = ligne
            .get("updated_at")
            .and_then(Value::as_str)
            .map(|s| s.to_string());
        let applique = if composite {
            appliquer_ligne_composite(&verrou, table, ligne)
        } else {
            appliquer_ligne_uuid(&verrou, table, ligne)
        };
        match applique {
            Ok(true) => dernier_horodatage = horodatage.or(dernier_horodatage),
            Ok(false) => {
                log::warn!(
                    "Pull {} : ligne ignorée (dépendance non résolue), nouvelle tentative au prochain cycle",
                    table
                );
                break;
            }
            Err(e) => {
                log::warn!("Pull {} : ligne ignorée ({})", table, e);
                break;
            }
        }
    }
    if let Some(horodatage) = dernier_horodatage {
        ecrire_curseur(&verrou, table, &horodatage).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub async fn executer_cycle_pull<C: SupabaseClient>(
    conn: &Arc<Mutex<Connection>>,
    client: &C,
    creds: &SyncCredentials,
) -> Result<(), String> {
    for table in PULL_ORDER {
        executer_pull_table(conn, client, creds, table).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_db;
    use serde_json::json;
    use std::collections::HashMap;
    use std::sync::Mutex as StdMutex;

    #[derive(Default)]
    struct ClientFactice {
        pages: StdMutex<HashMap<String, Vec<Value>>>,
    }

    impl ClientFactice {
        fn avec(table: &str, lignes: Vec<Value>) -> Self {
            let mut pages = HashMap::new();
            pages.insert(table.to_string(), lignes);
            Self {
                pages: StdMutex::new(pages),
            }
        }
    }

    impl SupabaseClient for ClientFactice {
        async fn upsert(&self, _: &SyncCredentials, _: &str, _: Value) -> Result<(), String> {
            Ok(())
        }
        async fn supprimer(
            &self,
            _: &SyncCredentials,
            _: &str,
            _: &[(String, String)],
        ) -> Result<(), String> {
            Ok(())
        }
        async fn recuperer(
            &self,
            _: &SyncCredentials,
            table: &str,
            _: Option<&str>,
            _: i64,
        ) -> Result<Vec<Value>, String> {
            Ok(self.pages.lock().unwrap().get(table).cloned().unwrap_or_default())
        }
    }

    fn identifiants_de_test() -> SyncCredentials {
        SyncCredentials {
            tenant_id: "tenant-1".to_string(),
            cloud_magasin_id: "magasin-1".to_string(),
            access_token: "jeton".to_string(),
            refresh_token: None,
            expires_at: None,
        }
    }

    #[test]
    fn test_appliquer_ligne_uuid_cree_puis_met_a_jour() {
        let conn = init_db(":memory:").unwrap();
        let ligne = json!({
            "id": "cloud-categorie-1",
            "tenant_id": "tenant-1",
            "nom": "Boissons",
            "description": null,
            "updated_at": "2026-01-01T00:00:00Z"
        });
        assert!(appliquer_ligne_uuid(&conn, "categories", &ligne).unwrap());
        let nom: String = conn
            .query_row(
                "SELECT nom FROM categories WHERE uuid = 'cloud-categorie-1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(nom, "Boissons");

        let maj = json!({
            "id": "cloud-categorie-1",
            "tenant_id": "tenant-1",
            "nom": "Boissons fraîches",
            "description": null,
            "updated_at": "2026-01-02T00:00:00Z"
        });
        assert!(appliquer_ligne_uuid(&conn, "categories", &maj).unwrap());
        let compte: i64 = conn
            .query_row("SELECT COUNT(*) FROM categories", [], |r| r.get(0))
            .unwrap();
        assert_eq!(compte, 1, "une même ligne cloud ne doit pas se dupliquer localement");
        let nom: String = conn
            .query_row(
                "SELECT nom FROM categories WHERE uuid = 'cloud-categorie-1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(nom, "Boissons fraîches");
    }

    #[test]
    fn test_appliquer_ligne_uuid_traduit_les_cles_etrangeres() {
        let conn = init_db(":memory:").unwrap();
        conn.execute("INSERT INTO categories (nom) VALUES ('Boissons')", [])
            .unwrap();
        let uuid_categorie: String = conn
            .query_row("SELECT uuid FROM categories WHERE nom = 'Boissons'", [], |r| {
                r.get(0)
            })
            .unwrap();

        let ligne = json!({
            "id": "cloud-article-1",
            "tenant_id": "tenant-1",
            "code_barre": "",
            "designation": "Coca",
            "description": "",
            "image_url": "",
            "prix_achat": 5.0,
            "prix_vente": 8.0,
            "prix_grossiste": null,
            "tva": 20.0,
            "stock": 0.0,
            "stock_alerte": 0.0,
            "categorie_id": uuid_categorie,
            "fournisseur_id": null,
            "actif": true,
            "divers_taux": 0.0,
            "suivi_lot": false,
            "est_kit": false
        });
        assert!(appliquer_ligne_uuid(&conn, "articles", &ligne).unwrap());
        let (designation, categorie_id, actif): (String, i64, i64) = conn
            .query_row(
                "SELECT designation, categorie_id, actif FROM articles WHERE uuid = 'cloud-article-1'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(designation, "Coca");
        assert_eq!(actif, 1);
        let id_categorie_locale: i64 = conn
            .query_row("SELECT id FROM categories WHERE nom = 'Boissons'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(categorie_id, id_categorie_locale);
    }

    #[test]
    fn test_appliquer_ligne_uuid_ignore_dependance_obligatoire_non_resolue() {
        let conn = init_db(":memory:").unwrap();
        let ligne = json!({
            "id": "cloud-vente-article-1",
            "tenant_id": "tenant-1",
            "vente_id": "cloud-vente-inconnue",
            "article_id": null,
            "variante_id": null,
            "quantite": 1.0,
            "prix_unitaire": 10.0,
            "prix_type": "public",
            "tva": 0.0,
            "remise_ligne": 0.0,
            "total_ligne": 10.0,
            "montant_ht": null,
            "montant_tva": null,
            "note": null
        });
        assert!(!appliquer_ligne_uuid(&conn, "vente_articles", &ligne).unwrap());
        let compte: i64 = conn
            .query_row("SELECT COUNT(*) FROM vente_articles", [], |r| r.get(0))
            .unwrap();
        assert_eq!(compte, 0, "la ligne ne doit pas être créée sans sa vente");
    }

    #[test]
    fn test_appliquer_ligne_composite() {
        let conn = init_db(":memory:").unwrap();
        conn.execute("INSERT INTO articles (designation) VALUES ('Coca')", [])
            .unwrap();
        let uuid_article: String = conn
            .query_row("SELECT uuid FROM articles WHERE id = 1", [], |r| r.get(0))
            .unwrap();
        let uuid_magasin: String = conn
            .query_row("SELECT uuid FROM magasins WHERE id = 1", [], |r| r.get(0))
            .unwrap();

        let ligne = json!({
            "tenant_id": "tenant-1",
            "article_id": uuid_article,
            "magasin_id": uuid_magasin,
            "quantite": 42.0
        });
        assert!(appliquer_ligne_composite(&conn, "article_stocks", &ligne).unwrap());
        let quantite: f64 = conn
            .query_row(
                "SELECT quantite FROM article_stocks WHERE article_id = 1 AND magasin_id = 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(quantite, 42.0);

        let maj = json!({
            "tenant_id": "tenant-1",
            "article_id": uuid_article,
            "magasin_id": uuid_magasin,
            "quantite": 7.0
        });
        assert!(appliquer_ligne_composite(&conn, "article_stocks", &maj).unwrap());
        let quantite: f64 = conn
            .query_row(
                "SELECT quantite FROM article_stocks WHERE article_id = 1 AND magasin_id = 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(quantite, 7.0);
    }

    #[tokio::test]
    async fn test_executer_pull_table_avance_le_curseur() {
        let conn = Arc::new(StdMutex::new(init_db(":memory:").unwrap()));
        let client = ClientFactice::avec(
            "categories",
            vec![json!({
                "id": "cloud-categorie-1",
                "tenant_id": "tenant-1",
                "nom": "Boissons",
                "description": null,
                "updated_at": "2026-01-01T00:00:00Z"
            })],
        );
        let creds = identifiants_de_test();
        executer_pull_table(&conn, &client, &creds, "categories")
            .await
            .unwrap();
        let verrou = conn.lock().unwrap();
        let curseur = lire_curseur(&verrou, "categories").unwrap();
        assert_eq!(curseur, Some("2026-01-01T00:00:00Z".to_string()));
        let compte: i64 = verrou
            .query_row("SELECT COUNT(*) FROM categories", [], |r| r.get(0))
            .unwrap();
        assert_eq!(compte, 1);
    }
}
