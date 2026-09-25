use crate::db::*;
use rusqlite::{backup::Backup, Connection};
use std::time::Duration;
use tauri::State;

use super::log_audit;

#[tauri::command]
pub fn backup_database(db: State<DbState>) -> Result<String, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let _db_path = conn.path().ok_or("Base de données non fichier")?.to_string();
    let backup_dir = std::env::current_dir()
        .map_err(|e| e.to_string())?
        .join("backups");
    std::fs::create_dir_all(&backup_dir).map_err(|e| e.to_string())?;
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let backup_path = backup_dir.join(format!("supercaisse_{}.db", timestamp));
    let mut dst = Connection::open(&backup_path).map_err(|e| e.to_string())?;
    let backup = Backup::new(&*conn, &mut dst).map_err(|e| e.to_string())?;
    backup.run_to_completion(5, Duration::from_millis(250), None).map_err(|e| e.to_string())?;
    Ok(backup_path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn export_database(db: State<DbState>) -> Result<String, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let _db_path = conn.path().ok_or("Base de données non fichier")?.to_string();
    let export_dir = std::env::current_dir()
        .map_err(|e| e.to_string())?
        .join("exports");
    std::fs::create_dir_all(&export_dir).map_err(|e| e.to_string())?;
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let export_path = export_dir.join(format!("supercaisse_export_{}.db", timestamp));
    let mut dst = Connection::open(&export_path).map_err(|e| e.to_string())?;
    let backup = Backup::new(&*conn, &mut dst).map_err(|e| e.to_string())?;
    backup.run_to_completion(5, Duration::from_millis(250), None).map_err(|e| e.to_string())?;
    Ok(export_path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn import_database(db: State<DbState>, path: String) -> Result<(), String> {
    let mut conn = db.conn.lock().map_err(|e| e.to_string())?;
    {
        let src = Connection::open(&path).map_err(|e| e.to_string())?;
        let backup = Backup::new(&src, &mut *conn).map_err(|e| e.to_string())?;
        backup.run_to_completion(5, Duration::from_millis(250), None).map_err(|e| e.to_string())?;
    }
    log_audit(&*conn, None, "importer_base",
        &format!("Import base de données depuis: {}", path),
        None, None);
    Ok(())
}
