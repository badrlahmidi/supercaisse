use crate::db::*;
use crate::session::{autoriser, Acces, AuthState};
use rusqlite::params;
use tauri::State;

#[tauri::command]
pub fn get_cheques(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let _me = autoriser(&auth, &conn, &token, Acces::Module("cheques", "voir"))?;
    let mut stmt = conn.prepare(
        "SELECT c.id, c.numero, c.banque, c.tireur, c.montant, c.date_emission, c.date_echeance, c.statut, c.ctype, c.client_id, c.fournisseur_id,
                cl.nom as client_nom, f.nom as fournisseur_nom
         FROM cheques c
         LEFT JOIN clients cl ON c.client_id = cl.id
         LEFT JOIN fournisseurs f ON c.fournisseur_id = f.id
         ORDER BY c.date_echeance ASC"
    ).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, i64>(0)?,
                "numero": row.get::<_, String>(1)?,
                "banque": row.get::<_, String>(2)?,
                "tireur": row.get::<_, Option<String>>(3)?,
                "montant": row.get::<_, f64>(4)?,
                "date_emission": row.get::<_, String>(5)?,
                "date_echeance": row.get::<_, String>(6)?,
                "statut": row.get::<_, String>(7)?,
                "ctype": row.get::<_, String>(8)?,
                "client_id": row.get::<_, Option<i64>>(9)?,
                "fournisseur_id": row.get::<_, Option<i64>>(10)?,
                "client_nom": row.get::<_, Option<String>>(11)?,
                "fournisseur_nom": row.get::<_, Option<String>>(12)?,
            }))
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_cheque(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    numero: String,
    banque: String,
    tireur: Option<String>,
    montant: f64,
    date_emission: String,
    date_echeance: String,
    ctype: String,
    client_id: Option<i64>,
    fournisseur_id: Option<i64>,
) -> Result<i64, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let _me = autoriser(&auth, &conn, &token, Acces::Module("cheques", "creer"))?;
    conn.execute(
        "INSERT INTO cheques (numero, banque, tireur, montant, date_emission, date_echeance, ctype, client_id, fournisseur_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![numero, banque, tireur, montant, date_emission, date_echeance, ctype, client_id, fournisseur_id],
    ).map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

#[tauri::command]
pub fn update_cheque_status(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    cheque_id: i64,
    statut: String,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let _me = autoriser(&auth, &conn, &token, Acces::Module("cheques", "modifier"))?;
    conn.execute(
        "UPDATE cheques SET statut = ?1 WHERE id = ?2",
        params![statut, cheque_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
