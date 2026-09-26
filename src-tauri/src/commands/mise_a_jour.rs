use crate::db::*;
use crate::paths::AppDirs;
use crate::session::{autoriser, Acces, AuthState};
use serde::Serialize;
use tauri::{AppHandle, State};
use tauri_plugin_updater::UpdaterExt;

use super::backup::sauvegarder;
use super::log_audit;

#[derive(Debug, Serialize, PartialEq)]
pub struct MiseAJourDisponible {
    pub version: String,
    pub date: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct EtatMiseAJour {
    pub version_actuelle: String,
    pub configuree: bool,
    pub disponible: Option<MiseAJourDisponible>,
}

pub(crate) fn cle_publique_configuree(config: &tauri::Config) -> bool {
    config
        .plugins
        .0
        .get("updater")
        .and_then(|u| u.get("pubkey"))
        .and_then(|k| k.as_str())
        .is_some_and(|k| !k.trim().is_empty())
}

async fn rechercher(app: &AppHandle) -> Result<Option<tauri_plugin_updater::Update>, String> {
    if !cle_publique_configuree(app.config()) {
        return Err(
            "Mises à jour automatiques non configurées : clé publique de signature absente"
                .to_string(),
        );
    }
    app.updater()
        .map_err(|e| e.to_string())?
        .check()
        .await
        .map_err(|e| format!("Recherche de mise à jour impossible : {}", e))
}

#[tauri::command(async)]
pub async fn verifier_mise_a_jour(
    app: AppHandle,
    db: State<'_, DbState>,
    auth: State<'_, AuthState>,
    token: String,
) -> Result<EtatMiseAJour, String> {
    let _me = {
        let conn = db.lecture()?;
        autoriser(&auth, &conn, &token, Acces::Admin)?.user_id
    };
    let version_actuelle = app.package_info().version.to_string();
    if !cle_publique_configuree(app.config()) {
        return Ok(EtatMiseAJour {
            version_actuelle,
            configuree: false,
            disponible: None,
        });
    }
    let disponible = rechercher(&app).await?.map(|u| MiseAJourDisponible {
        version: u.version.clone(),
        date: u.date.map(|d| d.to_string()),
        notes: u.body.clone(),
    });
    log::info!(
        "Recherche de mise à jour : {}",
        disponible
            .as_ref()
            .map(|d| format!("version {} disponible", d.version))
            .unwrap_or_else(|| "à jour".to_string())
    );
    Ok(EtatMiseAJour {
        version_actuelle,
        configuree: true,
        disponible,
    })
}

#[tauri::command(async)]
pub async fn installer_mise_a_jour(
    app: AppHandle,
    db: State<'_, DbState>,
    dirs: State<'_, AppDirs>,
    auth: State<'_, AuthState>,
    token: String,
) -> Result<(), String> {
    let me = {
        let conn = db.lecture()?;
        autoriser(&auth, &conn, &token, Acces::Admin)?.user_id
    };
    let mise_a_jour = rechercher(&app)
        .await?
        .ok_or("Aucune mise à jour disponible")?;
    let sauvegarde = {
        let conn = db.lecture()?;
        sauvegarder(&conn, &dirs, "avant_mise_a_jour_")?
    };
    {
        let conn = db.conn.lock().map_err(|e| e.to_string())?;
        log_audit(
            &conn,
            Some(me),
            "mise_a_jour",
            &format!(
                "Lancement de l'installation de la version {} (depuis {}), sauvegarde préalable : {}",
                mise_a_jour.version,
                mise_a_jour.current_version,
                sauvegarde.display()
            ),
            None,
            None,
        )?;
    }
    log::info!(
        "Installation de la mise à jour {} (sauvegarde {})",
        mise_a_jour.version,
        sauvegarde.display()
    );
    mise_a_jour
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(|e| {
            log::error!("Installation de la mise à jour impossible : {}", e);
            format!("Installation de la mise à jour impossible : {}", e)
        })?;
    app.restart()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cle_publique_requise() {
        let mut config: tauri::Config =
            serde_json::from_str(include_str!("../../tauri.conf.json")).unwrap();
        let pubkey = config.plugins.0["updater"]["pubkey"].clone();
        assert_eq!(
            cle_publique_configuree(&config),
            !pubkey.as_str().unwrap().is_empty()
        );
        config.plugins.0.get_mut("updater").unwrap()["pubkey"] = serde_json::json!("  ");
        assert!(!cle_publique_configuree(&config));
        config.plugins.0.get_mut("updater").unwrap()["pubkey"] = serde_json::json!("dW50cnVzdGVk");
        assert!(cle_publique_configuree(&config));
        assert_eq!(config.plugins.0["updater"]["requireSignedVersion"], true);
    }

    #[test]
    fn test_versions_alignees() {
        let package: serde_json::Value =
            serde_json::from_str(include_str!("../../../package.json")).unwrap();
        assert_eq!(package["version"], env!("CARGO_PKG_VERSION"));
        let config: serde_json::Value =
            serde_json::from_str(include_str!("../../tauri.conf.json")).unwrap();
        assert_eq!(config["version"], "../package.json");
    }
}
