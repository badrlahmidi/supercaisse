use crate::db::*;
use crate::paths::{ensure_dir, AppDirs};
use crate::session::{autoriser, Acces, AuthState};
use rusqlite::{backup::Backup, params, Connection, OpenFlags};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tauri::State;

use super::log_audit;

pub(crate) const SAUVEGARDES_AUTO_CONSERVEES: usize = 30;
const PREFIXE_AUTO: &str = "auto_";
const TABLES_ATTENDUES: [&str; 4] = ["articles", "ventes", "utilisateurs", "settings"];

fn horodatage() -> String {
    chrono::Local::now().format("%Y%m%d_%H%M%S_%3f").to_string()
}

pub(crate) fn copier_base(conn: &Connection, destination: &Path) -> Result<(), String> {
    if let Some(dossier) = destination.parent() {
        ensure_dir(dossier)?;
    }
    let mut dst = Connection::open(destination).map_err(|e| e.to_string())?;
    {
        let copie = Backup::new(conn, &mut dst).map_err(|e| e.to_string())?;
        copie
            .run_to_completion(64, Duration::from_millis(50), None)
            .map_err(|e| format!("Copie de la base impossible : {}", e))?;
    }
    dst.query_row("PRAGMA journal_mode = DELETE", [], |_| Ok(()))
        .map_err(|e| e.to_string())
}

pub(crate) fn sauvegarder(
    conn: &Connection,
    dirs: &AppDirs,
    prefixe: &str,
) -> Result<PathBuf, String> {
    let chemin = dirs
        .backups()
        .join(format!("{}supercaisse_{}.db", prefixe, horodatage()));
    copier_base(conn, &chemin)?;
    Ok(chemin)
}

pub(crate) fn sauvegarde_quotidienne(
    conn: &Connection,
    dirs: &AppDirs,
) -> Result<Option<PathBuf>, String> {
    let dossier = dirs.backups();
    ensure_dir(&dossier)?;
    let jour = chrono::Local::now().format("%Y%m%d").to_string();
    let deja_faite = std::fs::read_dir(&dossier)
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .any(|f| {
            f.file_name()
                .to_string_lossy()
                .starts_with(&format!("{}supercaisse_{}", PREFIXE_AUTO, jour))
        });
    if deja_faite {
        return Ok(None);
    }
    let chemin = sauvegarder(conn, dirs, PREFIXE_AUTO)?;
    purger_sauvegardes_auto(&dossier, SAUVEGARDES_AUTO_CONSERVEES)?;
    Ok(Some(chemin))
}

pub(crate) fn purger_sauvegardes_auto(dossier: &Path, a_conserver: usize) -> Result<(), String> {
    let mut fichiers: Vec<PathBuf> = std::fs::read_dir(dossier)
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .map(|f| f.path())
        .filter(|p| {
            p.file_name()
                .map(|n| n.to_string_lossy().starts_with(PREFIXE_AUTO))
                .unwrap_or(false)
                && p.extension().map(|e| e == "db").unwrap_or(false)
        })
        .collect();
    fichiers.sort();
    let surplus = fichiers.len().saturating_sub(a_conserver);
    for fichier in fichiers.into_iter().take(surplus) {
        std::fs::remove_file(&fichier)
            .map_err(|e| format!("Suppression de {} impossible : {}", fichier.display(), e))?;
    }
    Ok(())
}

pub(crate) fn verifier_sauvegarde(chemin: &Path) -> Result<i64, String> {
    if !chemin.is_file() {
        return Err(format!("Fichier introuvable : {}", chemin.display()));
    }
    let src = Connection::open_with_flags(chemin, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| format!("Fichier illisible : {}", e))?;
    let integrite: String = src
        .query_row("PRAGMA integrity_check", [], |r| r.get(0))
        .map_err(|_| "Le fichier n'est pas une base SQLite valide".to_string())?;
    if integrite != "ok" {
        return Err(format!("Fichier corrompu : {}", integrite));
    }
    for table in TABLES_ATTENDUES {
        let existe: bool = src
            .query_row(
                "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type = 'table' AND name = ?1",
                params![table],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        if !existe {
            return Err(format!(
                "Ce fichier n'est pas une sauvegarde SuperCaisse (table « {} » absente)",
                table
            ));
        }
    }
    let version = version_schema(&src).map_err(|e| e.to_string())?;
    if version > SCHEMA_VERSION {
        return Err(format!(
            "Sauvegarde au schéma v{} issue d'une version plus récente de SuperCaisse (cette version gère jusqu'à v{})",
            version, SCHEMA_VERSION
        ));
    }
    let admins: i64 = src
        .query_row(
            "SELECT COUNT(*) FROM utilisateurs WHERE role = 'admin'",
            [],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if admins == 0 {
        return Err(
            "La sauvegarde ne contient aucun administrateur : restauration refusée".to_string(),
        );
    }
    Ok(version)
}

fn remplacer_base(conn: &mut Connection, source: &Path) -> Result<(), String> {
    let src = Connection::open_with_flags(source, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| e.to_string())?;
    {
        let copie = Backup::new(&src, conn).map_err(|e| e.to_string())?;
        copie
            .run_to_completion(64, Duration::from_millis(50), None)
            .map_err(|e| format!("Restauration impossible : {}", e))?;
    }
    conn.query_row("PRAGMA journal_mode = WAL", [], |_| Ok(()))
        .map_err(|e| e.to_string())
}

pub(crate) fn restaurer(
    conn: &mut Connection,
    dirs: &AppDirs,
    source: &Path,
    auteur: Option<i64>,
) -> Result<PathBuf, String> {
    let version_source = verifier_sauvegarde(source)?;
    let securite = sauvegarder(conn, dirs, "avant_restauration_")?;
    let resultat = remplacer_base(conn, source)
        .and_then(|_| migrer(conn))
        .and_then(|_| {
            log_audit(
                conn,
                auteur,
                "importer_base",
                &format!(
                    "Restauration depuis {} (schéma v{} → v{}), sauvegarde préalable : {}",
                    source.display(),
                    version_source,
                    SCHEMA_VERSION,
                    securite.display()
                ),
                None,
                None,
            )
        });
    if let Err(e) = resultat {
        remplacer_base(conn, &securite).map_err(|e2| {
            format!(
                "Restauration échouée ({}) et retour arrière impossible ({}) : restaurez manuellement {}",
                e,
                e2,
                securite.display()
            )
        })?;
        return Err(format!(
            "Restauration annulée, base actuelle conservée : {}",
            e
        ));
    }
    Ok(securite)
}

#[tauri::command(async)]
pub fn backup_database(
    db: State<DbState>,
    dirs: State<AppDirs>,
    auth: State<AuthState>,
    token: String,
) -> Result<String, String> {
    let conn = db.lecture()?;
    let _me = autoriser(&auth, &conn, &token, Acces::Module("settings", "exporter"))?;
    let chemin = super::tracer("Sauvegarde manuelle", sauvegarder(&conn, &dirs, ""))?;
    Ok(chemin.to_string_lossy().to_string())
}

#[tauri::command(async)]
pub fn export_database(
    db: State<DbState>,
    dirs: State<AppDirs>,
    auth: State<AuthState>,
    token: String,
) -> Result<String, String> {
    let conn = db.lecture()?;
    let _me = autoriser(&auth, &conn, &token, Acces::Module("settings", "exporter"))?;
    let chemin = dirs
        .exports()
        .join(format!("supercaisse_export_{}.db", horodatage()));
    copier_base(&conn, &chemin)?;
    Ok(chemin.to_string_lossy().to_string())
}

#[tauri::command(async)]
pub fn list_backups(
    db: State<DbState>,
    dirs: State<AppDirs>,
    auth: State<AuthState>,
    token: String,
) -> Result<Vec<super::contrats::FichierSauvegarde>, String> {
    let conn = db.lecture()?;
    let _me = autoriser(&auth, &conn, &token, Acces::Admin)?;
    let mut fichiers = Vec::new();
    for dossier in [dirs.backups(), dirs.exports()] {
        let Ok(entrees) = std::fs::read_dir(&dossier) else {
            continue;
        };
        for entree in entrees.filter_map(Result::ok) {
            let chemin = entree.path();
            if chemin.extension().map(|e| e == "db").unwrap_or(false) {
                let meta = entree.metadata().map_err(|e| e.to_string())?;
                let modifie = meta.modified().ok().map(|t| {
                    chrono::DateTime::<chrono::Local>::from(t)
                        .format("%Y-%m-%d %H:%M")
                        .to_string()
                });
                fichiers.push(super::contrats::FichierSauvegarde {
                    chemin: chemin.to_string_lossy().to_string(),
                    nom: entree.file_name().to_string_lossy().to_string(),
                    taille: meta.len(),
                    date: modifie,
                });
            }
        }
    }
    fichiers.sort_by(|a, b| b.date.cmp(&a.date));
    Ok(fichiers)
}

#[tauri::command(async)]
pub fn import_database(
    db: State<DbState>,
    dirs: State<AppDirs>,
    auth: State<AuthState>,
    token: String,
    path: String,
) -> Result<String, String> {
    let mut conn = db.conn.lock().map_err(|e| e.to_string())?;
    let me = autoriser(&auth, &conn, &token, Acces::Admin)?;
    let securite = super::tracer(
        &format!("Restauration de la base depuis {}", path.trim()),
        restaurer(&mut conn, &dirs, Path::new(path.trim()), Some(me.user_id)),
    )?;
    auth.fermer_tout()?;
    Ok(securite.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn racine(nom: &str) -> PathBuf {
        let dossier = std::env::temp_dir().join(format!(
            "supercaisse_test_backup_{}_{}_{}",
            nom,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dossier).unwrap();
        dossier
    }

    fn dirs(racine: &Path) -> AppDirs {
        AppDirs {
            data: racine.join("data"),
            documents: racine.join("docs"),
        }
    }

    fn nb_clients(conn: &Connection) -> i64 {
        conn.query_row("SELECT COUNT(*) FROM clients", [], |r| r.get(0))
            .unwrap()
    }

    #[test]
    fn test_restauration_complete() {
        let r = racine("ok");
        let d = dirs(&r);
        let source = r.join("source.db");
        {
            let ancienne = init_db(source.to_str().unwrap()).unwrap();
            ancienne
                .execute("INSERT INTO clients (nom) VALUES ('Client sauvegardé')", [])
                .unwrap();
            ancienne.pragma_update(None, "user_version", 0).unwrap();
        }
        let mut conn = init_db(":memory:").unwrap();
        conn.execute("INSERT INTO clients (nom) VALUES ('A'), ('B')", [])
            .unwrap();
        let securite = restaurer(&mut conn, &d, &source, Some(1)).unwrap();
        let nom: String = conn
            .query_row("SELECT nom FROM clients", [], |r| r.get(0))
            .unwrap();
        assert_eq!(nom, "Client sauvegardé");
        assert_eq!(version_schema(&conn).unwrap(), SCHEMA_VERSION);
        let detail: String = conn
            .query_row(
                "SELECT detail FROM audit_log WHERE action = 'importer_base'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(detail.contains("sauvegarde préalable"));
        let copie = Connection::open(&securite).unwrap();
        assert_eq!(nb_clients(&copie), 2);
        std::fs::remove_dir_all(&r).unwrap();
    }

    #[test]
    fn test_restauration_et_sauvegarde_avec_lecteurs_ouverts() {
        let r = racine("lecteurs");
        let d = dirs(&r);
        let source = r.join("source.db");
        {
            let ancienne = init_db(source.to_str().unwrap()).unwrap();
            ancienne
                .execute("INSERT INTO clients (nom) VALUES ('Restauré')", [])
                .unwrap();
        }
        let courante = r.join("courante.db");
        let etat = crate::db::DbState::avec_lecteurs(
            init_db(courante.to_str().unwrap()).unwrap(),
            &courante,
            2,
        )
        .unwrap();
        etat.conn
            .lock()
            .unwrap()
            .execute("INSERT INTO clients (nom) VALUES ('Ancien')", [])
            .unwrap();
        let copie = sauvegarder(&etat.lecture().unwrap(), &d, "").unwrap();
        assert_eq!(nb_clients(&Connection::open(&copie).unwrap()), 1);
        assert_eq!(nb_clients(&etat.lecture().unwrap()), 1);
        restaurer(&mut etat.conn.lock().unwrap(), &d, &source, None).unwrap();
        let nom: String = etat
            .lecture()
            .unwrap()
            .query_row("SELECT nom FROM clients", [], |r| r.get(0))
            .unwrap();
        assert_eq!(nom, "Restauré");
        drop(etat);
        std::fs::remove_dir_all(&r).unwrap();
    }

    #[test]
    fn test_fichiers_invalides_refuses_sans_toucher_la_base() {
        let r = racine("invalides");
        let d = dirs(&r);
        let mut conn = init_db(":memory:").unwrap();
        conn.execute("INSERT INTO clients (nom) VALUES ('A')", [])
            .unwrap();

        assert!(restaurer(&mut conn, &d, &r.join("absent.db"), None)
            .unwrap_err()
            .contains("introuvable"));

        let texte = r.join("texte.db");
        std::fs::write(&texte, b"ceci n'est pas une base").unwrap();
        assert!(restaurer(&mut conn, &d, &texte, None).is_err());

        let etrangere = r.join("etrangere.db");
        Connection::open(&etrangere)
            .unwrap()
            .execute_batch("CREATE TABLE notes (t TEXT)")
            .unwrap();
        assert!(restaurer(&mut conn, &d, &etrangere, None)
            .unwrap_err()
            .contains("pas une sauvegarde SuperCaisse"));

        let future = r.join("future.db");
        {
            let c = init_db(future.to_str().unwrap()).unwrap();
            c.pragma_update(None, "user_version", SCHEMA_VERSION + 1)
                .unwrap();
        }
        assert!(restaurer(&mut conn, &d, &future, None)
            .unwrap_err()
            .contains("plus récente"));

        let sans_admin = r.join("sans_admin.db");
        {
            let c = init_db(sans_admin.to_str().unwrap()).unwrap();
            c.execute("UPDATE utilisateurs SET role = 'caissier'", [])
                .unwrap();
        }
        assert!(restaurer(&mut conn, &d, &sans_admin, None)
            .unwrap_err()
            .contains("aucun administrateur"));

        assert_eq!(nb_clients(&conn), 1);
        assert!(!d.backups().exists() || std::fs::read_dir(d.backups()).unwrap().count() == 0);
        std::fs::remove_dir_all(&r).unwrap();
    }

    #[test]
    fn test_migration_en_echec_remet_la_base_actuelle() {
        let r = racine("retour_arriere");
        let d = dirs(&r);
        let bancale = r.join("bancale.db");
        Connection::open(&bancale)
            .unwrap()
            .execute_batch(
                "
                CREATE TABLE articles (id INTEGER PRIMARY KEY);
                CREATE TABLE ventes (id INTEGER PRIMARY KEY);
                CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT);
                CREATE TABLE utilisateurs (id INTEGER PRIMARY KEY, login TEXT, password_hash TEXT, nom TEXT, role TEXT);
                INSERT INTO utilisateurs (login, password_hash, nom, role) VALUES ('admin', 'x', 'Admin', 'admin');
            ",
            )
            .unwrap();
        let fichier_courant = r.join("courante.db");
        let mut conn = init_db(fichier_courant.to_str().unwrap()).unwrap();
        conn.execute("INSERT INTO clients (nom) VALUES ('A')", [])
            .unwrap();
        let err = restaurer(&mut conn, &d, &bancale, None).unwrap_err();
        assert!(err.contains("Restauration annulée"), "{err}");
        assert_eq!(nb_clients(&conn), 1);
        assert_eq!(version_schema(&conn).unwrap(), SCHEMA_VERSION);
        let mode: String = conn
            .query_row("PRAGMA journal_mode", [], |r| r.get(0))
            .unwrap();
        assert_eq!(mode, "wal");
        drop(conn);
        std::fs::remove_dir_all(&r).unwrap();
    }

    #[test]
    fn test_sauvegarde_quotidienne_et_rotation() {
        let r = racine("rotation");
        let d = dirs(&r);
        let fichier = r.join("courante.db");
        let conn = init_db(fichier.to_str().unwrap()).unwrap();
        let premiere = sauvegarde_quotidienne(&conn, &d).unwrap().unwrap();
        verifier_sauvegarde(&premiere).unwrap();
        let mode: String = Connection::open(&premiere)
            .unwrap()
            .query_row("PRAGMA journal_mode", [], |r| r.get(0))
            .unwrap();
        assert_eq!(mode, "delete");
        assert!(sauvegarde_quotidienne(&conn, &d).unwrap().is_none());
        for i in 0..40 {
            std::fs::write(
                d.backups()
                    .join(format!("auto_supercaisse_2020{:04}_000000_000.db", i)),
                b"x",
            )
            .unwrap();
        }
        std::fs::write(d.backups().join("supercaisse_manuelle.db"), b"x").unwrap();
        purger_sauvegardes_auto(&d.backups(), SAUVEGARDES_AUTO_CONSERVEES).unwrap();
        let noms: Vec<String> = std::fs::read_dir(d.backups())
            .unwrap()
            .map(|f| f.unwrap().file_name().to_string_lossy().to_string())
            .collect();
        assert_eq!(
            noms.iter().filter(|n| n.starts_with("auto_")).count(),
            SAUVEGARDES_AUTO_CONSERVEES
        );
        assert!(noms.contains(&"supercaisse_manuelle.db".to_string()));
        assert!(!noms
            .iter()
            .any(|n| n.ends_with("-wal") || n.ends_with("-shm")));
        assert!(noms.iter().any(|n| n.starts_with(&format!(
            "auto_supercaisse_{}",
            chrono::Local::now().format("%Y%m%d")
        ))));
        std::fs::remove_dir_all(&r).unwrap();
    }
}
