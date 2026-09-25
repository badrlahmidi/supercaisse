use rand::RngCore;
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub const INACTIVITE_MAX: Duration = Duration::from_secs(60 * 60);
pub const DUREE_MAX: Duration = Duration::from_secs(16 * 60 * 60);

#[derive(Debug, Clone, PartialEq)]
pub struct SessionUtilisateur {
    pub user_id: i64,
    pub role: String,
}

impl SessionUtilisateur {
    pub fn est_admin(&self) -> bool {
        self.role == "admin"
    }
}

struct Entree {
    session: SessionUtilisateur,
    creee: Instant,
    derniere_activite: Instant,
}

#[derive(Default)]
pub struct AuthState {
    sessions: Mutex<HashMap<String, Entree>>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Acces {
    Connecte,
    Admin,
    Module(&'static str, &'static str),
}

pub const ERREUR_SESSION: &str = "Session invalide ou expirée : veuillez vous reconnecter";

impl AuthState {
    pub fn ouvrir(&self, user_id: i64, role: &str) -> Result<String, String> {
        let mut octets = [0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut octets);
        let token = hex::encode(octets);
        let maintenant = Instant::now();
        let mut sessions = self.sessions.lock().map_err(|e| e.to_string())?;
        sessions.retain(|_, e| !Self::expiree(e, maintenant));
        sessions.insert(token.clone(), Entree {
            session: SessionUtilisateur { user_id, role: role.to_string() },
            creee: maintenant,
            derniere_activite: maintenant,
        });
        Ok(token)
    }

    pub fn fermer(&self, token: &str) -> Result<(), String> {
        self.sessions.lock().map_err(|e| e.to_string())?.remove(token);
        Ok(())
    }

    pub fn fermer_utilisateur(&self, user_id: i64) -> Result<(), String> {
        self.sessions.lock().map_err(|e| e.to_string())?.retain(|_, e| e.session.user_id != user_id);
        Ok(())
    }

    pub fn session(&self, token: &str) -> Result<SessionUtilisateur, String> {
        let maintenant = Instant::now();
        let mut sessions = self.sessions.lock().map_err(|e| e.to_string())?;
        let expiree = match sessions.get(token) {
            None => return Err(ERREUR_SESSION.to_string()),
            Some(e) => Self::expiree(e, maintenant),
        };
        if expiree {
            sessions.remove(token);
            return Err(ERREUR_SESSION.to_string());
        }
        let entree = sessions.get_mut(token).ok_or(ERREUR_SESSION)?;
        entree.derniere_activite = maintenant;
        Ok(entree.session.clone())
    }

    fn expiree(e: &Entree, maintenant: Instant) -> bool {
        maintenant.duration_since(e.derniere_activite) > INACTIVITE_MAX || maintenant.duration_since(e.creee) > DUREE_MAX
    }

    #[cfg(test)]
    pub fn vieillir(&self, token: &str, duree: Duration) {
        let mut sessions = self.sessions.lock().unwrap();
        if let Some(e) = sessions.get_mut(token) {
            e.derniere_activite -= duree;
            e.creee -= duree;
        }
    }
}

pub fn autoriser(auth: &AuthState, conn: &Connection, token: &str, acces: Acces) -> Result<SessionUtilisateur, String> {
    let session = auth.session(token)?;
    let role_actuel: Option<String> = conn
        .query_row("SELECT role FROM utilisateurs WHERE id = ?1", params![session.user_id], |r| r.get(0))
        .optional()
        .map_err(|e| e.to_string())?;
    let Some(role) = role_actuel else {
        auth.fermer_utilisateur(session.user_id)?;
        return Err(ERREUR_SESSION.to_string());
    };
    let session = SessionUtilisateur { role, ..session };
    verifier_acces(conn, &session, acces)?;
    Ok(session)
}

pub fn verifier_acces(conn: &Connection, session: &SessionUtilisateur, acces: Acces) -> Result<(), String> {
    match acces {
        Acces::Connecte => Ok(()),
        _ if session.est_admin() => Ok(()),
        Acces::Admin => Err("Accès refusé : réservé à l'administrateur".to_string()),
        Acces::Module(module, action) => {
            let autorise: bool = conn
                .query_row(
                    "SELECT allowed FROM permissions WHERE role = ?1 AND module = ?2 AND action = ?3",
                    params![session.role, module, action],
                    |r| r.get(0),
                )
                .optional()
                .map_err(|e| e.to_string())?
                .unwrap_or(false);
            if autorise {
                Ok(())
            } else {
                Err(format!("Accès refusé : {} / {}", module, action))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db() -> Connection {
        let conn = crate::db::init_db(":memory:").unwrap();
        conn.execute("INSERT INTO utilisateurs (id, login, password_hash, nom, role) VALUES (2, 'karim', 'x', 'Karim', 'caissier')", []).unwrap();
        conn.execute("INSERT INTO utilisateurs (id, login, password_hash, nom, role) VALUES (3, 'sara', 'x', 'Sara', 'manager')", []).unwrap();
        conn
    }

    #[test]
    fn test_token_inconnu_refuse() {
        let auth = AuthState::default();
        assert_eq!(autoriser(&auth, &db(), "faux", Acces::Connecte).unwrap_err(), ERREUR_SESSION);
        assert_eq!(autoriser(&auth, &db(), "", Acces::Connecte).unwrap_err(), ERREUR_SESSION);
    }

    #[test]
    fn test_tokens_uniques_et_longs() {
        let auth = AuthState::default();
        let a = auth.ouvrir(1, "admin").unwrap();
        let b = auth.ouvrir(1, "admin").unwrap();
        assert_ne!(a, b);
        assert_eq!(a.len(), 64);
    }

    #[test]
    fn test_permissions_par_role() {
        let auth = AuthState::default();
        let conn = db();
        let caissier = auth.ouvrir(2, "caissier").unwrap();
        let manager = auth.ouvrir(3, "manager").unwrap();
        let admin = auth.ouvrir(1, "admin").unwrap();

        assert!(autoriser(&auth, &conn, &caissier, Acces::Module("ventes", "creer")).is_ok());
        assert!(autoriser(&auth, &conn, &caissier, Acces::Module("articles", "modifier")).is_err());
        assert!(autoriser(&auth, &conn, &caissier, Acces::Admin).is_err());
        assert!(autoriser(&auth, &conn, &manager, Acces::Module("articles", "modifier")).is_ok());
        assert!(autoriser(&auth, &conn, &manager, Acces::Module("settings", "modifier")).is_err());
        assert!(autoriser(&auth, &conn, &manager, Acces::Admin).is_err());
        assert!(autoriser(&auth, &conn, &admin, Acces::Admin).is_ok());
        assert!(autoriser(&auth, &conn, &admin, Acces::Module("inexistant", "voir")).is_ok());
    }

    #[test]
    fn test_role_relu_en_base() {
        let auth = AuthState::default();
        let conn = db();
        let token = auth.ouvrir(3, "admin").unwrap();
        assert!(autoriser(&auth, &conn, &token, Acces::Admin).is_err());
        conn.execute("UPDATE utilisateurs SET role = 'admin' WHERE id = 3", []).unwrap();
        assert!(autoriser(&auth, &conn, &token, Acces::Admin).is_ok());
    }

    #[test]
    fn test_utilisateur_supprime_deconnecte() {
        let auth = AuthState::default();
        let conn = db();
        let token = auth.ouvrir(2, "caissier").unwrap();
        conn.execute("DELETE FROM utilisateurs WHERE id = 2", []).unwrap();
        assert!(autoriser(&auth, &conn, &token, Acces::Connecte).is_err());
        assert!(auth.session(&token).is_err());
    }

    #[test]
    fn test_expiration() {
        let auth = AuthState::default();
        let conn = db();
        let token = auth.ouvrir(2, "caissier").unwrap();
        auth.vieillir(&token, INACTIVITE_MAX - Duration::from_secs(5));
        assert!(autoriser(&auth, &conn, &token, Acces::Connecte).is_ok());
        auth.vieillir(&token, INACTIVITE_MAX + Duration::from_secs(1));
        assert!(autoriser(&auth, &conn, &token, Acces::Connecte).is_err());
    }

    #[test]
    fn test_deconnexion() {
        let auth = AuthState::default();
        let conn = db();
        let token = auth.ouvrir(2, "caissier").unwrap();
        auth.fermer(&token).unwrap();
        assert!(autoriser(&auth, &conn, &token, Acces::Connecte).is_err());
    }

    #[test]
    fn test_toutes_les_commandes_verifient_la_session() {
        let publiques = ["login", "login_pin", "logout"];
        let dossier = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/commands");
        let mut commandes = 0;
        let mut fautives = Vec::new();
        for entree in std::fs::read_dir(dossier).unwrap() {
            let source = std::fs::read_to_string(entree.unwrap().path()).unwrap();
            for bloc in source.split("#[tauri::command]").skip(1) {
                let nom = bloc.trim_start().trim_start_matches("pub fn ").split('(').next().unwrap().to_string();
                commandes += 1;
                if publiques.contains(&nom.as_str()) {
                    continue;
                }
                let signature = bloc.split('{').next().unwrap();
                let corps = bloc.split_once('{').map(|x| x.1).unwrap_or("");
                let debut: String = corps.lines().take(4).collect::<Vec<_>>().join("\n");
                let controle = debut.contains("autoriser(&auth, &conn, &token,") || debut.contains("auth.session(&token)?");
                if !signature.contains("token: String") || !controle {
                    fautives.push(nom);
                }
            }
        }
        assert!(commandes >= 98, "{} commandes trouvées", commandes);
        assert!(fautives.is_empty(), "commandes sans contrôle de session : {:?}", fautives);
    }
}
