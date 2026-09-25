use crate::db::*;
use rusqlite::{params, Connection, OptionalExtension};
use tauri::State;

use super::log_audit;

pub(crate) const MOT_DE_PASSE_LONGUEUR_MIN: usize = 8;

pub(crate) fn mot_de_passe_faible(login: &str, password: &str) -> bool {
    let p = password.trim().to_lowercase();
    p.chars().count() < MOT_DE_PASSE_LONGUEUR_MIN || p == "admin" || p == login.trim().to_lowercase()
}

pub(crate) fn valider_nouveau_mot_de_passe(login: &str, password: &str) -> Result<(), String> {
    if password.chars().count() < MOT_DE_PASSE_LONGUEUR_MIN {
        return Err(format!("Le mot de passe doit contenir au moins {} caractères", MOT_DE_PASSE_LONGUEUR_MIN));
    }
    if mot_de_passe_faible(login, password) {
        return Err("Mot de passe trop faible : il ne doit être ni « admin » ni identique au login".to_string());
    }
    Ok(())
}

pub(crate) fn login_impl(conn: &Connection, login: &str, password: &str) -> Result<Option<Utilisateur>, String> {
    let result = conn.query_row(
        "SELECT id, login, nom, role, password_hash, must_change_password FROM utilisateurs WHERE login = ?1",
        params![login],
        |row| Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, bool>(5)?,
        )),
    ).optional().map_err(|e| e.to_string())?;
    let Some((id, ulogin, nom, role, hash, must_change)) = result else {
        return Ok(None);
    };
    if !verify_password(password, &hash) {
        log_audit(conn, Some(id), "echec_connexion",
            &format!("Tentative de connexion échouée pour: {}", ulogin),
            Some("utilisateur"), Some(id));
        return Ok(None);
    }
    if !hash.starts_with("$argon2") {
        conn.execute(
            "UPDATE utilisateurs SET password_hash = ?1 WHERE id = ?2",
            params![hash_password(password), id],
        ).map_err(|e| e.to_string())?;
    }
    let must_change_password = must_change || mot_de_passe_faible(&ulogin, password);
    if must_change_password && !must_change {
        conn.execute("UPDATE utilisateurs SET must_change_password = 1 WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
    }
    log_audit(conn, Some(id), "connexion",
        &format!("Connexion réussie: {} ({})", nom, role),
        Some("utilisateur"), Some(id));
    Ok(Some(Utilisateur { id: Some(id), login: ulogin, nom, role, must_change_password }))
}

#[tauri::command]
pub fn login(db: State<DbState>, login: String, password: String) -> Result<Option<Utilisateur>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    login_impl(&conn, &login, &password)
}

pub(crate) fn change_password_impl(conn: &Connection, user_id: i64, ancien: &str, nouveau: &str) -> Result<(), String> {
    let (login, hash): (String, String) = conn.query_row(
        "SELECT login, password_hash FROM utilisateurs WHERE id = ?1",
        params![user_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    ).optional().map_err(|e| e.to_string())?.ok_or("Utilisateur introuvable")?;
    if !verify_password(ancien, &hash) {
        log_audit(conn, Some(user_id), "echec_changement_mot_de_passe",
            &format!("Ancien mot de passe incorrect pour: {}", login),
            Some("utilisateur"), Some(user_id));
        return Err("Ancien mot de passe incorrect".to_string());
    }
    if ancien == nouveau {
        return Err("Le nouveau mot de passe doit être différent de l'ancien".to_string());
    }
    valider_nouveau_mot_de_passe(&login, nouveau)?;
    conn.execute(
        "UPDATE utilisateurs SET password_hash = ?1, must_change_password = 0 WHERE id = ?2",
        params![hash_password(nouveau), user_id],
    ).map_err(|e| e.to_string())?;
    log_audit(conn, Some(user_id), "changer_mot_de_passe",
        &format!("Mot de passe modifié: {}", login),
        Some("utilisateur"), Some(user_id));
    Ok(())
}

#[tauri::command]
pub fn change_password(db: State<DbState>, user_id: i64, ancien_mot_de_passe: String, nouveau_mot_de_passe: String) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    change_password_impl(&conn, user_id, &ancien_mot_de_passe, &nouveau_mot_de_passe)
}

#[tauri::command]
pub fn login_pin(db: State<DbState>, pin: String) -> Result<Option<Utilisateur>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT id, login, nom, role, pin_hash, must_change_password FROM utilisateurs WHERE pin_hash IS NOT NULL AND pin_hash != ''"
    ).map_err(|e| e.to_string())?;
    let users: Vec<(i64, String, String, String, String, bool)> = stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, bool>(5)?,
        ))
    }).map_err(|e| e.to_string())?.filter_map(Result::ok).collect();
    for (id, ulogin, nom, role, hash, must_change_password) in users {
        if verify_password(&pin, &hash) {
            return Ok(Some(Utilisateur { id: Some(id), login: ulogin, nom, role, must_change_password }));
        }
    }
    Ok(None)
}

#[tauri::command]
pub fn set_user_pin(db: State<DbState>, user_id: i64, pin: String) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let hash = if pin.is_empty() {
        String::new()
    } else {
        hash_password(&pin)
    };
    conn.execute(
        "UPDATE utilisateurs SET pin_hash = ?1 WHERE id = ?2",
        params![hash, user_id],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::utilisateurs::{delete_utilisateur_impl, update_utilisateur_impl};

    fn temp_db(name: &str) -> String {
        std::env::temp_dir().join(format!(
            "supercaisse_test_auth_{}_{}_{}.db",
            name,
            std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        )).to_string_lossy().to_string()
    }

    fn cleanup(path: &str) {
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{}", path, suffix));
        }
    }

    #[test]
    fn test_premiere_installation_impose_le_changement() {
        let conn = crate::db::init_db(":memory:").unwrap();
        let admin = login_impl(&conn, "admin", "admin").unwrap().unwrap();
        assert_eq!(admin.role, "admin");
        assert!(admin.must_change_password);
    }

    #[test]
    fn test_admin_renomme_pas_recree() {
        let path = temp_db("rename");
        {
            let conn = crate::db::init_db(&path).unwrap();
            conn.execute("UPDATE utilisateurs SET login = 'patron' WHERE login = 'admin'", []).unwrap();
        }
        let conn = crate::db::init_db(&path).unwrap();
        let nb: i64 = conn.query_row("SELECT COUNT(*) FROM utilisateurs", [], |r| r.get(0)).unwrap();
        assert_eq!(nb, 1);
        assert!(login_impl(&conn, "admin", "admin").unwrap().is_none());
        assert!(login_impl(&conn, "patron", "admin").unwrap().is_some());
        drop(conn);
        cleanup(&path);
    }

    #[test]
    fn test_admin_supprime_pas_recree_si_autres_utilisateurs() {
        let path = temp_db("delete");
        {
            let conn = crate::db::init_db(&path).unwrap();
            conn.execute("INSERT INTO utilisateurs (login, password_hash, nom, role) VALUES ('gerant', ?1, 'Gérant', 'admin')",
                params![hash_password("motdepasse-solide")]).unwrap();
            delete_utilisateur_impl(&conn, 1).unwrap();
        }
        let conn = crate::db::init_db(&path).unwrap();
        assert!(login_impl(&conn, "admin", "admin").unwrap().is_none());
        drop(conn);
        cleanup(&path);
    }

    #[test]
    fn test_base_existante_avec_mot_de_passe_par_defaut() {
        let path = temp_db("legacy");
        {
            let legacy = Connection::open(&path).unwrap();
            use sha2::Digest;
            let sha = hex::encode(sha2::Sha256::digest(b"admin"));
            legacy.execute_batch(&format!("
                CREATE TABLE utilisateurs (id INTEGER PRIMARY KEY AUTOINCREMENT, login TEXT NOT NULL UNIQUE,
                    password_hash TEXT NOT NULL, nom TEXT NOT NULL, role TEXT DEFAULT 'caissier');
                INSERT INTO utilisateurs (login, password_hash, nom, role) VALUES ('admin', '{}', 'Administrateur', 'admin');
            ", sha)).unwrap();
        }
        let conn = crate::db::init_db(&path).unwrap();
        let nb: i64 = conn.query_row("SELECT COUNT(*) FROM utilisateurs", [], |r| r.get(0)).unwrap();
        assert_eq!(nb, 1);
        let admin = login_impl(&conn, "admin", "admin").unwrap().unwrap();
        assert!(admin.must_change_password);
        let (hash, flag): (String, bool) = conn.query_row(
            "SELECT password_hash, must_change_password FROM utilisateurs WHERE login = 'admin'", [],
            |r| Ok((r.get(0)?, r.get(1)?))).unwrap();
        assert!(hash.starts_with("$argon2"));
        assert!(flag);
        drop(conn);
        cleanup(&path);
    }

    #[test]
    fn test_changement_de_mot_de_passe() {
        let conn = crate::db::init_db(":memory:").unwrap();
        assert!(change_password_impl(&conn, 1, "mauvais", "Caisse-2026!").unwrap_err().contains("incorrect"));
        assert!(change_password_impl(&conn, 1, "admin", "admin").is_err());
        assert!(change_password_impl(&conn, 1, "admin", "court").is_err());
        assert!(change_password_impl(&conn, 1, "admin", "ADMIN").is_err());
        assert!(change_password_impl(&conn, 999, "admin", "Caisse-2026!").is_err());
        change_password_impl(&conn, 1, "admin", "Caisse-2026!").unwrap();
        assert!(login_impl(&conn, "admin", "admin").unwrap().is_none());
        let admin = login_impl(&conn, "admin", "Caisse-2026!").unwrap().unwrap();
        assert!(!admin.must_change_password);
    }

    #[test]
    fn test_mot_de_passe_faible() {
        assert!(mot_de_passe_faible("admin", "admin"));
        assert!(mot_de_passe_faible("karim", "Karim"));
        assert!(mot_de_passe_faible("karim", "1234567"));
        assert!(mot_de_passe_faible("caissier01", "CAISSIER01"));
        assert!(!mot_de_passe_faible("karim", "Caisse-2026!"));
        assert!(valider_nouveau_mot_de_passe("karim", "1234567").is_err());
        assert!(valider_nouveau_mot_de_passe("karim", "12345678").is_ok());
    }

    #[test]
    fn test_dernier_admin_protege() {
        let conn = crate::db::init_db(":memory:").unwrap();
        assert!(delete_utilisateur_impl(&conn, 1).is_err());
        assert!(update_utilisateur_impl(&conn, 1, "admin", "Administrateur", "manager", None).is_err());
        update_utilisateur_impl(&conn, 1, "admin", "Administrateur", "admin", None).unwrap();
        conn.execute("INSERT INTO utilisateurs (id, login, password_hash, nom, role) VALUES (2, 'gerant', 'x', 'Gérant', 'admin')", []).unwrap();
        update_utilisateur_impl(&conn, 1, "admin", "Administrateur", "manager", None).unwrap();
        assert!(delete_utilisateur_impl(&conn, 2).is_err());
        delete_utilisateur_impl(&conn, 1).unwrap();
    }

    #[test]
    fn test_nouveau_mot_de_passe_utilisateur_valide() {
        let conn = crate::db::init_db(":memory:").unwrap();
        conn.execute("INSERT INTO utilisateurs (id, login, password_hash, nom, role) VALUES (2, 'karim', 'x', 'Karim', 'caissier')", []).unwrap();
        assert!(update_utilisateur_impl(&conn, 2, "karim", "Karim", "caissier", Some("karim")).is_err());
        update_utilisateur_impl(&conn, 2, "karim", "Karim", "caissier", Some("Vente-Karim-1")).unwrap();
        assert!(login_impl(&conn, "karim", "Vente-Karim-1").unwrap().is_some());
    }
}
