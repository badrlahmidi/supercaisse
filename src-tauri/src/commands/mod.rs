macro_rules! filtre_ca {
    () => {
        "(v.statut != 'annulee' AND (COALESCE(v.dtype, 'facture') IN ('facture', 'avoir') OR (v.dtype = 'bl' AND NOT EXISTS (SELECT 1 FROM ventes vf WHERE vf.source_vente_id = v.id AND vf.dtype = 'facture'))))"
    };
}

macro_rules! quantite_signee {
    ($alias:literal) => {
        concat!(
            "(CASE WHEN v.dtype = 'avoir' THEN -",
            $alias,
            ".quantite ELSE ",
            $alias,
            ".quantite END)"
        )
    };
}

mod achats;
mod articles;
mod audit;
mod auth;
mod backup;
mod caisses;
mod calcul;
mod categories;
mod cheques;
mod clients;
mod composants;
pub mod contrats;
mod diagnostic;
mod fiscal;
mod fournisseurs;
mod inventaire;
mod journal;
mod lots;
mod magasins;
mod mise_a_jour;
mod mouvements;
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
    conn.query_row("SELECT id FROM magasins ORDER BY id LIMIT 1", [], |r| {
        r.get(0)
    })
    .map_err(|e| format!("Aucun magasin configuré: {}", e))
}

pub(crate) fn adjust_article_stock(
    conn: &Connection,
    article_id: i64,
    magasin_id: i64,
    delta: f64,
) -> Result<(), String> {
    conn.execute(
        "INSERT INTO article_stocks (article_id, magasin_id, quantite) VALUES (?1, ?2, ?3)
         ON CONFLICT(article_id, magasin_id) DO UPDATE SET quantite = quantite + ?3",
        params![article_id, magasin_id, delta],
    )
    .map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE articles SET stock = (SELECT COALESCE(SUM(quantite), 0) FROM article_stocks WHERE article_id = ?1) WHERE id = ?1",
        params![article_id],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

pub(crate) fn document_prefixe(dtype: &str) -> Result<&'static str, String> {
    match dtype {
        "facture" => Ok("FA"),
        "bl" => Ok("BL"),
        "devis" => Ok("DE"),
        "commande" => Ok("CO"),
        "avoir" => Ok("AV"),
        _ => Err(format!("Type de document inconnu : {}", dtype)),
    }
}

pub(crate) fn next_numero_document(
    conn: &Connection,
    dtype: &str,
    annee: i32,
) -> Result<String, String> {
    let prefixe = document_prefixe(dtype)?;
    let incrementer = || -> Result<String, String> {
        let numero: i64 = conn.query_row(
            "INSERT INTO numerotation_v2 (ntype, annee, prefixe, dernier_numero) VALUES (?1, ?2, ?3, 1)
             ON CONFLICT(ntype, annee) DO UPDATE SET dernier_numero = dernier_numero + 1
             RETURNING dernier_numero",
            params![format!("{}_client", dtype), annee, prefixe],
            |r| r.get(0),
        ).map_err(|e| format!("Numérotation {} {} impossible: {}", dtype, annee, e))?;
        Ok(format!("{}-{}-{:05}", prefixe, annee, numero))
    };
    let deja_attribue = |numero: &str| -> Result<bool, String> {
        conn.query_row(
            "SELECT COUNT(*) > 0 FROM ventes WHERE numero_facture = ?1",
            params![numero],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())
    };
    let numero = incrementer()?;
    if !deja_attribue(&numero)? {
        return Ok(numero);
    }
    log::warn!(
        "Compteur de numérotation en retard ({} déjà attribué) : resynchronisation",
        numero
    );
    resynchroniser_numerotation(conn).map_err(|e| e.to_string())?;
    let numero = incrementer()?;
    if deja_attribue(&numero)? {
        return Err(format!(
            "Numérotation incohérente : {} est déjà attribué",
            numero
        ));
    }
    Ok(numero)
}

pub(crate) fn resynchroniser_numerotation(conn: &Connection) -> rusqlite::Result<()> {
    for dtype in crate::db::TYPES_DOCUMENT {
        let Ok(prefixe) = document_prefixe(dtype) else {
            continue;
        };
        let mut stmt = conn.prepare(
            "SELECT CAST(substr(numero_facture, 4, 4) AS INTEGER), MAX(CAST(substr(numero_facture, 9) AS INTEGER))
             FROM ventes
             WHERE numero_facture GLOB ?1 || '-[0-9][0-9][0-9][0-9]-[0-9]*'
             GROUP BY 1",
        )?;
        let maxima = stmt
            .query_map(params![prefixe], |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        for (annee, dernier) in maxima {
            conn.execute(
                "INSERT INTO numerotation_v2 (ntype, annee, prefixe, dernier_numero) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(ntype, annee) DO UPDATE SET dernier_numero = MAX(dernier_numero, excluded.dernier_numero)",
                params![format!("{}_client", dtype), annee, prefixe, dernier],
            )?;
        }
    }
    Ok(())
}

pub(crate) fn annee_courante() -> i32 {
    use chrono::Datelike;
    chrono::Local::now().year()
}

pub(crate) fn log_audit(
    conn: &Connection,
    utilisateur_id: Option<i64>,
    action: &str,
    detail: &str,
    reference_type: Option<&str>,
    reference_id: Option<i64>,
) -> Result<(), String> {
    conn.execute(
        "INSERT INTO audit_log (utilisateur_id, action, detail, reference_type, reference_id) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![utilisateur_id, action, detail, reference_type, reference_id],
    )
    .map(|_| ())
    .map_err(|e| format!("Journal d'audit impossible à écrire ({}) : opération annulée", e))
}

pub(crate) fn tracer_creation(
    conn: &Connection,
    table: &str,
    id: i64,
    auteur: Option<i64>,
) -> Result<(), String> {
    debug_assert!(crate::db::TABLES_TRACEES.contains(&table));
    conn.execute(
        &format!("UPDATE {} SET created_by = ?1 WHERE id = ?2", table),
        params![auteur, id],
    )
    .map(|_| ())
    .map_err(|e| e.to_string())
}

pub(crate) fn tracer_modification(
    conn: &Connection,
    table: &str,
    id: i64,
    auteur: Option<i64>,
) -> Result<(), String> {
    debug_assert!(crate::db::TABLES_TRACEES.contains(&table));
    conn.execute(
        &format!(
            "UPDATE {} SET updated_at = datetime('now', 'localtime'), updated_by = ?1 WHERE id = ?2",
            table
        ),
        params![auteur, id],
    )
    .map(|_| ())
    .map_err(|e| e.to_string())
}

pub(crate) fn valeur_autorisee(
    libelle: &str,
    valeur: &str,
    autorisees: &[&str],
) -> Result<(), String> {
    if autorisees.contains(&valeur) {
        Ok(())
    } else {
        Err(format!(
            "{} invalide : « {} » (valeurs possibles : {})",
            libelle,
            valeur,
            autorisees.join(", ")
        ))
    }
}

pub(crate) fn erreur_suppression(erreur: rusqlite::Error, element: &str) -> String {
    match &erreur {
        rusqlite::Error::SqliteFailure(e, _)
            if e.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_FOREIGNKEY =>
        {
            format!(
                "Suppression impossible : {} est utilisé par d'autres enregistrements (ventes, stock, achats…)",
                element
            )
        }
        _ => erreur.to_string(),
    }
}

pub(crate) fn fin_de_journee(fin: &str) -> String {
    if fin.len() == 10 {
        format!("{} 23:59:59", fin)
    } else {
        fin.to_string()
    }
}

pub(crate) fn tracer<T>(operation: &str, resultat: Result<T, String>) -> Result<T, String> {
    match &resultat {
        Ok(_) => log::info!("{} : réussi", operation),
        Err(e) => log::warn!("{} : échec ({})", operation, e),
    }
    resultat
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
pub use diagnostic::*;
pub use fournisseurs::*;
pub use inventaire::*;
pub use journal::*;
pub use lots::*;
pub use magasins::*;
pub use mise_a_jour::*;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_suppression_d_un_element_utilise_explicite() {
        let conn = crate::db::init_db(":memory:").unwrap();
        conn.execute(
            "INSERT INTO ventes (montant_total, magasin_id) VALUES (10, 1)",
            [],
        )
        .unwrap();
        let err = conn
            .execute("DELETE FROM magasins WHERE id = 1", [])
            .map_err(|e| erreur_suppression(e, "ce magasin"))
            .unwrap_err();
        assert!(
            err.starts_with("Suppression impossible : ce magasin"),
            "{err}"
        );
        assert!(valeur_autorisee("Rôle", "patron", crate::db::ROLES).is_err());
        assert!(valeur_autorisee("Rôle", "manager", crate::db::ROLES).is_ok());
    }

    #[test]
    fn test_fin_de_journee_inclut_toute_la_journee() {
        assert_eq!(fin_de_journee("2026-09-25"), "2026-09-25 23:59:59");
        assert_eq!(fin_de_journee("2026-09-25 12:00:00"), "2026-09-25 12:00:00");
        assert!("2026-09-25 11:03:53" <= fin_de_journee("2026-09-25").as_str());
    }
    use rusqlite::Connection;

    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "
            CREATE TABLE magasins (id INTEGER PRIMARY KEY, nom TEXT);
            CREATE TABLE articles (id INTEGER PRIMARY KEY, stock REAL DEFAULT 0);
            CREATE TABLE article_stocks (
                id INTEGER PRIMARY KEY,
                article_id INTEGER, magasin_id INTEGER, quantite REAL DEFAULT 0,
                UNIQUE(article_id, magasin_id)
            );
            INSERT INTO magasins (id, nom) VALUES (1, 'Principal');
            INSERT INTO articles (id, stock) VALUES (1, 0);
        ",
        )
        .unwrap();
        conn
    }

    #[test]
    fn test_default_magasin_id() {
        let conn = setup_test_db();
        assert_eq!(default_magasin_id(&conn).unwrap(), 1);
    }

    fn init_full_db() -> Connection {
        crate::db::init_db(":memory:").unwrap()
    }

    #[test]
    fn test_numero_document_increments() {
        let conn = init_full_db();
        assert_eq!(
            next_numero_document(&conn, "facture", 2026).unwrap(),
            "FA-2026-00001"
        );
        assert_eq!(
            next_numero_document(&conn, "facture", 2026).unwrap(),
            "FA-2026-00002"
        );
    }

    #[test]
    fn test_numero_document_changement_annee() {
        let conn = init_full_db();
        assert_eq!(
            next_numero_document(&conn, "facture", 2026).unwrap(),
            "FA-2026-00001"
        );
        assert_eq!(
            next_numero_document(&conn, "facture", 2026).unwrap(),
            "FA-2026-00002"
        );
        assert_eq!(
            next_numero_document(&conn, "facture", 2027).unwrap(),
            "FA-2027-00001"
        );
        assert_eq!(
            next_numero_document(&conn, "facture", 2027).unwrap(),
            "FA-2027-00002"
        );
        assert_eq!(
            next_numero_document(&conn, "facture", 2028).unwrap(),
            "FA-2028-00001"
        );
    }

    #[test]
    fn test_numero_document_sequences_par_type() {
        let conn = init_full_db();
        assert_eq!(
            next_numero_document(&conn, "facture", 2026).unwrap(),
            "FA-2026-00001"
        );
        assert_eq!(
            next_numero_document(&conn, "avoir", 2026).unwrap(),
            "AV-2026-00001"
        );
        assert_eq!(
            next_numero_document(&conn, "bl", 2026).unwrap(),
            "BL-2026-00001"
        );
        assert_eq!(
            next_numero_document(&conn, "devis", 2026).unwrap(),
            "DE-2026-00001"
        );
        assert_eq!(
            next_numero_document(&conn, "commande", 2026).unwrap(),
            "CO-2026-00001"
        );
        assert_eq!(
            next_numero_document(&conn, "facture", 2026).unwrap(),
            "FA-2026-00002"
        );
    }

    #[test]
    fn test_numero_document_type_inconnu() {
        let conn = init_full_db();
        assert!(next_numero_document(&conn, "ticket", 2026).is_err());
    }

    #[test]
    fn test_numero_document_rollback_transaction() {
        let mut conn = init_full_db();
        {
            let tx = conn.transaction().unwrap();
            assert_eq!(
                next_numero_document(&tx, "facture", 2026).unwrap(),
                "FA-2026-00001"
            );
        }
        assert_eq!(
            next_numero_document(&conn, "facture", 2026).unwrap(),
            "FA-2026-00001"
        );
    }

    #[test]
    fn test_migration_numerotation_existante() {
        let path = std::env::temp_dir().join(format!(
            "supercaisse_test_numerotation_{}_{}.db",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path_str = path.to_string_lossy().to_string();
        {
            let legacy = Connection::open(&path).unwrap();
            legacy.execute_batch("
                CREATE TABLE numerotation (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    ntype TEXT NOT NULL UNIQUE,
                    annee INTEGER NOT NULL,
                    prefixe TEXT NOT NULL,
                    dernier_numero INTEGER NOT NULL DEFAULT 0
                );
                INSERT INTO numerotation (ntype, annee, prefixe, dernier_numero) VALUES ('facture_client', 2026, 'FA', 42);
                INSERT INTO numerotation (ntype, annee, prefixe, dernier_numero) VALUES ('avoir_client', 2026, 'AV', 3);
            ").unwrap();
        }
        let conn = crate::db::init_db(&path_str).unwrap();
        assert_eq!(
            next_numero_document(&conn, "facture", 2026).unwrap(),
            "FA-2026-00043"
        );
        assert_eq!(
            next_numero_document(&conn, "avoir", 2026).unwrap(),
            "AV-2026-00004"
        );
        assert_eq!(
            next_numero_document(&conn, "facture", 2027).unwrap(),
            "FA-2027-00001"
        );
        drop(conn);
        let reopened = crate::db::init_db(&path_str).unwrap();
        assert_eq!(
            next_numero_document(&reopened, "facture", 2026).unwrap(),
            "FA-2026-00044"
        );
        drop(reopened);
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{}", path_str, suffix));
        }
    }

    #[test]
    fn test_numero_facture_unique() {
        let conn = init_full_db();
        conn.execute(
            "INSERT INTO ventes (numero_facture) VALUES ('FA-2026-00001')",
            [],
        )
        .unwrap();
        assert!(conn
            .execute(
                "INSERT INTO ventes (numero_facture) VALUES ('FA-2026-00001')",
                []
            )
            .is_err());
        conn.execute("INSERT INTO ventes (numero_facture) VALUES (NULL)", [])
            .unwrap();
        conn.execute("INSERT INTO ventes (numero_facture) VALUES (NULL)", [])
            .unwrap();
    }

    #[test]
    fn test_adjust_article_stock() {
        let conn = setup_test_db();
        adjust_article_stock(&conn, 1, 1, 10.0).unwrap();
        let stock: f64 = conn
            .query_row("SELECT stock FROM articles WHERE id = 1", [], |r| r.get(0))
            .unwrap();
        assert_eq!(stock, 10.0);

        adjust_article_stock(&conn, 1, 1, -3.0).unwrap();
        let stock: f64 = conn
            .query_row("SELECT stock FROM articles WHERE id = 1", [], |r| r.get(0))
            .unwrap();
        assert_eq!(stock, 7.0);
    }
}
