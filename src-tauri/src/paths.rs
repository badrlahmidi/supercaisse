use rusqlite::{backup::Backup, Connection, OpenFlags};
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const DB_FILE_NAME: &str = "supercaisse.db";

pub struct AppDirs {
    pub data: PathBuf,
    pub documents: PathBuf,
}

impl AppDirs {
    pub fn database(&self) -> PathBuf {
        self.data.join(DB_FILE_NAME)
    }

    pub fn backups(&self) -> PathBuf {
        self.data.join("backups")
    }

    pub fn exports(&self) -> PathBuf {
        self.documents.join("exports")
    }

    pub fn pdf_documents(&self) -> PathBuf {
        self.documents.join("documents")
    }
}

pub fn ensure_dir(dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir)
        .map_err(|e| format!("Impossible de créer le dossier {} : {}", dir.display(), e))
}

pub fn legacy_database_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("data").join(DB_FILE_NAME));
    }
    if let Some(exe_dir) = std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf)) {
        candidates.push(exe_dir.join("data").join(DB_FILE_NAME));
    }
    candidates.dedup();
    candidates
}

pub fn prepare_database(dirs: &AppDirs, legacy_candidates: &[PathBuf]) -> Result<PathBuf, String> {
    ensure_dir(&dirs.data)?;
    let target = dirs.database();
    if target.exists() {
        return Ok(target);
    }
    if let Some(legacy) = legacy_candidates.iter().find(|p| p.is_file() && **p != target) {
        copy_sqlite_database(legacy, &target)?;
        log::info!("Base migrée de {} vers {}", legacy.display(), target.display());
    }
    Ok(target)
}

fn copy_sqlite_database(source: &Path, target: &Path) -> Result<(), String> {
    let tmp = target.with_extension("db.migration");
    let _ = std::fs::remove_file(&tmp);
    let result = (|| -> Result<(), String> {
        let src = Connection::open_with_flags(source, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|e| format!("Lecture de l'ancienne base {} impossible : {}", source.display(), e))?;
        let mut dst = Connection::open(&tmp).map_err(|e| e.to_string())?;
        Backup::new(&src, &mut dst)
            .map_err(|e| e.to_string())?
            .run_to_completion(64, Duration::from_millis(50), None)
            .map_err(|e| format!("Copie de l'ancienne base impossible : {}", e))?;
        Ok(())
    })();
    if let Err(e) = result {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }
    std::fs::rename(&tmp, target).map_err(|e| format!("Finalisation de la migration impossible : {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "supercaisse_paths_{}_{}_{}",
            name,
            std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn dirs_in(root: &Path) -> AppDirs {
        AppDirs { data: root.join("appdata"), documents: root.join("docs") }
    }

    #[test]
    fn test_nouvelle_installation_cree_le_dossier() {
        let root = temp_root("new");
        let dirs = dirs_in(&root);
        let path = prepare_database(&dirs, &[root.join("absent").join(DB_FILE_NAME)]).unwrap();
        assert_eq!(path, root.join("appdata").join(DB_FILE_NAME));
        assert!(root.join("appdata").is_dir());
        assert!(!path.exists());
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn test_migration_depuis_ancien_dossier_data() {
        let root = temp_root("legacy");
        let legacy_dir = root.join("install").join("data");
        std::fs::create_dir_all(&legacy_dir).unwrap();
        let legacy = legacy_dir.join(DB_FILE_NAME);
        {
            let conn = Connection::open(&legacy).unwrap();
            conn.execute_batch("PRAGMA journal_mode=WAL; CREATE TABLE t (v TEXT); INSERT INTO t VALUES ('vente-1');").unwrap();
            let dirs = dirs_in(&root);
            let path = prepare_database(&dirs, &[legacy.clone()]).unwrap();
            let migrated = Connection::open(&path).unwrap();
            let v: String = migrated.query_row("SELECT v FROM t", [], |r| r.get(0)).unwrap();
            assert_eq!(v, "vente-1");
        }
        assert!(legacy.exists());
        assert!(!root.join("appdata").join("supercaisse.db.migration").exists());
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn test_base_existante_jamais_ecrasee() {
        let root = temp_root("existing");
        let dirs = dirs_in(&root);
        std::fs::create_dir_all(&dirs.data).unwrap();
        Connection::open(dirs.database()).unwrap()
            .execute_batch("CREATE TABLE t (v TEXT); INSERT INTO t VALUES ('actuelle');").unwrap();
        let legacy_dir = root.join("old");
        std::fs::create_dir_all(&legacy_dir).unwrap();
        Connection::open(legacy_dir.join(DB_FILE_NAME)).unwrap()
            .execute_batch("CREATE TABLE t (v TEXT); INSERT INTO t VALUES ('ancienne');").unwrap();
        let path = prepare_database(&dirs, &[legacy_dir.join(DB_FILE_NAME)]).unwrap();
        let v: String = Connection::open(&path).unwrap().query_row("SELECT v FROM t", [], |r| r.get(0)).unwrap();
        assert_eq!(v, "actuelle");
        std::fs::remove_dir_all(&root).unwrap();
    }
}
