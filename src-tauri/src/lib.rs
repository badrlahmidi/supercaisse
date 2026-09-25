#![allow(clippy::too_many_arguments)]

mod commands;
mod db;
mod paths;
mod session;

use db::{init_db, DbState};
use paths::{legacy_database_candidates, prepare_database, AppDirs};
use session::AuthState;
use std::sync::{Arc, Mutex};
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AuthState::default())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
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
            app.manage(DbState {
                conn: Arc::new(Mutex::new(conn)),
            });
            app.manage(dirs);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::login,
            commands::logout,
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
            commands::open_caisse,
            commands::close_caisse,
            commands::get_tresorerie,
            commands::compare_fournisseur_prices,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
