use crate::db::*;
use crate::session::{autoriser, Acces, AuthState};
use rusqlite::{params, Connection, OptionalExtension};
use tauri::State;

use super::auth::valider_nouveau_mot_de_passe;
use super::log_audit;

#[tauri::command(async)]
pub fn get_utilisateurs(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
) -> Result<Vec<Utilisateur>, String> {
    let conn = db.lecture()?;
    let _me = autoriser(&auth, &conn, &token, Acces::Admin)?;
    let mut stmt = conn
        .prepare("SELECT id, login, nom, role, must_change_password FROM utilisateurs ORDER BY nom")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok(Utilisateur {
                id: Some(row.get(0)?),
                login: row.get(1)?,
                nom: row.get(2)?,
                role: row.get(3)?,
                must_change_password: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

#[tauri::command(async)]
pub fn add_utilisateur(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    login: String,
    nom: String,
    role: String,
    password: String,
) -> Result<i64, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let me = autoriser(&auth, &conn, &token, Acces::Admin)?;
    let conn = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    super::valeur_autorisee("Rôle", &role, ROLES)?;
    valider_nouveau_mot_de_passe(&login, &password)?;
    let hash = hash_password(&password)?;
    conn.execute(
        "INSERT INTO utilisateurs (login, password_hash, nom, role) VALUES (?1, ?2, ?3, ?4)",
        params![login, hash, nom, role],
    )
    .map_err(|e| e.to_string())?;
    let id = conn.last_insert_rowid();
    super::tracer_creation(&conn, "utilisateurs", id, Some(me.user_id))?;
    log_audit(
        &conn,
        Some(me.user_id),
        "ajouter_utilisateur",
        &format!("Nouvel utilisateur: {} ({}) - rôle {}", nom, login, role),
        Some("utilisateur"),
        Some(id),
    )?;
    conn.commit().map_err(|e| e.to_string())?;
    Ok(id)
}

#[tauri::command(async)]
pub fn update_utilisateur(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    id: i64,
    login: String,
    nom: String,
    role: String,
    password: Option<String>,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let me = autoriser(&auth, &conn, &token, Acces::Admin)?;
    update_utilisateur_impl(
        &conn,
        id,
        &login,
        &nom,
        &role,
        password.as_deref(),
        Some(me.user_id),
    )?;
    if password.as_deref().is_some_and(|p| !p.is_empty()) && id != me.user_id {
        auth.fermer_utilisateur(id)?;
    }
    Ok(())
}

pub(crate) fn verifier_admin_restant(conn: &Connection, id_retire: i64) -> Result<(), String> {
    let autres_admins: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM utilisateurs WHERE role = 'admin' AND id != ?1",
            params![id_retire],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    let est_admin: bool = conn
        .query_row(
            "SELECT COUNT(*) > 0 FROM utilisateurs WHERE id = ?1 AND role = 'admin'",
            params![id_retire],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if est_admin && autres_admins == 0 {
        return Err("Impossible : il doit rester au moins un administrateur".to_string());
    }
    Ok(())
}

pub(crate) fn update_utilisateur_impl(
    conn: &Connection,
    id: i64,
    login: &str,
    nom: &str,
    role: &str,
    password: Option<&str>,
    auteur: Option<i64>,
) -> Result<(), String> {
    super::valeur_autorisee("Rôle", role, ROLES)?;
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    let conn: &Connection = &tx;
    if role != "admin" {
        verifier_admin_restant(conn, id)?;
    }
    match password {
        Some(pwd) if !pwd.is_empty() => {
            valider_nouveau_mot_de_passe(login, pwd)?;
            let hash = hash_password(pwd)?;
            conn.execute(
                "UPDATE utilisateurs SET login=?1, nom=?2, role=?3, password_hash=?4 WHERE id=?5",
                params![login, nom, role, hash, id],
            )
            .map_err(|e| e.to_string())?;
        }
        _ => {
            conn.execute(
                "UPDATE utilisateurs SET login=?1, nom=?2, role=?3 WHERE id=?4",
                params![login, nom, role, id],
            )
            .map_err(|e| e.to_string())?;
        }
    }
    super::tracer_modification(conn, "utilisateurs", id, auteur)?;
    log_audit(
        conn,
        auteur,
        "modifier_utilisateur",
        &format!(
            "Modification utilisateur #{}: {} ({}) - rôle {}",
            id, nom, login, role
        ),
        Some("utilisateur"),
        Some(id),
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command(async)]
pub fn delete_utilisateur(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    id: i64,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let me = autoriser(&auth, &conn, &token, Acces::Admin)?;
    delete_utilisateur_impl(&conn, id, Some(me.user_id))
}

pub(crate) fn delete_utilisateur_impl(
    conn: &Connection,
    id: i64,
    auteur: Option<i64>,
) -> Result<(), String> {
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    let conn: &Connection = &tx;
    verifier_admin_restant(conn, id)?;
    let nom: String = conn
        .query_row(
            "SELECT nom FROM utilisateurs WHERE id = ?1",
            params![id],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?
        .unwrap_or_else(|| format!("ID {}", id));
    conn.execute("DELETE FROM utilisateurs WHERE id=?1", params![id])
        .map_err(|e| super::erreur_suppression(e, "cet utilisateur"))?;
    log_audit(
        conn,
        auteur,
        "supprimer_utilisateur",
        &format!("Suppression utilisateur: {} (ID {})", nom, id),
        Some("utilisateur"),
        Some(id),
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> Connection {
        let conn = init_db(":memory:").unwrap();
        conn.execute(
            "INSERT INTO utilisateurs (id, login, password_hash, nom, role) VALUES (2, 'caissier', 'x', 'Caissier', 'caissier')",
            [],
        )
        .unwrap();
        conn
    }

    #[test]
    fn test_audit_en_echec_annule_l_operation() {
        let conn = base();
        conn.execute_batch(
            "CREATE TRIGGER audit_bloque BEFORE INSERT ON audit_log BEGIN SELECT RAISE(ABORT, 'disque plein'); END;",
        )
        .unwrap();
        let err = delete_utilisateur_impl(&conn, 2, Some(1)).unwrap_err();
        assert!(err.contains("Journal d'audit"), "{err}");
        let existe: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM utilisateurs WHERE id = 2",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(existe);
        assert!(update_utilisateur_impl(
            &conn,
            2,
            "caissier",
            "Renommé",
            "caissier",
            None,
            Some(1)
        )
        .is_err());
        let nom: String = conn
            .query_row("SELECT nom FROM utilisateurs WHERE id = 2", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(nom, "Caissier");
    }

    #[test]
    fn test_suppression_journalisee() {
        let conn = base();
        delete_utilisateur_impl(&conn, 2, Some(1)).unwrap();
        let n: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM audit_log WHERE action = 'supprimer_utilisateur'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n, 1);
    }
}
