use crate::db::*;
use crate::session::{autoriser, Acces, AuthState};
use rusqlite::{params, OptionalExtension};
use tauri::State;

use super::log_audit;

#[tauri::command(async)]
pub fn get_fournisseurs(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
) -> Result<Vec<Fournisseur>, String> {
    let conn = db.lecture()?;
    let _me = autoriser(&auth, &conn, &token, Acces::Module("fournisseurs", "voir"))?;
    let mut stmt = conn
        .prepare("SELECT id, nom, adresse, telephone, ice, email FROM fournisseurs ORDER BY nom")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok(Fournisseur {
                id: Some(row.get(0)?),
                nom: row.get(1)?,
                adresse: row.get(2)?,
                telephone: row.get(3)?,
                ice: row.get(4)?,
                email: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

#[tauri::command(async)]
pub fn add_fournisseur(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    nom: String,
    adresse: Option<String>,
    telephone: Option<String>,
    ice: Option<String>,
    email: Option<String>,
) -> Result<i64, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let _me = autoriser(&auth, &conn, &token, Acces::Module("fournisseurs", "creer"))?;
    conn.execute(
        "INSERT INTO fournisseurs (nom, adresse, telephone, ice, email) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![nom, adresse, telephone, ice, email],
    ).map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

#[tauri::command(async)]
pub fn update_fournisseur(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    id: i64,
    nom: String,
    adresse: Option<String>,
    telephone: Option<String>,
    ice: Option<String>,
    email: Option<String>,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let _me = autoriser(
        &auth,
        &conn,
        &token,
        Acces::Module("fournisseurs", "modifier"),
    )?;
    conn.execute(
        "UPDATE fournisseurs SET nom=?1, adresse=?2, telephone=?3, ice=?4, email=?5 WHERE id=?6",
        params![nom, adresse, telephone, ice, email, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command(async)]
pub fn delete_fournisseur(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    id: i64,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let me = autoriser(
        &auth,
        &conn,
        &token,
        Acces::Module("fournisseurs", "modifier"),
    )?;
    let conn = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    let nom: String = conn
        .query_row(
            "SELECT nom FROM fournisseurs WHERE id = ?1",
            params![id],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?
        .unwrap_or_else(|| format!("ID {}", id));
    conn.execute("DELETE FROM fournisseurs WHERE id=?1", params![id])
        .map_err(|e| super::erreur_suppression(e, "ce fournisseur"))?;
    log_audit(
        &conn,
        Some(me.user_id),
        "supprimer_fournisseur",
        &format!("Suppression fournisseur: {} (ID {})", nom, id),
        Some("fournisseur"),
        Some(id),
    )?;
    conn.commit().map_err(|e| e.to_string())?;
    Ok(())
}
