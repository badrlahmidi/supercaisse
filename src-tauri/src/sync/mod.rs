mod auth;
mod client;
mod outbox;
mod pull;

pub use auth::{connexion_par_mot_de_passe, lire_tenant_id, lister_magasins};
pub use client::{ReqwestSupabaseClient, SupabaseClient};
pub use outbox::{
    cle_suppression_cloud, ligne_pour_cloud, lire_outbox_en_attente, marquer_echec,
    marquer_synchronise,
};
pub use pull::executer_cycle_pull;

const SUPABASE_URL: &str = "https://codpcxvcrfgrcazwdtgw.supabase.co";
const SUPABASE_PUBLISHABLE_KEY: &str = "sb_publishable_F8jJLoN3EfFp_-i8YeRcdA_cuiCi8Ly";

use rusqlite::{params, Connection, OptionalExtension};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::Manager;

static SYNC_EN_COURS: AtomicBool = AtomicBool::new(false);

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
pub fn enregistrer_identifiants(
    conn: &Connection,
    creds: &SyncCredentials,
) -> rusqlite::Result<()> {
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

fn mettre_a_jour_jeton(
    conn: &Connection,
    access_token: &str,
    refresh_token: &str,
    expires_at: &str,
) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE sync_credentials SET access_token = ?1, refresh_token = ?2, expires_at = ?3 WHERE id = 1",
        params![access_token, refresh_token, expires_at],
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
            let filtres = {
                let verrou = conn.lock().map_err(|e| e.to_string())?;
                cle_suppression_cloud(&verrou, &entree)
            };
            match filtres {
                Ok(f) => {
                    client
                        .supprimer(&identifiants, &entree.table_name, &f)
                        .await
                }
                Err(e) => Err(e),
            }
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
    if SYNC_EN_COURS
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        log::info!("Boucle de synchronisation déjà en cours");
        return;
    }
    let conn = etat.conn.clone();
    tauri::async_runtime::spawn(async move {
        struct ResetOnDrop;
        impl Drop for ResetOnDrop {
            fn drop(&mut self) {
                SYNC_EN_COURS.store(false, Ordering::SeqCst);
            }
        }
        let _guard = ResetOnDrop;
        boucle_synchro(conn).await;
    });
}

async fn rafraichir_si_expire(conn: &Arc<Mutex<Connection>>) {
    let (refresh_token, expires_at) = {
        let Ok(verrou) = conn.lock() else { return };
        let Some(creds) = lire_identifiants(&verrou).ok().flatten() else {
            return;
        };
        match (creds.refresh_token, creds.expires_at) {
            (Some(rt), Some(ea)) => (rt, ea),
            _ => return,
        }
    };

    let expire = match chrono::DateTime::parse_from_rfc3339(&expires_at) {
        Ok(dt) => dt,
        Err(_) => return,
    };
    if expire > chrono::Utc::now() + chrono::Duration::minutes(5) {
        return;
    }

    match auth::rafraichir_jeton(&refresh_token).await {
        Ok(session) => {
            let nouveau_expires =
                (chrono::Utc::now() + chrono::Duration::seconds(session.expires_in)).to_rfc3339();
            if let Ok(verrou) = conn.lock() {
                let _ = mettre_a_jour_jeton(
                    &verrou,
                    &session.access_token,
                    &session.refresh_token,
                    &nouveau_expires,
                );
            }
        }
        Err(e) => log::warn!("Rafraîchissement du jeton échoué : {}", e),
    }
}

async fn boucle_synchro(conn: Arc<Mutex<Connection>>) {
    let client = ReqwestSupabaseClient::nouveau();
    let mut intervalle = tokio::time::interval(Duration::from_secs(15));
    loop {
        intervalle.tick().await;
        rafraichir_si_expire(&conn).await;
        if let Err(e) = executer_un_cycle(&conn, &client).await {
            log::warn!("Cycle de synchronisation (envoi) échoué : {}", e);
        }
        let identifiants = match conn.lock() {
            Ok(verrou) => lire_identifiants(&verrou).ok().flatten(),
            Err(_) => None,
        };
        if let Some(creds) = identifiants {
            if let Err(e) = executer_cycle_pull(&conn, &client, &creds).await {
                log::warn!("Cycle de synchronisation (réception) échoué : {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_db;
    use serde_json::Value;
    use std::sync::Mutex as StdMutex;

    type SuppressionEnregistree = (String, Vec<(String, String)>);

    #[derive(Default)]
    struct ClientFactice {
        upserts: StdMutex<Vec<(String, Value)>>,
        suppressions: StdMutex<Vec<SuppressionEnregistree>>,
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
            filtres: &[(String, String)],
        ) -> Result<(), String> {
            self.suppressions
                .lock()
                .unwrap()
                .push((table.to_string(), filtres.to_vec()));
            Ok(())
        }

        async fn recuperer(
            &self,
            _creds: &SyncCredentials,
            _table: &str,
            _depuis: Option<&str>,
            _limite: i64,
            _colonne_curseur: Option<&str>,
        ) -> Result<Vec<Value>, String> {
            Ok(Vec::new())
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
