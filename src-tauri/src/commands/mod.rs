mod achats;
mod articles;
mod audit;
mod auth;
mod backup;
mod caisses;
mod categories;
mod cheques;
mod clients;
mod composants;
mod fournisseurs;
mod inventaire;
mod journal;
mod lots;
mod magasins;
mod paiements;
mod permissions;
mod print;
mod rapports;
mod sessions;
mod settings;
mod stats;
mod stock;
mod tables;
mod utilisateurs;
mod variantes;
mod ventes;

use rusqlite::{params, Connection};

pub(crate) fn default_magasin_id(conn: &Connection) -> Result<i64, String> {
    conn.query_row("SELECT id FROM magasins ORDER BY id LIMIT 1", [], |r| r.get(0))
        .map_err(|e| format!("Aucun magasin configuré: {}", e))
}

pub(crate) fn adjust_article_stock(conn: &Connection, article_id: i64, magasin_id: i64, delta: f64) -> Result<(), String> {
    conn.execute(
        "INSERT INTO article_stocks (article_id, magasin_id, quantite) VALUES (?1, ?2, ?3)
         ON CONFLICT(article_id, magasin_id) DO UPDATE SET quantite = quantite + ?3",
        params![article_id, magasin_id, delta],
    ).map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE articles SET stock = (SELECT COALESCE(SUM(quantite), 0) FROM article_stocks WHERE article_id = ?1) WHERE id = ?1",
        params![article_id],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

pub(crate) fn log_audit(conn: &Connection, utilisateur_id: Option<i64>, action: &str, detail: &str, reference_type: Option<&str>, reference_id: Option<i64>) {
    let _ = conn.execute(
        "INSERT INTO audit_log (utilisateur_id, action, detail, reference_type, reference_id) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![utilisateur_id, action, detail, reference_type, reference_id],
    );
}

pub use achats::*;
pub use articles::*;
pub use audit::*;
pub use auth::*;
pub use backup::*;
pub use caisses::*;
pub use categories::*;
pub use cheques::*;
pub use clients::*;
pub use composants::*;
pub use fournisseurs::*;
pub use inventaire::*;
pub use journal::*;
pub use lots::*;
pub use magasins::*;
pub use paiements::*;
pub use permissions::*;
pub use print::*;
pub use rapports::*;
pub use sessions::*;
pub use settings::*;
pub use stats::*;
pub use stock::*;
pub use tables::*;
pub use utilisateurs::*;
pub use variantes::*;
pub use ventes::*;
