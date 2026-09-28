use crate::db::*;
use crate::session::{autoriser, Acces, AuthState};
use rusqlite::params;
use std::sync::Mutex;
use tauri::{AppHandle, State};

use super::contrats::{
    AssociationMagasinCloud, ChoixAppairageCloud, EtatSynchroCloud, MagasinAppairageCloud,
    MagasinLocalNonAssocie,
};
use super::log_audit;

pub struct PairingPending {
    tenant_id: String,
    access_token: String,
    refresh_token: String,
    expires_in: i64,
}

#[derive(Default)]
pub struct PairingState(pub Mutex<Option<PairingPending>>);

#[tauri::command(async)]
pub async fn demarrer_appairage_cloud(
    db: State<'_, DbState>,
    auth: State<'_, AuthState>,
    pairing: State<'_, PairingState>,
    token: String,
    email: String,
    mot_de_passe: String,
) -> Result<ChoixAppairageCloud, String> {
    {
        let conn = db.lecture()?;
        autoriser(&auth, &conn, &token, Acces::Admin)?;
    }

    let session = crate::sync::connexion_par_mot_de_passe(&email, &mot_de_passe).await?;
    let tenant_id = crate::sync::lire_tenant_id(&session.access_token, &session.user.id).await?;
    let magasins_cloud = crate::sync::lister_magasins(&session.access_token, &tenant_id).await?;

    let magasins_locaux = {
        let conn = db.lecture()?;
        let mut stmt = conn
            .prepare("SELECT id, nom FROM magasins WHERE cloud_magasin_id IS NULL ORDER BY nom")
            .map_err(|e| e.to_string())?;
        let lignes = stmt
            .query_map([], |r| {
                Ok(MagasinLocalNonAssocie {
                    id: r.get(0)?,
                    nom: r.get(1)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| e.to_string())?;
        lignes
    };

    *pairing.0.lock().map_err(|e| e.to_string())? = Some(PairingPending {
        tenant_id,
        access_token: session.access_token,
        refresh_token: session.refresh_token,
        expires_in: session.expires_in,
    });

    Ok(ChoixAppairageCloud {
        magasins_cloud: magasins_cloud
            .into_iter()
            .map(|m| MagasinAppairageCloud {
                cloud_magasin_id: m.id,
                nom: m.nom,
            })
            .collect(),
        magasins_locaux,
    })
}

#[tauri::command(async)]
pub fn annuler_appairage_cloud(
    db: State<DbState>,
    auth: State<AuthState>,
    pairing: State<PairingState>,
    token: String,
) -> Result<(), String> {
    let conn = db.lecture()?;
    autoriser(&auth, &conn, &token, Acces::Admin)?;
    *pairing.0.lock().map_err(|e| e.to_string())? = None;
    Ok(())
}

#[tauri::command(async)]
pub fn finaliser_appairage_cloud(
    app: AppHandle,
    db: State<DbState>,
    auth: State<AuthState>,
    pairing: State<PairingState>,
    token: String,
    associations: Vec<AssociationMagasinCloud>,
) -> Result<(), String> {
    let mut conn = db.conn.lock().map_err(|e| e.to_string())?;
    let me = autoriser(&auth, &conn, &token, Acces::Admin)?;
    if associations.is_empty() {
        return Err("Sélectionnez au moins une boutique à associer".to_string());
    }
    let pending = pairing
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .take()
        .ok_or_else(|| "Aucun appairage en cours : reconnectez-vous".to_string())?;
    let expires_at =
        (chrono::Utc::now() + chrono::Duration::seconds(pending.expires_in)).to_rfc3339();

    let tx = conn.transaction().map_err(|e| e.to_string())?;
    for association in &associations {
        tx.execute(
            "UPDATE magasins SET cloud_magasin_id = ?1 WHERE id = ?2",
            params![association.cloud_magasin_id, association.magasin_local_id],
        )
        .map_err(|e| e.to_string())?;
    }
    crate::sync::enregistrer_identifiants(
        &tx,
        &crate::sync::SyncCredentials {
            tenant_id: pending.tenant_id,
            cloud_magasin_id: associations[0].cloud_magasin_id.clone(),
            access_token: pending.access_token,
            refresh_token: Some(pending.refresh_token),
            expires_at: Some(expires_at),
        },
    )
    .map_err(|e| e.to_string())?;
    log_audit(
        &tx,
        Some(me.user_id),
        "appairage_cloud",
        "Synchronisation cloud activée",
        None,
        None,
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    drop(conn);

    crate::sync::demarrer_si_configure(&app);
    Ok(())
}

#[tauri::command(async)]
pub fn obtenir_etat_synchro(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
) -> Result<EtatSynchroCloud, String> {
    let conn = db.lecture()?;
    autoriser(&auth, &conn, &token, Acces::Admin)?;
    let identifiants = crate::sync::lire_identifiants(&conn).map_err(|e| e.to_string())?;
    let en_attente: i64 = conn
        .query_row("SELECT COUNT(*) FROM sync_outbox", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    Ok(match identifiants {
        Some(creds) => EtatSynchroCloud {
            connecte: true,
            tenant_id: Some(creds.tenant_id),
            cloud_magasin_id: Some(creds.cloud_magasin_id),
            en_attente,
        },
        None => EtatSynchroCloud {
            connecte: false,
            tenant_id: None,
            cloud_magasin_id: None,
            en_attente,
        },
    })
}

#[tauri::command(async)]
pub fn desappairer_cloud(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let me = autoriser(&auth, &conn, &token, Acces::Admin)?;
    conn.execute("DELETE FROM sync_credentials", [])
        .map_err(|e| e.to_string())?;
    log_audit(
        &conn,
        Some(me.user_id),
        "desappairage_cloud",
        "Synchronisation cloud désactivée",
        None,
        None,
    )?;
    Ok(())
}
