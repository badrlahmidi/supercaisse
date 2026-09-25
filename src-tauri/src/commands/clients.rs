use crate::db::*;
use crate::session::{autoriser, Acces, AuthState};
use rusqlite::{params, Connection, OptionalExtension};
use tauri::State;

use super::calcul::montant_positif;
use super::log_audit;

fn erreur_code_client(erreur: rusqlite::Error, code: Option<&str>) -> String {
    match (&erreur, code) {
        (rusqlite::Error::SqliteFailure(e, _), Some(code))
            if e.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE =>
        {
            format!("Le code client « {} » est déjà utilisé", code)
        }
        _ => erreur.to_string(),
    }
}

#[tauri::command(async)]
pub fn get_clients(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
) -> Result<Vec<Client>, String> {
    let conn = db.lecture()?;
    let _me = autoriser(&auth, &conn, &token, Acces::Module("clients", "voir"))?;
    let mut stmt = conn.prepare("SELECT id, code, nom, adresse, telephone, email, credit_plafond, credit_actuel, ice, segment FROM clients ORDER BY nom")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok(Client {
                id: Some(row.get(0)?),
                code: row.get(1)?,
                nom: row.get(2)?,
                adresse: row.get(3)?,
                telephone: row.get(4)?,
                email: row.get(5)?,
                credit_plafond: row.get(6)?,
                credit_actuel: row.get(7)?,
                ice: row.get(8)?,
                segment: row.get(9)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

#[tauri::command(async)]
pub fn add_client(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    code: Option<String>,
    nom: String,
    adresse: Option<String>,
    telephone: Option<String>,
    email: Option<String>,
    credit_plafond: Option<f64>,
    ice: Option<String>,
    segment: Option<String>,
) -> Result<i64, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let me = autoriser(&auth, &conn, &token, Acces::Module("clients", "creer"))?;
    let credit_plafond = credit_plafond
        .map(|m| montant_positif("Plafond de crédit", m))
        .transpose()?;
    let code = code.map(|c| c.trim().to_string()).filter(|c| !c.is_empty());
    let ice = super::fiscal::normaliser_ice("ICE du client", ice)?;
    let conn = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO clients (code, nom, adresse, telephone, email, credit_plafond, ice, segment) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![code, nom, adresse, telephone, email, credit_plafond, ice, segment],
    ).map_err(|e| erreur_code_client(e, code.as_deref()))?;
    let id = conn.last_insert_rowid();
    super::tracer_creation(&conn, "clients", id, Some(me.user_id))?;
    if code.is_none() {
        attribuer_code_client(&conn, id)?;
    }
    conn.commit().map_err(|e| e.to_string())?;
    Ok(id)
}

pub(crate) fn attribuer_code_client(conn: &Connection, id: i64) -> Result<String, String> {
    let base = format!("CLI-{:05}", id);
    for essai in 0..100 {
        let code = if essai == 0 {
            base.clone()
        } else {
            format!("{}-{}", base, essai)
        };
        let libre: bool = conn
            .query_row(
                "SELECT COUNT(*) = 0 FROM clients WHERE code = ?1",
                params![code],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        if libre {
            conn.execute(
                "UPDATE clients SET code = ?1 WHERE id = ?2",
                params![code, id],
            )
            .map_err(|e| e.to_string())?;
            return Ok(code);
        }
    }
    Err(format!("Impossible d'attribuer un code au client {}", id))
}

#[tauri::command(async)]
pub fn update_client(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    id: i64,
    code: Option<String>,
    nom: String,
    adresse: Option<String>,
    telephone: Option<String>,
    email: Option<String>,
    credit_plafond: Option<f64>,
    ice: Option<String>,
    segment: Option<String>,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let me = autoriser(&auth, &conn, &token, Acces::Module("clients", "modifier"))?;
    let credit_plafond = credit_plafond
        .map(|m| montant_positif("Plafond de crédit", m))
        .transpose()?;
    let code = code.map(|c| c.trim().to_string()).filter(|c| !c.is_empty());
    let ice = super::fiscal::normaliser_ice("ICE du client", ice)?;
    conn.execute(
        "UPDATE clients SET code=?1, nom=?2, adresse=?3, telephone=?4, email=?5, credit_plafond=?6, ice=?7, segment=?8 WHERE id=?9",
        params![code, nom, adresse, telephone, email, credit_plafond, ice, segment, id],
    ).map_err(|e| erreur_code_client(e, code.as_deref()))?;
    super::tracer_modification(&conn, "clients", id, Some(me.user_id))?;
    Ok(())
}

#[tauri::command(async)]
pub fn delete_client(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    id: i64,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let me = autoriser(&auth, &conn, &token, Acces::Module("clients", "modifier"))?;
    let conn = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    let nom: String = conn
        .query_row("SELECT nom FROM clients WHERE id = ?1", params![id], |r| {
            r.get(0)
        })
        .optional()
        .map_err(|e| e.to_string())?
        .unwrap_or_else(|| format!("ID {}", id));
    conn.execute("DELETE FROM clients WHERE id=?1", params![id])
        .map_err(|e| {
            super::erreur_suppression(e, "ce client").replace(
                "…)",
                "…) : les pièces comptables qui le mentionnent doivent être conservées 10 ans",
            )
        })?;
    log_audit(
        &conn,
        Some(me.user_id),
        "supprimer_client",
        &format!("Suppression client: {} (ID {})", nom, id),
        Some("client"),
        Some(id),
    )?;
    conn.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command(async)]
pub fn get_releve_client(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    client_id: i64,
) -> Result<serde_json::Value, String> {
    let conn = db.lecture()?;
    let _me = autoriser(&auth, &conn, &token, Acces::Module("clients", "voir"))?;

    let (nom, credit_actuel, credit_plafond): (String, f64, f64) = conn.query_row(
        "SELECT nom, COALESCE(credit_actuel, 0), COALESCE(credit_plafond, 0) FROM clients WHERE id = ?1",
        params![client_id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    ).map_err(|_| "Client introuvable".to_string())?;

    let mut stmt = conn.prepare(
        "SELECT v.id, v.date, v.numero_facture, v.montant_total, v.montant_remise, v.mode_paiement, v.statut, v.dtype
         FROM ventes v WHERE v.client_id = ?1
         ORDER BY v.date DESC LIMIT 200"
    ).map_err(|e| e.to_string())?;
    let ventes = stmt
        .query_map(params![client_id], |r| {
            Ok(serde_json::json!({
                "id": r.get::<_, i64>(0)?,
                "date": r.get::<_, String>(1)?,
                "numero_facture": r.get::<_, Option<String>>(2)?,
                "montant_total": r.get::<_, f64>(3)?,
                "montant_remise": r.get::<_, f64>(4)?,
                "mode_paiement": r.get::<_, String>(5)?,
                "statut": r.get::<_, String>(6)?,
                "dtype": r.get::<_, Option<String>>(7)?
            }))
        })
        .map_err(|e| e.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| e.to_string())?;

    let mut stmt2 = conn
        .prepare(
            "SELECT p.id, p.date, p.montant, p.type, p.reference
         FROM paiements p WHERE p.client_id = ?1
         ORDER BY p.date DESC LIMIT 200",
        )
        .map_err(|e| e.to_string())?;
    let paiements = stmt2
        .query_map(params![client_id], |r| {
            Ok(serde_json::json!({
                "id": r.get::<_, i64>(0)?,
                "date": r.get::<_, String>(1)?,
                "montant": r.get::<_, f64>(2)?,
                "type": r.get::<_, String>(3)?,
                "reference": r.get::<_, Option<String>>(4)?
            }))
        })
        .map_err(|e| e.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| e.to_string())?;

    Ok(serde_json::json!({
        "client_id": client_id,
        "nom": nom,
        "credit_actuel": credit_actuel,
        "credit_plafond": credit_plafond,
        "ventes": ventes,
        "paiements": paiements,
    }))
}

#[tauri::command(async)]
pub fn get_mouvements_fidelite(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    client_id: i64,
) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.lecture()?;
    let _me = autoriser(&auth, &conn, &token, Acces::Module("clients", "voir"))?;
    let mut stmt = conn.prepare(
        "SELECT mf.id, mf.client_id, mf.vente_id, mf.points, mf.mtype, mf.date, v.numero_facture
         FROM mouvements_fidelite mf
         LEFT JOIN ventes v ON v.id = mf.vente_id
         WHERE mf.client_id = ?1
         ORDER BY mf.date DESC
         LIMIT 100"
    ).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![client_id], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, i64>(0)?,
                "client_id": row.get::<_, i64>(1)?,
                "vente_id": row.get::<_, Option<i64>>(2)?,
                "points": row.get::<_, f64>(3)?,
                "mtype": row.get::<_, String>(4)?,
                "date": row.get::<_, String>(5)?,
                "numero_facture": row.get::<_, Option<String>>(6)?
            }))
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_client_genere_sans_collision() {
        let conn = init_db(":memory:").unwrap();
        conn.execute(
            "INSERT INTO clients (id, nom, code) VALUES (1, 'Saisi', 'CLI-00002')",
            [],
        )
        .unwrap();
        conn.execute("INSERT INTO clients (id, nom) VALUES (2, 'Auto')", [])
            .unwrap();
        assert_eq!(attribuer_code_client(&conn, 2).unwrap(), "CLI-00002-1");
        conn.execute("INSERT INTO clients (id, nom) VALUES (3, 'Auto')", [])
            .unwrap();
        assert_eq!(attribuer_code_client(&conn, 3).unwrap(), "CLI-00003");
        let err = conn
            .execute(
                "INSERT INTO clients (nom, code) VALUES ('X', 'CLI-00003')",
                [],
            )
            .map_err(|e| erreur_code_client(e, Some("CLI-00003")))
            .unwrap_err();
        assert_eq!(err, "Le code client « CLI-00003 » est déjà utilisé");
    }
}
