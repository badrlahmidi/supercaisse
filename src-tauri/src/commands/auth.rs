use crate::db::*;
use crate::session::{autoriser, Acces, AuthState};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use tauri::State;

use super::log_audit;

pub(crate) const MOT_DE_PASSE_LONGUEUR_MIN: usize = 8;

pub(crate) fn mot_de_passe_faible(login: &str, password: &str) -> bool {
    let p = password.trim().to_lowercase();
    p.chars().count() < MOT_DE_PASSE_LONGUEUR_MIN
        || p == "admin"
        || p == login.trim().to_lowercase()
}

pub(crate) fn valider_nouveau_mot_de_passe(login: &str, password: &str) -> Result<(), String> {
    if password.chars().count() < MOT_DE_PASSE_LONGUEUR_MIN {
        return Err(format!(
            "Le mot de passe doit contenir au moins {} caractères",
            MOT_DE_PASSE_LONGUEUR_MIN
        ));
    }
    if mot_de_passe_faible(login, password) {
        return Err(
            "Mot de passe trop faible : il ne doit être ni « admin » ni identique au login"
                .to_string(),
        );
    }
    Ok(())
}

pub(crate) fn login_impl(
    conn: &Connection,
    login: &str,
    password: &str,
) -> Result<Option<Utilisateur>, String> {
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
        verification_factice(password);
        return Ok(None);
    };
    if !verify_password(password, &hash) {
        log_audit(
            conn,
            Some(id),
            "echec_connexion",
            &format!("Tentative de connexion échouée pour: {}", ulogin),
            Some("utilisateur"),
            Some(id),
        )?;
        return Ok(None);
    }
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    let conn: &Connection = &tx;
    if !hash.starts_with("$argon2") {
        conn.execute(
            "UPDATE utilisateurs SET password_hash = ?1 WHERE id = ?2",
            params![hash_password(password)?, id],
        )
        .map_err(|e| e.to_string())?;
    }
    let must_change_password = must_change || mot_de_passe_faible(&ulogin, password);
    if must_change_password && !must_change {
        conn.execute(
            "UPDATE utilisateurs SET must_change_password = 1 WHERE id = ?1",
            params![id],
        )
        .map_err(|e| e.to_string())?;
    }
    conn.execute(
        "UPDATE utilisateurs SET pin_echecs = 0, pin_bloque_jusqua = NULL WHERE id = ?1 AND (pin_echecs != 0 OR pin_bloque_jusqua IS NOT NULL)",
        params![id],
    )
    .map_err(|e| e.to_string())?;
    log_audit(
        conn,
        Some(id),
        "connexion",
        &format!("Connexion réussie: {} ({})", nom, role),
        Some("utilisateur"),
        Some(id),
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(Some(Utilisateur {
        id: Some(id),
        login: ulogin,
        nom,
        role,
        must_change_password,
    }))
}

#[derive(Debug, Serialize)]
pub struct Connexion {
    #[serde(flatten)]
    pub utilisateur: Utilisateur,
    pub token: String,
}

fn ouvrir_session(
    auth: &AuthState,
    utilisateur: Option<Utilisateur>,
) -> Result<Option<Connexion>, String> {
    match utilisateur {
        None => Ok(None),
        Some(u) => {
            let token = auth.ouvrir(u.id.ok_or("Utilisateur sans identifiant")?, &u.role)?;
            Ok(Some(Connexion {
                utilisateur: u,
                token,
            }))
        }
    }
}

#[tauri::command(async)]
pub fn login(
    db: State<DbState>,
    auth: State<AuthState>,
    login: String,
    password: String,
) -> Result<Option<Connexion>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    ouvrir_session(&auth, login_impl(&conn, &login, &password)?)
}

#[tauri::command(async)]
pub fn logout(auth: State<AuthState>, token: String) -> Result<(), String> {
    auth.fermer(&token)
}

pub(crate) fn change_password_impl(
    conn: &Connection,
    user_id: i64,
    ancien: &str,
    nouveau: &str,
) -> Result<(), String> {
    let (login, hash): (String, String) = conn
        .query_row(
            "SELECT login, password_hash FROM utilisateurs WHERE id = ?1",
            params![user_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()
        .map_err(|e| e.to_string())?
        .ok_or("Utilisateur introuvable")?;
    if !verify_password(ancien, &hash) {
        log_audit(
            conn,
            Some(user_id),
            "echec_changement_mot_de_passe",
            &format!("Ancien mot de passe incorrect pour: {}", login),
            Some("utilisateur"),
            Some(user_id),
        )?;
        return Err("Ancien mot de passe incorrect".to_string());
    }
    if ancien == nouveau {
        return Err("Le nouveau mot de passe doit être différent de l'ancien".to_string());
    }
    valider_nouveau_mot_de_passe(&login, nouveau)?;
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    let conn: &Connection = &tx;
    conn.execute(
        "UPDATE utilisateurs SET password_hash = ?1, must_change_password = 0 WHERE id = ?2",
        params![hash_password(nouveau)?, user_id],
    )
    .map_err(|e| e.to_string())?;
    log_audit(
        conn,
        Some(user_id),
        "changer_mot_de_passe",
        &format!("Mot de passe modifié: {}", login),
        Some("utilisateur"),
        Some(user_id),
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command(async)]
pub fn change_password(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    ancien_mot_de_passe: String,
    nouveau_mot_de_passe: String,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let me = autoriser(&auth, &conn, &token, Acces::Connecte)?;
    change_password_impl(
        &conn,
        me.user_id,
        &ancien_mot_de_passe,
        &nouveau_mot_de_passe,
    )
}

pub(crate) const PIN_ESSAIS_MAX: i64 = 5;
pub(crate) const PIN_BLOCAGE_MINUTES: i64 = 5;

pub(crate) fn valider_pin(pin: &str) -> Result<(), String> {
    if !(4..=6).contains(&pin.len()) || !pin.chars().all(|c| c.is_ascii_digit()) {
        return Err("Le PIN doit comporter de 4 à 6 chiffres".to_string());
    }
    let chiffres: Vec<i32> = pin.bytes().map(|b| (b - b'0') as i32).collect();
    let pas: Vec<i32> = chiffres.windows(2).map(|w| w[1] - w[0]).collect();
    let constant = pas.iter().all(|p| *p == 0);
    let suite = pas.iter().all(|p| *p == 1) || pas.iter().all(|p| *p == -1);
    if constant || suite {
        return Err(
            "PIN trop simple : évitez les chiffres identiques ou qui se suivent".to_string(),
        );
    }
    Ok(())
}

#[tauri::command(async)]
pub fn login_pin(
    db: State<DbState>,
    auth: State<AuthState>,
    login: String,
    pin: String,
) -> Result<Option<Connexion>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    ouvrir_session(&auth, login_pin_impl(&conn, &login, &pin)?)
}

struct ComptePinStocke {
    id: i64,
    login: String,
    nom: String,
    role: String,
    hash: Option<String>,
    must_change_password: bool,
    echecs: i64,
    bloque: bool,
}

pub(crate) fn login_pin_impl(
    conn: &Connection,
    login: &str,
    pin: &str,
) -> Result<Option<Utilisateur>, String> {
    let compte = conn
        .query_row(
            "SELECT id, login, nom, role, pin_hash, must_change_password, pin_echecs,
                    COALESCE(pin_bloque_jusqua > datetime('now', 'localtime'), 0)
             FROM utilisateurs WHERE login = ?1",
            params![login.trim()],
            |r| {
                Ok(ComptePinStocke {
                    id: r.get(0)?,
                    login: r.get(1)?,
                    nom: r.get(2)?,
                    role: r.get(3)?,
                    hash: r.get(4)?,
                    must_change_password: r.get(5)?,
                    echecs: r.get(6)?,
                    bloque: r.get(7)?,
                })
            },
        )
        .optional()
        .map_err(|e| e.to_string())?;
    let Some(ComptePinStocke {
        id,
        login: ulogin,
        nom,
        role,
        hash,
        must_change_password,
        echecs,
        bloque,
    }) = compte
    else {
        verification_factice(pin);
        return Ok(None);
    };
    let Some(hash) = hash.filter(|h| !h.is_empty()) else {
        verification_factice(pin);
        return Ok(None);
    };
    if bloque {
        return Err(format!(
            "PIN bloqué après {} essais incorrects : réessayez dans {} minutes ou utilisez le mot de passe",
            PIN_ESSAIS_MAX, PIN_BLOCAGE_MINUTES
        ));
    }
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    if !verify_password(pin, &hash) {
        let echecs = echecs + 1;
        if echecs >= PIN_ESSAIS_MAX {
            tx.execute(
                &format!(
                    "UPDATE utilisateurs SET pin_echecs = 0, pin_bloque_jusqua = datetime('now', 'localtime', '+{} minutes') WHERE id = ?1",
                    PIN_BLOCAGE_MINUTES
                ),
                params![id],
            )
            .map_err(|e| e.to_string())?;
        } else {
            tx.execute(
                "UPDATE utilisateurs SET pin_echecs = ?1 WHERE id = ?2",
                params![echecs, id],
            )
            .map_err(|e| e.to_string())?;
        }
        log_audit(
            &tx,
            Some(id),
            if echecs >= PIN_ESSAIS_MAX {
                "blocage_pin"
            } else {
                "echec_pin"
            },
            &format!(
                "PIN incorrect pour {} ({}/{})",
                ulogin, echecs, PIN_ESSAIS_MAX
            ),
            Some("utilisateur"),
            Some(id),
        )?;
        tx.commit().map_err(|e| e.to_string())?;
        return Ok(None);
    }
    tx.execute(
        "UPDATE utilisateurs SET pin_echecs = 0, pin_bloque_jusqua = NULL WHERE id = ?1",
        params![id],
    )
    .map_err(|e| e.to_string())?;
    log_audit(
        &tx,
        Some(id),
        "connexion_pin",
        &format!("Connexion par PIN : {} ({})", nom, role),
        Some("utilisateur"),
        Some(id),
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(Some(Utilisateur {
        id: Some(id),
        login: ulogin,
        nom,
        role,
        must_change_password,
    }))
}

#[derive(Debug, Serialize, PartialEq)]
pub struct ComptePin {
    pub login: String,
    pub nom: String,
}

#[tauri::command(async)]
pub fn get_comptes_pin(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
) -> Result<Vec<ComptePin>, String> {
    let conn = db.lecture()?;
    let _me = autoriser(&auth, &conn, &token, Acces::Connecte)?;
    let mut stmt = conn
        .prepare("SELECT login, nom FROM utilisateurs WHERE pin_hash IS NOT NULL AND pin_hash != '' ORDER BY nom")
        .map_err(|e| e.to_string())?;
    let comptes = stmt
        .query_map([], |r| {
            Ok(ComptePin {
                login: r.get(0)?,
                nom: r.get(1)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| e.to_string());
    comptes
}

#[tauri::command(async)]
pub fn set_user_pin(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    user_id: i64,
    pin: String,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let _me = autoriser(&auth, &conn, &token, Acces::Admin)?;
    let hash = if pin.is_empty() {
        String::new()
    } else {
        valider_pin(&pin)?;
        hash_password(&pin)?
    };
    conn.execute(
        "UPDATE utilisateurs SET pin_hash = ?1, pin_echecs = 0, pin_bloque_jusqua = NULL WHERE id = ?2",
        params![hash, user_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::utilisateurs::{delete_utilisateur_impl, update_utilisateur_impl};

    fn temp_db(name: &str) -> String {
        std::env::temp_dir()
            .join(format!(
                "supercaisse_test_auth_{}_{}_{}.db",
                name,
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ))
            .to_string_lossy()
            .to_string()
    }

    fn cleanup(path: &str) {
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{}", path, suffix));
        }
    }

    fn base_pin() -> Connection {
        let conn = crate::db::init_db(":memory:").unwrap();
        let h = hash_password("4826").unwrap();
        conn.execute(
            "INSERT INTO utilisateurs (id, login, password_hash, nom, role, pin_hash) VALUES
                (2, 'karim', ?1, 'Karim', 'caissier', ?2),
                (3, 'gerant', ?1, 'Gérant', 'manager', ?2)",
            params![hash_password("motdepasse-solide").unwrap(), h],
        )
        .unwrap();
        conn
    }

    #[test]
    fn test_login_inconnu_aussi_lent_qu_un_mot_de_passe_faux() {
        let conn = base_pin();
        verification_factice("amorce");
        let mesure = |login: &str| {
            let debut = std::time::Instant::now();
            assert!(login_impl(&conn, login, "mauvais-mot-de-passe")
                .unwrap()
                .is_none());
            debut.elapsed()
        };
        let inconnu = (0..3).map(|_| mesure("inconnu")).min().unwrap();
        let existant = (0..3).map(|_| mesure("karim")).min().unwrap();
        assert!(
            inconnu * 3 >= existant,
            "login inconnu {:?} contre login existant {:?}",
            inconnu,
            existant
        );
    }

    #[test]
    fn test_format_du_pin() {
        for ok in ["4826", "48261", "482619", "1357"] {
            assert!(valider_pin(ok).is_ok(), "{ok}");
        }
        for ko in [
            "123", "1234567", "12a4", "0000", "1234", "4321", "987654", "111111",
        ] {
            assert!(valider_pin(ko).is_err(), "{ko}");
        }
    }

    #[test]
    fn test_pin_lie_a_l_identifiant() {
        let conn = base_pin();
        assert_eq!(
            login_pin_impl(&conn, "karim", "4826")
                .unwrap()
                .unwrap()
                .role,
            "caissier"
        );
        assert_eq!(
            login_pin_impl(&conn, "gerant", "4826")
                .unwrap()
                .unwrap()
                .role,
            "manager"
        );
        assert!(login_pin_impl(&conn, "inconnu", "4826").unwrap().is_none());
        assert!(login_pin_impl(&conn, "admin", "4826").unwrap().is_none());
    }

    #[test]
    fn test_blocage_apres_cinq_echecs() {
        let conn = base_pin();
        for _ in 0..PIN_ESSAIS_MAX {
            assert!(login_pin_impl(&conn, "karim", "9999").unwrap().is_none());
        }
        let err = login_pin_impl(&conn, "karim", "4826").unwrap_err();
        assert!(err.contains("bloqué"), "{err}");
        assert!(login_pin_impl(&conn, "gerant", "4826").unwrap().is_some());
        let (echecs, blocages): (i64, i64) = conn
            .query_row(
                "SELECT SUM(action = 'echec_pin'), SUM(action = 'blocage_pin') FROM audit_log WHERE utilisateur_id = 2",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!((echecs, blocages), (PIN_ESSAIS_MAX - 1, 1));
        conn.execute(
            "UPDATE utilisateurs SET pin_bloque_jusqua = datetime('now', 'localtime', '-1 minutes') WHERE id = 2",
            [],
        )
        .unwrap();
        assert!(login_pin_impl(&conn, "karim", "4826").unwrap().is_some());
    }

    #[test]
    fn test_succes_remet_le_compteur_a_zero() {
        let conn = base_pin();
        for _ in 0..3 {
            login_pin_impl(&conn, "karim", "9999").unwrap();
        }
        login_pin_impl(&conn, "karim", "4826").unwrap().unwrap();
        for _ in 0..PIN_ESSAIS_MAX - 1 {
            login_pin_impl(&conn, "karim", "9999").unwrap();
        }
        assert!(login_pin_impl(&conn, "karim", "4826").unwrap().is_some());
        for _ in 0..PIN_ESSAIS_MAX {
            login_pin_impl(&conn, "karim", "9999").unwrap();
        }
        assert!(login_pin_impl(&conn, "karim", "4826").is_err());
        login_impl(&conn, "karim", "motdepasse-solide")
            .unwrap()
            .unwrap();
        assert!(login_pin_impl(&conn, "karim", "4826").unwrap().is_some());
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
            conn.execute(
                "UPDATE utilisateurs SET login = 'patron' WHERE login = 'admin'",
                [],
            )
            .unwrap();
        }
        let conn = crate::db::init_db(&path).unwrap();
        let nb: i64 = conn
            .query_row("SELECT COUNT(*) FROM utilisateurs", [], |r| r.get(0))
            .unwrap();
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
                params![hash_password("motdepasse-solide").unwrap()]).unwrap();
            delete_utilisateur_impl(&conn, 1, None).unwrap();
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
        let nb: i64 = conn
            .query_row("SELECT COUNT(*) FROM utilisateurs", [], |r| r.get(0))
            .unwrap();
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
        assert!(change_password_impl(&conn, 1, "mauvais", "Caisse-2026!")
            .unwrap_err()
            .contains("incorrect"));
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
        assert!(delete_utilisateur_impl(&conn, 1, None).is_err());
        assert!(update_utilisateur_impl(
            &conn,
            1,
            "admin",
            "Administrateur",
            "manager",
            None,
            None
        )
        .is_err());
        update_utilisateur_impl(&conn, 1, "admin", "Administrateur", "admin", None, None).unwrap();
        conn.execute("INSERT INTO utilisateurs (id, login, password_hash, nom, role) VALUES (2, 'gerant', 'x', 'Gérant', 'admin')", []).unwrap();
        update_utilisateur_impl(&conn, 1, "admin", "Administrateur", "manager", None, None)
            .unwrap();
        assert!(delete_utilisateur_impl(&conn, 2, None).is_err());
        delete_utilisateur_impl(&conn, 1, None).unwrap();
    }

    #[test]
    fn test_nouveau_mot_de_passe_utilisateur_valide() {
        let conn = crate::db::init_db(":memory:").unwrap();
        conn.execute("INSERT INTO utilisateurs (id, login, password_hash, nom, role) VALUES (2, 'karim', 'x', 'Karim', 'caissier')", []).unwrap();
        assert!(update_utilisateur_impl(
            &conn,
            2,
            "karim",
            "Karim",
            "caissier",
            Some("karim"),
            None
        )
        .is_err());
        update_utilisateur_impl(
            &conn,
            2,
            "karim",
            "Karim",
            "caissier",
            Some("Vente-Karim-1"),
            None,
        )
        .unwrap();
        assert!(login_impl(&conn, "karim", "Vente-Karim-1")
            .unwrap()
            .is_some());
    }
}
