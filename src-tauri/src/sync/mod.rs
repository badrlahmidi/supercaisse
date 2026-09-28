mod client;
mod outbox;

pub use client::{ReqwestSupabaseClient, SupabaseClient};
pub use outbox::{ligne_pour_cloud, lire_outbox_en_attente, marquer_echec, marquer_synchronise};

use rusqlite::{params, Connection, OptionalExtension};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::Manager;

#[derive(Debug, Clone, PartialEq)]
pub struct SyncCredentials {
    pub tenant_id: String,
    pub cloud_magasin_id: String,
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<String>,
}

pub fn lire_identifiants(conn: &Connection) -> rusqlite::Result<Option<SyncCredentials>> {
    conn.query_row(
        "SELECT tenant_id, cloud_magasin_id, access_token, refresh_token, expires_at
         FROM sync_credentials WHERE id = 1",
        [],
        |r| {
            Ok(SyncCredentials {
                tenant_id: r.get(0)?,
                cloud_magasin_id: r.get(1)?,
                access_token: r.get(2)?,
                refresh_token: r.get(3)?,
                expires_at: r.get(4)?,
            })
        },
    )
    .optional()
}

#[allow(dead_code)]
pub fn enregistrer_identifiants(conn: &Connection, creds: &SyncCredentials) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO sync_credentials (id, tenant_id, cloud_magasin_id, access_token, refresh_token, expires_at)
         VALUES (1, ?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(id) DO UPDATE SET
            tenant_id = excluded.tenant_id,
            cloud_magasin_id = excluded.cloud_magasin_id,
            access_token = excluded.access_token,
            refresh_token = excluded.refresh_token,
            expires_at = excluded.expires_at",
        params![
            creds.tenant_id,
            creds.cloud_magasin_id,
            creds.access_token,
            creds.refresh_token,
            creds.expires_at
        ],
    )?;
    Ok(())
}

pub async fn executer_un_cycle<C: SupabaseClient>(
    conn: &Arc<Mutex<Connection>>,
    client: &C,
) -> Result<(), String> {
    let (identifiants, lignes) = {
        let verrou = conn.lock().map_err(|e| e.to_string())?;
        let identifiants = match lire_identifiants(&verrou).map_err(|e| e.to_string())? {
            Some(i) => i,
            None => return Ok(()),
        };
        let lignes = lire_outbox_en_attente(&verrou, 50).map_err(|e| e.to_string())?;
        (identifiants, lignes)
    };

    for entree in lignes {
        let resultat = if entree.operation == "delete" {
            client
                .supprimer(&identifiants, &entree.table_name, &entree.row_uuid)
                .await
        } else {
            let payload = {
                let verrou = conn.lock().map_err(|e| e.to_string())?;
                ligne_pour_cloud(&verrou, &entree.table_name, &entree.row_uuid)
            };
            match payload {
                Ok(p) => client.upsert(&identifiants, &entree.table_name, p).await,
                Err(e) => Err(e),
            }
        };

        let verrou = conn.lock().map_err(|e| e.to_string())?;
        match resultat {
            Ok(()) => marquer_synchronise(&verrou, entree.id).map_err(|e| e.to_string())?,
            Err(e) => marquer_echec(&verrou, entree.id, &e).map_err(|e| e.to_string())?,
        }
    }
    Ok(())
}

pub fn demarrer_si_configure(app: &tauri::AppHandle) {
    let etat = app.state::<crate::db::DbState>();
    let configure = match etat.conn.lock() {
        Ok(conn) => lire_identifiants(&conn).ok().flatten().is_some(),
        Err(_) => false,
    };
    if !configure {
        log::info!("Synchronisation cloud non configurée : boucle non démarrée");
        return;
    }
    let conn = etat.conn.clone();
    tauri::async_runtime::spawn(async move {
        boucle_synchro(conn).await;
    });
}

async fn boucle_synchro(conn: Arc<Mutex<Connection>>) {
    let client = ReqwestSupabaseClient::nouveau();
    let mut intervalle = tokio::time::interval(Duration::from_secs(15));
    loop {
        intervalle.tick().await;
        if let Err(e) = executer_un_cycle(&conn, &client).await {
            log::warn!("Cycle de synchronisation échoué : {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_db;
    use serde_json::Value;
    use std::sync::Mutex as StdMutex;

    #[derive(Default)]
    struct ClientFactice {
        upserts: StdMutex<Vec<(String, Value)>>,
        suppressions: StdMutex<Vec<(String, String)>>,
        echoue_sur: Option<String>,
    }

    impl SupabaseClient for ClientFactice {
        async fn upsert(
            &self,
            _creds: &SyncCredentials,
            table: &str,
            ligne: Value,
        ) -> Result<(), String> {
            if self.echoue_sur.as_deref() == Some(table) {
                return Err("échec simulé".to_string());
            }
            self.upserts
                .lock()
                .unwrap()
                .push((table.to_string(), ligne));
            Ok(())
        }

        async fn supprimer(
            &self,
            _creds: &SyncCredentials,
            table: &str,
            row_uuid: &str,
        ) -> Result<(), String> {
            self.suppressions
                .lock()
                .unwrap()
                .push((table.to_string(), row_uuid.to_string()));
            Ok(())
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

    #[tokio::test]
    async fn test_identifiants_absents_ne_fait_rien() {
        let conn = Arc::new(StdMutex::new(init_db(":memory:").unwrap()));
        let client = ClientFactice::default();
        let resultat = executer_un_cycle(&conn, &client).await;
        assert!(resultat.is_ok());
        assert!(client.upserts.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_cycle_upsert_puis_delete() {
        let conn = Arc::new(StdMutex::new(init_db(":memory:").unwrap()));
        {
            let verrou = conn.lock().unwrap();
            enregistrer_identifiants(&verrou, &identifiants_de_test()).unwrap();
            verrou
                .execute("INSERT INTO categories (nom) VALUES ('Boissons')", [])
                .unwrap();
        }
        let client = ClientFactice::default();
        executer_un_cycle(&conn, &client).await.unwrap();
        assert_eq!(client.upserts.lock().unwrap().len(), 1);
        assert_eq!(client.upserts.lock().unwrap()[0].0, "categories");
        {
            let verrou = conn.lock().unwrap();
            assert_eq!(lire_outbox_en_attente(&verrou, 10).unwrap().len(), 0);
        }

        {
            let verrou = conn.lock().unwrap();
            verrou
                .execute("DELETE FROM categories WHERE nom = 'Boissons'", [])
                .unwrap();
        }
        executer_un_cycle(&conn, &client).await.unwrap();
        assert_eq!(client.suppressions.lock().unwrap().len(), 1);
        assert_eq!(client.suppressions.lock().unwrap()[0].0, "categories");
    }

    #[tokio::test]
    async fn test_cycle_echec_conserve_dans_l_outbox_avec_erreur() {
        let conn = Arc::new(StdMutex::new(init_db(":memory:").unwrap()));
        {
            let verrou = conn.lock().unwrap();
            enregistrer_identifiants(&verrou, &identifiants_de_test()).unwrap();
            verrou
                .execute("INSERT INTO categories (nom) VALUES ('Boissons')", [])
                .unwrap();
        }
        let client = ClientFactice {
            echoue_sur: Some("categories".to_string()),
            ..Default::default()
        };
        executer_un_cycle(&conn, &client).await.unwrap();
        let verrou = conn.lock().unwrap();
        let en_attente = lire_outbox_en_attente(&verrou, 10).unwrap();
        assert_eq!(en_attente.len(), 1, "l'entrée en échec doit rester en file");
        let (tentatives, erreur): (i64, String) = verrou
            .query_row(
                "SELECT attempts, last_error FROM sync_outbox WHERE id = ?1",
                params![en_attente[0].id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(tentatives, 1);
        assert_eq!(erreur, "échec simulé");
    }
}
