#![allow(clippy::too_many_arguments)]

mod commands;
mod db;
mod paths;
mod session;
mod sync;

use db::{init_db, DbState};
use paths::{legacy_database_candidates, prepare_database, AppDirs};
use session::AuthState;
use tauri::Manager;
use tauri_plugin_log::{RotationStrategy, Target, TargetKind, TimezoneStrategy};

fn journal() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    let mut cibles = vec![Target::new(TargetKind::LogDir {
        file_name: Some("supercaisse".into()),
    })];
    if cfg!(debug_assertions) {
        cibles.push(Target::new(TargetKind::Stdout));
    }
    tauri_plugin_log::Builder::new()
        .targets(cibles)
        .level(log::LevelFilter::Info)
        .level_for("tao", log::LevelFilter::Warn)
        .level_for("wry", log::LevelFilter::Warn)
        .timezone_strategy(TimezoneStrategy::UseLocal)
        .rotation_strategy(RotationStrategy::KeepSome(10))
        .max_file_size(5_000_000)
        .build()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    commands::installer_hook_panique();
    let resultat = tauri::Builder::default()
        .plugin(journal())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(AuthState::default())
        .setup(|app| {
            log::info!("Démarrage de SuperCaisse {}", app.package_info().version);
            let data = app.path().app_data_dir()?;
            let documents = app
                .path()
                .document_dir()
                .map(|d| d.join("SuperCaisse"))
                .unwrap_or_else(|_| data.clone());
            let dirs = AppDirs { data, documents };
            let db_path = prepare_database(&dirs, &legacy_database_candidates())?;
            let conn = init_db(&db_path.to_string_lossy()).map_err(|e| {
                format!(
                    "Initialisation de la base {} impossible : {}",
                    db_path.display(),
                    e
                )
            })?;
            log::info!("Base de données : {}", db_path.display());
            match commands::sauvegarde_quotidienne(&conn, &dirs) {
                Ok(Some(chemin)) => log::info!("Sauvegarde automatique : {}", chemin.display()),
                Ok(None) => {}
                Err(e) => log::error!("Sauvegarde automatique impossible : {}", e),
            }
            app.manage(DbState::avec_lecteurs(conn, &db_path, db::LECTEURS)?);
            app.manage(dirs);
            app.manage(commands::PairingState::default());
            sync::demarrer_si_configure(app.handle());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::login,
            commands::logout,
            commands::journaliser_frontend,
            commands::verifier_mise_a_jour,
            commands::installer_mise_a_jour,
            commands::get_categories,
            commands::add_category,
            commands::update_category,
            commands::delete_category,
            commands::get_fournisseurs,
            commands::add_fournisseur,
            commands::update_fournisseur,
            commands::delete_fournisseur,
            commands::get_clients,
            commands::add_client,
            commands::update_client,
            commands::delete_client,
            commands::get_articles,
            commands::add_article,
            commands::update_article,
            commands::delete_article,
            commands::update_article_stock,
            commands::add_article_lot,
            commands::get_article_lots,
            commands::get_lots_peremption_proche,
            commands::discard_article_lot,
            commands::add_article_variante,
            commands::get_article_variantes,
            commands::update_article_variante,
            commands::adjust_article_variante_stock,
            commands::delete_article_variante,
            commands::find_variante_by_barcode,
            commands::add_article_composant,
            commands::get_article_composants,
            commands::update_article_composant_quantite,
            commands::delete_article_composant,
            commands::create_vente,
            commands::annuler_vente,
            commands::convert_document,
            commands::get_ventes,
            commands::get_vente_details,
            commands::create_achat,
            commands::get_mouvements_stock,
            commands::get_achats,
            commands::update_achat_status,
            commands::get_cheques,
            commands::add_cheque,
            commands::update_cheque_status,
            commands::get_paiements,
            commands::add_paiement,
            commands::get_stats,
            commands::print_ticket,
            commands::print_escpos,
            commands::open_cash_drawer,
            commands::print_receipt,
            commands::save_document_pdf,
            commands::get_articles_stock_alerte,
            commands::get_journal_caisse,
            commands::add_journal_caisse,
            commands::get_utilisateurs,
            commands::add_utilisateur,
            commands::update_utilisateur,
            commands::delete_utilisateur,
            commands::import_articles_csv,
            commands::get_settings,
            commands::update_settings,
            commands::backup_database,
            commands::export_database,
            commands::import_database,
            commands::list_backups,
            commands::get_current_session,
            commands::open_session,
            commands::close_session,
            commands::get_magasins,
            commands::get_stats_magasins,
            commands::add_magasin,
            commands::update_magasin,
            commands::delete_magasin,
            commands::get_transferts,
            commands::get_stock_par_magasin,
            commands::create_transfert,
            commands::validate_transfert,
            commands::get_tables,
            commands::update_table_status,
            commands::get_mouvements_fidelite,
            commands::get_rapport_x,
            commands::get_releve_client,
            commands::get_audit_log,
            commands::login_pin,
            commands::get_comptes_pin,
            commands::change_password,
            commands::set_user_pin,
            commands::get_rapport_detaille,
            commands::create_inventaire,
            commands::get_inventaire,
            commands::get_inventaires,
            commands::update_inventaire_ligne,
            commands::valider_inventaire,
            commands::get_permissions,
            commands::update_permission,
            commands::get_caisses,
            commands::get_tresorerie,
            commands::compare_fournisseur_prices,
            commands::demarrer_appairage_cloud,
            commands::annuler_appairage_cloud,
            commands::finaliser_appairage_cloud,
            commands::obtenir_etat_synchro,
            commands::desappairer_cloud,
        ])
        .run(tauri::generate_context!());
    if let Err(e) = resultat {
        log::error!("Arrêt de l'application sur erreur : {}", e);
        log::logger().flush();
        eprintln!("SuperCaisse n'a pas pu démarrer : {}", e);
        std::process::exit(1);
    }
}
