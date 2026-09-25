use argon2::password_hash::{rand_core::OsRng, SaltString};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use rusqlite::{params, Connection, OpenFlags, OptionalExtension, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, TryLockError};
use std::time::Duration;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Category {
    pub id: Option<i64>,
    pub nom: String,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Fournisseur {
    pub id: Option<i64>,
    pub nom: String,
    pub adresse: Option<String>,
    pub telephone: Option<String>,
    pub ice: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Client {
    pub id: Option<i64>,
    pub code: Option<String>,
    pub nom: String,
    pub adresse: Option<String>,
    pub telephone: Option<String>,
    pub email: Option<String>,
    pub ice: Option<String>,
    pub credit_plafond: Option<f64>,
    pub credit_actuel: Option<f64>,
    pub segment: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Utilisateur {
    pub id: Option<i64>,
    pub login: String,
    pub nom: String,
    pub role: String,
    #[serde(default)]
    pub must_change_password: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Settings {
    pub shop_name: String,
    pub shop_address: Option<String>,
    pub shop_phone: Option<String>,
    pub shop_email: Option<String>,
    pub ice: Option<String>,
    pub if_number: Option<String>,
    pub rc_number: Option<String>,
    pub patente: Option<String>,
    pub default_tva: f64,
    pub receipt_footer: Option<String>,
    pub currency: String,
    pub printer_name: Option<String>,
    pub fidelite_actif: Option<String>,
    pub autoriser_stock_negatif: Option<String>,
    pub remise_max_caissier: Option<String>,
    pub remise_max_manager: Option<String>,
    pub fidelite_dh_pour_1_point: Option<String>,
    pub fidelite_valeur_1_point: Option<String>,
    pub business_type: Option<String>,
    pub idle_timeout: Option<String>,
    pub logo_base64: Option<String>,
    pub receipt_header: Option<String>,
    pub doc_primary_color: Option<String>,
}

pub struct DbState {
    pub conn: Arc<Mutex<Connection>>,
    lecteurs: Vec<Mutex<Connection>>,
    prochain_lecteur: AtomicUsize,
}

pub const LECTEURS: usize = 3;

impl DbState {
    pub fn new(conn: Connection) -> Self {
        DbState {
            conn: Arc::new(Mutex::new(conn)),
            lecteurs: Vec::new(),
            prochain_lecteur: AtomicUsize::new(0),
        }
    }

    pub fn avec_lecteurs(
        conn: Connection,
        chemin: &Path,
        nombre: usize,
    ) -> std::result::Result<Self, String> {
        let mut etat = DbState::new(conn);
        for _ in 0..nombre {
            etat.lecteurs.push(Mutex::new(ouvrir_lecteur(chemin)?));
        }
        Ok(etat)
    }

    pub fn lecture(&self) -> std::result::Result<MutexGuard<'_, Connection>, String> {
        if self.lecteurs.is_empty() {
            return self.conn.lock().map_err(|e| e.to_string());
        }
        for lecteur in &self.lecteurs {
            match lecteur.try_lock() {
                Ok(garde) => return Ok(garde),
                Err(TryLockError::WouldBlock) => continue,
                Err(TryLockError::Poisoned(e)) => return Err(e.to_string()),
            }
        }
        let index = self.prochain_lecteur.fetch_add(1, Ordering::Relaxed) % self.lecteurs.len();
        self.lecteurs[index].lock().map_err(|e| e.to_string())
    }
}

pub(crate) fn ouvrir_lecteur(chemin: &Path) -> std::result::Result<Connection, String> {
    let conn = Connection::open_with_flags(
        chemin,
        OpenFlags::SQLITE_OPEN_READ_ONLY
            | OpenFlags::SQLITE_OPEN_NO_MUTEX
            | OpenFlags::SQLITE_OPEN_URI,
    )
    .map_err(|e| format!("Connexion en lecture impossible : {}", e))?;
    conn.busy_timeout(Duration::from_secs(5))
        .map_err(|e| e.to_string())?;
    conn.execute_batch("PRAGMA query_only = ON;")
        .map_err(|e| e.to_string())?;
    Ok(conn)
}

pub const SCHEMA_VERSION: i64 = MIGRATIONS.len() as i64;

type Migration = fn(&Connection) -> Result<()>;

const MIGRATIONS: &[Migration] = &[
    migration_001_base,
    migration_002_montants_au_centime,
    migration_003_contraintes,
    migration_004_unicite,
    migration_005_verrouillage_pin,
    migration_006_plafonds_remise,
];

pub fn init_db(db_path: &str) -> std::result::Result<Connection, String> {
    let mut conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
        .map_err(|e| e.to_string())?;
    conn.busy_timeout(Duration::from_secs(5))
        .map_err(|e| e.to_string())?;
    migrer(&mut conn)?;
    let version = version_schema(&conn).map_err(|e| e.to_string())?;
    if version != SCHEMA_VERSION {
        return Err(format!(
            "Schéma de base inattendu après migration : v{} au lieu de v{}",
            version, SCHEMA_VERSION
        ));
    }
    Ok(conn)
}

pub fn version_schema(conn: &Connection) -> Result<i64> {
    conn.pragma_query_value(None, "user_version", |r| r.get(0))
}

pub fn migrer(conn: &mut Connection) -> std::result::Result<(), String> {
    appliquer_migrations(conn, MIGRATIONS)
}

fn appliquer_migrations(
    conn: &mut Connection,
    migrations: &[Migration],
) -> std::result::Result<(), String> {
    let version_max = migrations.len() as i64;
    let actuelle = version_schema(conn).map_err(|e| e.to_string())?;
    if actuelle > version_max {
        return Err(format!(
            "Base de données au schéma v{} créée par une version plus récente de SuperCaisse (cette version gère jusqu'à v{}) : mettez l'application à jour",
            actuelle, version_max
        ));
    }
    for (index, migration) in migrations.iter().enumerate() {
        let cible = index as i64 + 1;
        if actuelle >= cible {
            continue;
        }
        if cible == 1 {
            migration(conn).map_err(|e| format!("Migration v1 : {}", e))?;
            conn.pragma_update(None, "user_version", cible)
                .map_err(|e| e.to_string())?;
        } else {
            let cles_actives: bool = conn
                .pragma_query_value(None, "foreign_keys", |r| r.get(0))
                .map_err(|e| e.to_string())?;
            conn.pragma_update(None, "foreign_keys", false)
                .map_err(|e| e.to_string())?;
            let resultat = executer_migration(conn, *migration, cible);
            conn.pragma_update(None, "foreign_keys", cles_actives)
                .map_err(|e| e.to_string())?;
            resultat?;
        }
        log::info!("Schéma de base migré en v{}", cible);
    }
    Ok(())
}

fn executer_migration(
    conn: &mut Connection,
    migration: Migration,
    cible: i64,
) -> std::result::Result<(), String> {
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    migration(&tx).map_err(|e| format!("Migration v{} : {}", cible, e))?;
    tx.pragma_update(None, "user_version", cible)
        .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())
}

fn erreur_migration(message: String) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(message.into())
}

fn colonne_existe(conn: &Connection, table: &str, colonne: &str) -> Result<bool> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({})", table))?;
    let noms = stmt.query_map([], |r| r.get::<_, String>(1))?;
    for nom in noms {
        if nom? == colonne {
            return Ok(true);
        }
    }
    Ok(false)
}

fn ajouter_colonne(conn: &Connection, table: &str, colonne: &str, definition: &str) -> Result<()> {
    if !colonne_existe(conn, table, colonne)? {
        conn.execute(
            &format!(
                "ALTER TABLE {} ADD COLUMN {} {}",
                table, colonne, definition
            ),
            [],
        )?;
    }
    Ok(())
}

pub(crate) const COLONNES_MONTANTS: &[(&str, &[&str])] = &[
    ("clients", &["credit_plafond", "credit_actuel"]),
    (
        "ventes",
        &[
            "montant_total",
            "montant_remise",
            "montant_ht",
            "montant_tva",
        ],
    ),
    (
        "vente_articles",
        &["total_ligne", "montant_ht", "montant_tva"],
    ),
    ("vente_paiements", &["montant"]),
    ("achats", &["montant_total"]),
    ("achat_articles", &["total_ligne"]),
    ("paiements", &["montant"]),
    ("journal_caisse", &["montant"]),
    ("cheques", &["montant"]),
    (
        "sessions_caisse",
        &[
            "fond_initial",
            "total_especes_attendu",
            "total_especes_declare",
            "ecart",
        ],
    ),
    (
        "caisses",
        &[
            "fond_initial",
            "recettes_especes",
            "recettes_cb",
            "recettes_cheque",
            "recettes_virement",
            "depenses",
            "ecart",
        ],
    ),
];

fn migration_002_montants_au_centime(conn: &Connection) -> Result<()> {
    let mut corriges = 0;
    for (table, colonnes) in COLONNES_MONTANTS {
        for colonne in *colonnes {
            let arrondi = format!(
                "ROUND({c} * 100 + (CASE WHEN {c} > 0 THEN 1e-6 WHEN {c} < 0 THEN -1e-6 ELSE 0 END)) / 100.0",
                c = colonne
            );
            corriges += conn.execute(
                &format!(
                    "UPDATE {t} SET {c} = {a} WHERE {c} IS NOT NULL AND typeof({c}) IN ('real', 'integer') AND {c} != {a}",
                    t = table,
                    c = colonne,
                    a = arrondi
                ),
                [],
            )?;
        }
    }
    if corriges > 0 {
        log::info!(
            "Montants arrondis au centime : {} valeurs corrigées",
            corriges
        );
    }
    Ok(())
}

pub(crate) const MODES_PAIEMENT_VENTE: &[&str] = &[
    "especes", "carte", "cb", "cheque", "virement", "credit", "fidelite", "mixte",
];
pub(crate) const TYPES_DOCUMENT: &[&str] = &["facture", "bl", "devis", "commande", "avoir"];
pub(crate) const STATUTS_VENTE: &[&str] = &["validee", "annulee", "convertie"];
pub(crate) const TYPES_JOURNAL: &[&str] = &["entree", "sortie", "encaissement"];
pub(crate) const TYPES_MOUVEMENT: &[&str] = &["entree", "sortie", "inventaire"];
pub(crate) const STATUTS_SESSION: &[&str] = &["ouverte", "cloturee"];
pub(crate) const ROLES: &[&str] = &["admin", "manager", "caissier"];
pub(crate) const TYPES_CHEQUE: &[&str] = &["client", "fournisseur"];
pub(crate) const STATUTS_CHEQUE: &[&str] = &["en_attente", "encaisse", "impaye"];

fn liste_sql(valeurs: &[&str]) -> String {
    valeurs
        .iter()
        .map(|v| format!("'{}'", v))
        .collect::<Vec<_>>()
        .join(",")
}

struct Reconstruction {
    table: &'static str,
    definition: String,
    colonnes: &'static str,
    selection: &'static str,
}

fn reconstructions() -> Vec<Reconstruction> {
    vec![
        Reconstruction {
            table: "utilisateurs",
            definition: format!(
                "id INTEGER PRIMARY KEY AUTOINCREMENT,
                login TEXT NOT NULL UNIQUE,
                password_hash TEXT NOT NULL,
                nom TEXT NOT NULL,
                role TEXT NOT NULL DEFAULT 'caissier' CHECK (role IN ({})),
                pin_hash TEXT,
                must_change_password INTEGER NOT NULL DEFAULT 0",
                liste_sql(ROLES)
            ),
            colonnes: "id, login, password_hash, nom, role, pin_hash, must_change_password",
            selection: "id, login, password_hash, nom, COALESCE(role, 'caissier'), pin_hash, COALESCE(must_change_password, 0)",
        },
        Reconstruction {
            table: "sessions_caisse",
            definition: format!(
                "id INTEGER PRIMARY KEY AUTOINCREMENT,
                caissier_id INTEGER NOT NULL REFERENCES utilisateurs(id),
                date_ouverture TEXT NOT NULL DEFAULT (datetime('now','localtime')),
                date_cloture TEXT,
                fond_initial REAL NOT NULL DEFAULT 0 CHECK (fond_initial >= 0),
                total_especes_attendu REAL DEFAULT 0,
                total_especes_declare REAL DEFAULT 0,
                ecart REAL DEFAULT 0,
                statut TEXT NOT NULL DEFAULT 'ouverte' CHECK (statut IN ({})),
                magasin_id INTEGER REFERENCES magasins(id)",
                liste_sql(STATUTS_SESSION)
            ),
            colonnes: "id, caissier_id, date_ouverture, date_cloture, fond_initial, total_especes_attendu, total_especes_declare, ecart, statut, magasin_id",
            selection: "id, caissier_id, date_ouverture, date_cloture, COALESCE(fond_initial, 0), total_especes_attendu, total_especes_declare, ecart, COALESCE(statut, 'ouverte'), magasin_id",
        },
        Reconstruction {
            table: "ventes",
            definition: format!(
                "id INTEGER PRIMARY KEY AUTOINCREMENT,
                date TEXT NOT NULL DEFAULT (datetime('now','localtime')),
                client_id INTEGER REFERENCES clients(id),
                caissier_id INTEGER REFERENCES utilisateurs(id),
                montant_total REAL NOT NULL DEFAULT 0,
                montant_remise REAL NOT NULL DEFAULT 0,
                mode_paiement TEXT NOT NULL DEFAULT 'especes' CHECK (mode_paiement IN ({})),
                statut TEXT NOT NULL DEFAULT 'validee' CHECK (statut IN ({})),
                points_utilises REAL NOT NULL DEFAULT 0 CHECK (points_utilises >= 0),
                points_gagnes REAL NOT NULL DEFAULT 0,
                numero_facture TEXT,
                dtype TEXT NOT NULL DEFAULT 'facture' CHECK (dtype IN ({})),
                session_id INTEGER REFERENCES sessions_caisse(id),
                source_vente_id INTEGER REFERENCES ventes(id),
                magasin_id INTEGER REFERENCES magasins(id),
                montant_ht REAL,
                montant_tva REAL,
                CHECK (dtype = 'avoir' OR montant_total >= 0)",
                liste_sql(MODES_PAIEMENT_VENTE),
                liste_sql(STATUTS_VENTE),
                liste_sql(TYPES_DOCUMENT)
            ),
            colonnes: "id, date, client_id, caissier_id, montant_total, montant_remise, mode_paiement, statut, points_utilises, points_gagnes, numero_facture, dtype, session_id, source_vente_id, magasin_id, montant_ht, montant_tva",
            selection: "id, date, client_id, caissier_id, COALESCE(montant_total, 0), COALESCE(montant_remise, 0), COALESCE(mode_paiement, 'especes'), COALESCE(statut, 'validee'), COALESCE(points_utilises, 0), COALESCE(points_gagnes, 0), numero_facture, COALESCE(dtype, 'facture'), session_id, source_vente_id, magasin_id, montant_ht, montant_tva",
        },
        Reconstruction {
            table: "vente_articles",
            definition: "id INTEGER PRIMARY KEY AUTOINCREMENT,
                vente_id INTEGER NOT NULL REFERENCES ventes(id),
                article_id INTEGER NOT NULL REFERENCES articles(id),
                quantite REAL NOT NULL CHECK (quantite > 0),
                prix_unitaire REAL NOT NULL,
                tva REAL NOT NULL DEFAULT 0 CHECK (tva >= 0 AND tva <= 100),
                total_ligne REAL NOT NULL,
                remise_ligne REAL NOT NULL DEFAULT 0 CHECK (remise_ligne >= 0 AND remise_ligne <= 100),
                note TEXT,
                variante_id INTEGER REFERENCES article_variantes(id),
                prix_type TEXT NOT NULL DEFAULT 'public' CHECK (prix_type IN ('public','grossiste')),
                montant_ht REAL,
                montant_tva REAL"
                .to_string(),
            colonnes: "id, vente_id, article_id, quantite, prix_unitaire, tva, total_ligne, remise_ligne, note, variante_id, prix_type, montant_ht, montant_tva",
            selection: "id, vente_id, article_id, quantite, prix_unitaire, COALESCE(tva, 0), total_ligne, COALESCE(remise_ligne, 0), note, variante_id, COALESCE(prix_type, 'public'), montant_ht, montant_tva",
        },
        Reconstruction {
            table: "journal_caisse",
            definition: format!(
                "id INTEGER PRIMARY KEY AUTOINCREMENT,
                date TEXT NOT NULL DEFAULT (datetime('now','localtime')),
                utilisateur_id INTEGER REFERENCES utilisateurs(id),
                jtype TEXT NOT NULL CHECK (jtype IN ({})),
                montant REAL NOT NULL,
                description TEXT,
                session_id INTEGER REFERENCES sessions_caisse(id)",
                liste_sql(TYPES_JOURNAL)
            ),
            colonnes: "id, date, utilisateur_id, jtype, montant, description, session_id",
            selection: "id, date, utilisateur_id, jtype, montant, description, session_id",
        },
        Reconstruction {
            table: "mouvements_stock",
            definition: format!(
                "id INTEGER PRIMARY KEY AUTOINCREMENT,
                date TEXT NOT NULL DEFAULT (datetime('now','localtime')),
                article_id INTEGER NOT NULL REFERENCES articles(id),
                quantite REAL NOT NULL,
                mtype TEXT NOT NULL CHECK (mtype IN ({})),
                reference_id INTEGER,
                reference_type TEXT,
                magasin_id INTEGER REFERENCES magasins(id)",
                liste_sql(TYPES_MOUVEMENT)
            ),
            colonnes: "id, date, article_id, quantite, mtype, reference_id, reference_type, magasin_id",
            selection: "id, date, article_id, quantite, mtype, reference_id, reference_type, magasin_id",
        },
        Reconstruction {
            table: "cheques",
            definition: format!(
                "id INTEGER PRIMARY KEY AUTOINCREMENT,
                numero TEXT NOT NULL,
                banque TEXT NOT NULL,
                tireur TEXT,
                montant REAL NOT NULL CHECK (montant >= 0),
                date_emission TEXT NOT NULL,
                date_echeance TEXT NOT NULL,
                statut TEXT NOT NULL DEFAULT 'en_attente' CHECK (statut IN ({})),
                ctype TEXT NOT NULL CHECK (ctype IN ({})),
                client_id INTEGER REFERENCES clients(id),
                fournisseur_id INTEGER REFERENCES fournisseurs(id)",
                liste_sql(STATUTS_CHEQUE),
                liste_sql(TYPES_CHEQUE)
            ),
            colonnes: "id, numero, banque, tireur, montant, date_emission, date_echeance, statut, ctype, client_id, fournisseur_id",
            selection: "id, numero, banque, tireur, montant, date_emission, date_echeance, COALESCE(statut, 'en_attente'), ctype, client_id, fournisseur_id",
        },
    ]
}

const NORMALISATIONS: &[&str] = &[
    "UPDATE utilisateurs SET role = lower(trim(role)) WHERE role != lower(trim(role))",
    "UPDATE sessions_caisse SET statut = lower(trim(statut)) WHERE statut != lower(trim(statut))",
    "UPDATE sessions_caisse SET statut = 'cloturee' WHERE statut IN ('fermee', 'cloture', 'clôturée', 'clôturé')",
    "UPDATE ventes SET mode_paiement = lower(trim(mode_paiement)) WHERE mode_paiement != lower(trim(mode_paiement))",
    "UPDATE ventes SET mode_paiement = 'mixte' WHERE instr(mode_paiement, '+') > 0 OR mode_paiement IN ('split', 'multiple')",
    "UPDATE ventes SET mode_paiement = 'especes' WHERE mode_paiement IN ('espece', 'espèces', 'cash', '')",
    "UPDATE ventes SET statut = lower(trim(statut)) WHERE statut != lower(trim(statut))",
    "UPDATE ventes SET statut = 'validee' WHERE statut IN ('valide', 'validée', '')",
    "UPDATE ventes SET statut = 'annulee' WHERE statut = 'annulée'",
    "UPDATE ventes SET dtype = lower(trim(dtype)) WHERE dtype != lower(trim(dtype))",
    "UPDATE vente_articles SET prix_type = 'public' WHERE prix_type IS NULL OR prix_type NOT IN ('public', 'grossiste')",
    "UPDATE journal_caisse SET jtype = lower(trim(jtype)) WHERE jtype != lower(trim(jtype))",
    "UPDATE mouvements_stock SET mtype = lower(trim(mtype)) WHERE mtype != lower(trim(mtype))",
    "UPDATE mouvements_stock SET mtype = 'sortie' WHERE mtype = 'vente'",
    "UPDATE mouvements_stock SET mtype = 'entree' WHERE mtype = 'achat'",
    "UPDATE cheques SET statut = lower(trim(statut)) WHERE statut != lower(trim(statut))",
    "UPDATE cheques SET statut = 'encaisse' WHERE statut = 'encaissé'",
    "UPDATE cheques SET statut = 'impaye' WHERE statut = 'impayé'",
    "UPDATE cheques SET ctype = lower(trim(ctype)) WHERE ctype != lower(trim(ctype))",
];

const REFERENCES_ORPHELINES: &[(&str, &str, &str)] = &[
    ("ventes", "session_id", "sessions_caisse"),
    ("ventes", "source_vente_id", "ventes"),
    ("ventes", "magasin_id", "magasins"),
    ("ventes", "client_id", "clients"),
    ("ventes", "caissier_id", "utilisateurs"),
    ("vente_articles", "variante_id", "article_variantes"),
    ("journal_caisse", "session_id", "sessions_caisse"),
    ("journal_caisse", "utilisateur_id", "utilisateurs"),
    ("sessions_caisse", "magasin_id", "magasins"),
    ("mouvements_stock", "magasin_id", "magasins"),
    ("cheques", "client_id", "clients"),
    ("cheques", "fournisseur_id", "fournisseurs"),
];

fn verifier_enumeration(
    conn: &Connection,
    table: &str,
    colonne: &str,
    valeurs: &[&str],
) -> Result<()> {
    let sql = format!(
        "SELECT DISTINCT COALESCE({c}, 'NULL') FROM {t} WHERE {c} IS NULL OR {c} NOT IN ({v})",
        t = table,
        c = colonne,
        v = liste_sql(valeurs)
    );
    let mut stmt = conn.prepare(&sql)?;
    let invalides = stmt
        .query_map([], |r| r.get::<_, String>(0))?
        .collect::<Result<Vec<_>>>()?;
    if invalides.is_empty() {
        Ok(())
    } else {
        Err(erreur_migration(format!(
            "valeurs inconnues dans {}.{} : {} (attendu : {}). Corrigez ces lignes puis relancez l'application",
            table,
            colonne,
            invalides.join(", "),
            valeurs.join(", ")
        )))
    }
}

fn migration_003_contraintes(conn: &Connection) -> Result<()> {
    for sql in NORMALISATIONS {
        conn.execute(sql, [])?;
    }
    let mut orphelines = 0;
    for (table, colonne, parent) in REFERENCES_ORPHELINES {
        orphelines += conn.execute(
            &format!(
                "UPDATE {t} SET {c} = NULL WHERE {c} IS NOT NULL AND {c} NOT IN (SELECT id FROM {p})",
                t = table,
                c = colonne,
                p = parent
            ),
            [],
        )?;
    }
    if orphelines > 0 {
        log::warn!(
            "Références orphelines remises à NULL avant ajout des clés étrangères : {}",
            orphelines
        );
    }
    for (table, colonne, valeurs) in [
        ("utilisateurs", "role", ROLES),
        ("sessions_caisse", "statut", STATUTS_SESSION),
        ("ventes", "mode_paiement", MODES_PAIEMENT_VENTE),
        ("ventes", "statut", STATUTS_VENTE),
        ("ventes", "dtype", TYPES_DOCUMENT),
        ("journal_caisse", "jtype", TYPES_JOURNAL),
        ("mouvements_stock", "mtype", TYPES_MOUVEMENT),
        ("cheques", "statut", STATUTS_CHEQUE),
        ("cheques", "ctype", TYPES_CHEQUE),
    ] {
        verifier_enumeration(conn, table, colonne, valeurs)?;
    }
    for reconstruction in reconstructions() {
        reconstruire_table(conn, &reconstruction)?;
    }
    let mut stmt = conn.prepare("PRAGMA foreign_key_check")?;
    let violations = stmt
        .query_map([], |r| {
            Ok(format!(
                "{} #{} → {}",
                r.get::<_, String>(0)?,
                r.get::<_, Option<i64>>(1)?.unwrap_or(0),
                r.get::<_, String>(2)?
            ))
        })?
        .collect::<Result<Vec<_>>>()?;
    if !violations.is_empty() {
        return Err(erreur_migration(format!(
            "références invalides : {}",
            violations
                .iter()
                .take(10)
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        )));
    }
    Ok(())
}

fn reconstruire_table(conn: &Connection, r: &Reconstruction) -> Result<()> {
    let annexes = {
        let mut stmt = conn.prepare(
            "SELECT sql FROM sqlite_master WHERE tbl_name = ?1 AND type IN ('index', 'trigger') AND sql IS NOT NULL",
        )?;
        let lignes = stmt
            .query_map([r.table], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>>>()?;
        lignes
    };
    let sequence: Option<i64> = conn
        .query_row(
            "SELECT seq FROM sqlite_sequence WHERE name = ?1",
            [r.table],
            |row| row.get(0),
        )
        .optional()?;
    let nouvelle = format!("{}_v3", r.table);
    conn.execute_batch(&format!(
        "CREATE TABLE {n} ({d});
         INSERT INTO {n} ({c}) SELECT {s} FROM {t};
         DROP TABLE {t};
         ALTER TABLE {n} RENAME TO {t};",
        n = nouvelle,
        d = r.definition,
        c = r.colonnes,
        s = r.selection,
        t = r.table
    ))?;
    for sql in annexes {
        conn.execute_batch(&sql)?;
    }
    if let Some(seq) = sequence {
        conn.execute(
            "UPDATE sqlite_sequence SET seq = MAX(seq, ?1) WHERE name = ?2",
            params![seq, r.table],
        )?;
    }
    Ok(())
}

fn migration_004_unicite(conn: &Connection) -> Result<()> {
    let numeros = conn.execute(
        "UPDATE ventes SET numero_facture = numero_facture || '-DOUBLON-' || id
         WHERE numero_facture IS NOT NULL
           AND id NOT IN (SELECT MIN(id) FROM ventes WHERE numero_facture IS NOT NULL GROUP BY numero_facture)",
        [],
    )?;
    conn.execute(
        "UPDATE clients SET code = NULLIF(trim(code), '') WHERE code IS NOT NULL AND code != trim(code) OR code = ''",
        [],
    )?;
    let codes = conn.execute(
        "UPDATE clients SET code = code || '-' || id
         WHERE code IS NOT NULL
           AND id NOT IN (SELECT MIN(id) FROM clients WHERE code IS NOT NULL GROUP BY code)",
        [],
    )?;
    if numeros + codes > 0 {
        let detail = format!(
            "Doublons renommés avant ajout des contraintes d'unicité : {} numéro(s) de document (suffixe -DOUBLON-<id>), {} code(s) client (suffixe -<id>)",
            numeros, codes
        );
        log::warn!("{}", detail);
        conn.execute(
            "INSERT INTO audit_log (utilisateur_id, action, detail) VALUES (NULL, 'correction_doublons', ?1)",
            params![detail],
        )?;
    }
    conn.execute_batch(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_ventes_numero_facture_unique ON ventes(numero_facture) WHERE numero_facture IS NOT NULL;
         CREATE UNIQUE INDEX IF NOT EXISTS idx_clients_code_unique ON clients(code) WHERE code IS NOT NULL;",
    )?;
    crate::commands::resynchroniser_numerotation(conn)
}

fn migration_005_verrouillage_pin(conn: &Connection) -> Result<()> {
    ajouter_colonne(
        conn,
        "utilisateurs",
        "pin_echecs",
        "INTEGER NOT NULL DEFAULT 0",
    )?;
    ajouter_colonne(conn, "utilisateurs", "pin_bloque_jusqua", "TEXT")?;
    Ok(())
}

fn migration_006_plafonds_remise(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "INSERT OR IGNORE INTO settings (key, value) VALUES ('remise_max_caissier', '10');
         INSERT OR IGNORE INTO settings (key, value) VALUES ('remise_max_manager', '100');",
    )
}

fn migration_001_base(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS categories (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            nom TEXT NOT NULL UNIQUE,
            description TEXT
        );

        CREATE TABLE IF NOT EXISTS fournisseurs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            nom TEXT NOT NULL,
            adresse TEXT,
            telephone TEXT,
            ice TEXT,
            email TEXT
        );

        CREATE TABLE IF NOT EXISTS clients (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            code TEXT,
            nom TEXT NOT NULL,
            adresse TEXT,
            telephone TEXT,
            email TEXT,
            ice TEXT,
            credit_plafond REAL DEFAULT 0,
            credit_actuel REAL DEFAULT 0,
            points_fidelite REAL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS articles (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            code_barre TEXT,
            designation TEXT NOT NULL,
            description TEXT,
            image_url TEXT,
            prix_achat REAL DEFAULT 0,
            prix_vente REAL DEFAULT 0,
            tva REAL DEFAULT 0,
            stock REAL DEFAULT 0,
            stock_alerte REAL DEFAULT 0,
            categorie_id INTEGER,
            fournisseur_id INTEGER,
            actif INTEGER DEFAULT 1,
            FOREIGN KEY (categorie_id) REFERENCES categories(id),
            FOREIGN KEY (fournisseur_id) REFERENCES fournisseurs(id)
        );

        CREATE TABLE IF NOT EXISTS utilisateurs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            login TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL,
            nom TEXT NOT NULL,
            role TEXT DEFAULT 'caissier'
        );

        CREATE TABLE IF NOT EXISTS ventes (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            date TEXT NOT NULL DEFAULT (datetime('now','localtime')),
            client_id INTEGER,
            caissier_id INTEGER,
            montant_total REAL DEFAULT 0,
            montant_remise REAL DEFAULT 0,
            mode_paiement TEXT DEFAULT 'especes',
            statut TEXT DEFAULT 'validee',
            points_utilises REAL DEFAULT 0,
            points_gagnes REAL DEFAULT 0,
            FOREIGN KEY (client_id) REFERENCES clients(id),
            FOREIGN KEY (caissier_id) REFERENCES utilisateurs(id)
        );

        CREATE TABLE IF NOT EXISTS vente_articles (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            vente_id INTEGER NOT NULL,
            article_id INTEGER NOT NULL,
            quantite REAL NOT NULL,
            prix_unitaire REAL NOT NULL,
            tva REAL DEFAULT 0,
            total_ligne REAL NOT NULL,
            FOREIGN KEY (vente_id) REFERENCES ventes(id),
            FOREIGN KEY (article_id) REFERENCES articles(id)
        );

        CREATE TABLE IF NOT EXISTS achats (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            date TEXT NOT NULL DEFAULT (datetime('now','localtime')),
            fournisseur_id INTEGER,
            reference TEXT,
            montant_total REAL DEFAULT 0,
            statut TEXT DEFAULT 'recu',
            FOREIGN KEY (fournisseur_id) REFERENCES fournisseurs(id)
        );

        CREATE TABLE IF NOT EXISTS achat_articles (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            achat_id INTEGER NOT NULL,
            article_id INTEGER NOT NULL,
            quantite REAL NOT NULL,
            prix_unitaire REAL NOT NULL,
            total_ligne REAL NOT NULL,
            FOREIGN KEY (achat_id) REFERENCES achats(id),
            FOREIGN KEY (article_id) REFERENCES articles(id)
        );

        CREATE TABLE IF NOT EXISTS paiements (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            client_id INTEGER NOT NULL,
            date TEXT NOT NULL DEFAULT (datetime('now','localtime')),
            montant REAL NOT NULL,
            type TEXT NOT NULL,
            reference TEXT,
            FOREIGN KEY (client_id) REFERENCES clients(id)
        );

        CREATE TABLE IF NOT EXISTS mouvements_stock (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            date TEXT NOT NULL DEFAULT (datetime('now','localtime')),
            article_id INTEGER NOT NULL,
            quantite REAL NOT NULL,
            mtype TEXT NOT NULL,
            reference_id INTEGER,
            reference_type TEXT,
            FOREIGN KEY (article_id) REFERENCES articles(id)
        );

        CREATE TABLE IF NOT EXISTS journal_caisse (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            date TEXT NOT NULL DEFAULT (datetime('now','localtime')),
            utilisateur_id INTEGER,
            jtype TEXT NOT NULL,
            montant REAL NOT NULL,
            description TEXT,
            FOREIGN KEY (utilisateur_id) REFERENCES utilisateurs(id)
        );

        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS sessions_caisse (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            caissier_id INTEGER NOT NULL,
            date_ouverture TEXT NOT NULL DEFAULT (datetime('now','localtime')),
            date_cloture TEXT,
            fond_initial REAL NOT NULL DEFAULT 0,
            total_especes_attendu REAL DEFAULT 0,
            total_especes_declare REAL DEFAULT 0,
            ecart REAL DEFAULT 0,
            statut TEXT DEFAULT 'ouverte',
            FOREIGN KEY (caissier_id) REFERENCES utilisateurs(id)
        );

        CREATE TABLE IF NOT EXISTS cheques (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            numero TEXT NOT NULL,
            banque TEXT NOT NULL,
            tireur TEXT,
            montant REAL NOT NULL,
            date_emission TEXT NOT NULL,
            date_echeance TEXT NOT NULL,
            statut TEXT DEFAULT 'en_attente',
            ctype TEXT NOT NULL,
            client_id INTEGER,
            fournisseur_id INTEGER,
            FOREIGN KEY (client_id) REFERENCES clients(id),
            FOREIGN KEY (fournisseur_id) REFERENCES fournisseurs(id)
        );

        CREATE TABLE IF NOT EXISTS magasins (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            nom TEXT NOT NULL UNIQUE,
            adresse TEXT
        );

        CREATE TABLE IF NOT EXISTS article_stocks (
            article_id INTEGER NOT NULL,
            magasin_id INTEGER NOT NULL,
            quantite REAL DEFAULT 0,
            PRIMARY KEY (article_id, magasin_id),
            FOREIGN KEY (article_id) REFERENCES articles(id),
            FOREIGN KEY (magasin_id) REFERENCES magasins(id)
        );

        CREATE TABLE IF NOT EXISTS transferts_stock (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            source_id INTEGER NOT NULL,
            dest_id INTEGER NOT NULL,
            date TEXT NOT NULL DEFAULT (datetime('now','localtime')),
            statut TEXT DEFAULT 'en_attente',
            utilisateur_id INTEGER,
            FOREIGN KEY (source_id) REFERENCES magasins(id),
            FOREIGN KEY (dest_id) REFERENCES magasins(id),
            FOREIGN KEY (utilisateur_id) REFERENCES utilisateurs(id)
        );

        CREATE TABLE IF NOT EXISTS transfert_lignes (
            transfert_id INTEGER NOT NULL,
            article_id INTEGER NOT NULL,
            quantite REAL NOT NULL,
            PRIMARY KEY (transfert_id, article_id),
            FOREIGN KEY (transfert_id) REFERENCES transferts_stock(id),
            FOREIGN KEY (article_id) REFERENCES articles(id)
        );

        CREATE TABLE IF NOT EXISTS mouvements_fidelite (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            client_id INTEGER NOT NULL,
            vente_id INTEGER,
            points REAL NOT NULL,
            mtype TEXT NOT NULL, -- 'gain', 'depense'
            date TEXT NOT NULL DEFAULT (datetime('now','localtime')),
            FOREIGN KEY (client_id) REFERENCES clients(id),
            FOREIGN KEY (vente_id) REFERENCES ventes(id)
        );

        CREATE TABLE IF NOT EXISTS tables_resto (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            nom TEXT NOT NULL,
            statut TEXT DEFAULT 'libre',
            ticket_id TEXT
        );

        CREATE TABLE IF NOT EXISTS article_variantes (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            article_id INTEGER NOT NULL,
            taille TEXT,
            couleur TEXT,
            stock_dedie REAL DEFAULT 0,
            FOREIGN KEY (article_id) REFERENCES articles(id)
        );

        CREATE TABLE IF NOT EXISTS audit_log (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            date TEXT NOT NULL DEFAULT (datetime('now','localtime')),
            utilisateur_id INTEGER,
            action TEXT NOT NULL,
            detail TEXT,
            reference_type TEXT,
            reference_id INTEGER,
            FOREIGN KEY (utilisateur_id) REFERENCES utilisateurs(id)
        );

        CREATE TABLE IF NOT EXISTS inventaires (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            date_debut TEXT NOT NULL DEFAULT (datetime('now','localtime')),
            date_fin TEXT,
            statut TEXT DEFAULT 'en_cours',
            magasin_id INTEGER NOT NULL,
            utilisateur_id INTEGER,
            FOREIGN KEY (magasin_id) REFERENCES magasins(id),
            FOREIGN KEY (utilisateur_id) REFERENCES utilisateurs(id)
        );

        CREATE TABLE IF NOT EXISTS inventaire_lignes (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            inventaire_id INTEGER NOT NULL,
            article_id INTEGER NOT NULL,
            stock_theorique REAL NOT NULL DEFAULT 0,
            stock_compte REAL,
            ecart REAL,
            FOREIGN KEY (inventaire_id) REFERENCES inventaires(id),
            FOREIGN KEY (article_id) REFERENCES articles(id)
        );
    ",
    )?;

    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS permissions (
            role TEXT NOT NULL,
            module TEXT NOT NULL,
            action TEXT NOT NULL,
            allowed INTEGER NOT NULL DEFAULT 1,
            PRIMARY KEY (role, module, action)
        );
    ",
    )?;

    {
        let modules = vec![
            "articles",
            "categories",
            "clients",
            "fournisseurs",
            "ventes",
            "achats",
            "stock",
            "inventaire",
            "journal",
            "cheques",
            "rapports",
            "magasins",
            "audit",
            "settings",
            "reappro",
        ];
        let actions = vec!["voir", "creer", "modifier", "exporter"];

        for module in &modules {
            for action in &actions {
                conn.execute(
                    "INSERT OR IGNORE INTO permissions (role, module, action, allowed) VALUES ('admin', ?1, ?2, 1)",
                    params![module, action],
                )?;
            }
        }

        let manager_denied = ["magasins", "audit", "settings"];
        for module in &modules {
            let allowed = if manager_denied.contains(module) {
                0
            } else {
                1
            };
            for action in &actions {
                conn.execute(
                    "INSERT OR IGNORE INTO permissions (role, module, action, allowed) VALUES ('manager', ?1, ?2, ?3)",
                    params![module, action, allowed],
                )?;
            }
        }

        let caissier_allowed: Vec<(&str, &str)> =
            vec![("ventes", "voir"), ("ventes", "creer"), ("clients", "voir")];
        for module in &modules {
            for action in &actions {
                let allowed = if caissier_allowed.contains(&(module, action)) {
                    1
                } else {
                    0
                };
                conn.execute(
                    "INSERT OR IGNORE INTO permissions (role, module, action, allowed) VALUES ('caissier', ?1, ?2, ?3)",
                    params![module, action, allowed],
                )?;
            }
        }
    }

    // Ajout des colonnes pour la migration des bases existantes
    ajouter_colonne(conn, "clients", "points_fidelite", "REAL DEFAULT 0")?;
    ajouter_colonne(conn, "ventes", "points_utilises", "REAL DEFAULT 0")?;
    ajouter_colonne(conn, "ventes", "points_gagnes", "REAL DEFAULT 0")?;

    // Configuration par défaut de la fidélité si elle n'existe pas
    conn.execute(
        "INSERT OR IGNORE INTO settings (key, value) VALUES ('fidelite_actif', 'true')",
        [],
    )?;
    conn.execute(
        "INSERT OR IGNORE INTO settings (key, value) VALUES ('fidelite_dh_pour_1_point', '100')",
        [],
    )?; // Dépenser 100 DH donne 1 point
    conn.execute(
        "INSERT OR IGNORE INTO settings (key, value) VALUES ('fidelite_valeur_1_point', '1')",
        [],
    )?;
    conn.execute(
        "INSERT OR IGNORE INTO settings (key, value) VALUES ('idle_timeout', '300')",
        [],
    )?;

    ajouter_colonne(conn, "articles", "image_url", "TEXT")?;
    ajouter_colonne(conn, "ventes", "numero_facture", "TEXT")?;
    ajouter_colonne(conn, "articles", "divers_taux", "REAL DEFAULT 0")?;
    ajouter_colonne(conn, "vente_articles", "remise_ligne", "REAL DEFAULT 0")?;
    ajouter_colonne(conn, "vente_articles", "note", "TEXT")?;
    ajouter_colonne(conn, "clients", "ice", "TEXT")?;
    ajouter_colonne(conn, "ventes", "dtype", "TEXT DEFAULT 'facture'")?;
    ajouter_colonne(conn, "achats", "statut_livraison", "TEXT DEFAULT 'recu'")?;
    ajouter_colonne(conn, "achats", "statut_paiement", "TEXT DEFAULT 'non_paye'")?;
    ajouter_colonne(conn, "ventes", "session_id", "INTEGER")?;
    ajouter_colonne(conn, "journal_caisse", "session_id", "INTEGER")?;
    ajouter_colonne(conn, "sessions_caisse", "magasin_id", "INTEGER")?;
    ajouter_colonne(conn, "articles", "suivi_lot", "INTEGER DEFAULT 0")?;
    ajouter_colonne(conn, "article_variantes", "code_barre", "TEXT")?;
    ajouter_colonne(conn, "utilisateurs", "pin_hash", "TEXT")?;
    ajouter_colonne(conn, "vente_articles", "variante_id", "INTEGER")?;
    if let Err(e) = conn.execute(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_variantes_code_barre_unique ON article_variantes(code_barre) WHERE code_barre IS NOT NULL AND code_barre != ''",
        [],
    ) {
        log::warn!("Index d'unicité des codes-barres de variantes non créé (doublons existants ?) : {}", e);
    }
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_article_variantes_article ON article_variantes(article_id)",
        [],
    )?;

    ajouter_colonne(conn, "ventes", "source_vente_id", "INTEGER")?;
    ajouter_colonne(conn, "clients", "segment", "TEXT")?;
    ajouter_colonne(conn, "ventes", "magasin_id", "INTEGER")?;

    // Multi-prix (public/grossiste) et produits composés (kits)
    ajouter_colonne(conn, "articles", "prix_grossiste", "REAL")?;
    ajouter_colonne(conn, "articles", "est_kit", "INTEGER DEFAULT 0")?;
    ajouter_colonne(conn, "vente_articles", "prix_type", "TEXT DEFAULT 'public'")?;
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS article_composants (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            article_id INTEGER NOT NULL,
            composant_id INTEGER NOT NULL,
            quantite REAL NOT NULL DEFAULT 1,
            FOREIGN KEY (article_id) REFERENCES articles(id),
            FOREIGN KEY (composant_id) REFERENCES articles(id)
        );
        CREATE INDEX IF NOT EXISTS idx_article_composants_article ON article_composants(article_id);

        CREATE TABLE IF NOT EXISTS caisses (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            nom TEXT NOT NULL,
            utilisateur_id INTEGER,
            statut TEXT NOT NULL DEFAULT 'fermee',
            ouverture_date TEXT,
            fermeture_date TEXT,
            fond_initial REAL NOT NULL DEFAULT 0,
            recettes_especes REAL NOT NULL DEFAULT 0,
            recettes_cb REAL NOT NULL DEFAULT 0,
            recettes_cheque REAL NOT NULL DEFAULT 0,
            recettes_virement REAL NOT NULL DEFAULT 0,
            depenses REAL NOT NULL DEFAULT 0,
            ecart REAL NOT NULL DEFAULT 0,
            note TEXT
        );
    ",
    )?;

    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS article_lots (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            article_id INTEGER NOT NULL,
            magasin_id INTEGER NOT NULL,
            numero_lot TEXT,
            date_peremption TEXT,
            quantite REAL NOT NULL DEFAULT 0,
            date_reception TEXT NOT NULL DEFAULT (datetime('now','localtime')),
            FOREIGN KEY (article_id) REFERENCES articles(id),
            FOREIGN KEY (magasin_id) REFERENCES magasins(id)
        );
        CREATE INDEX IF NOT EXISTS idx_article_lots_article ON article_lots(article_id);
        CREATE INDEX IF NOT EXISTS idx_article_lots_peremption ON article_lots(date_peremption);
    ",
    )?;

    // Migration en-tête légal : l'ancien champ unique 'tax_number' (ICE/IF confondus)
    // devient 'ice' ; 'if_number'/'rc_number'/'patente' sont ajoutés en distinct.
    conn.execute(
        "INSERT OR IGNORE INTO settings (key, value) SELECT 'ice', value FROM settings WHERE key = 'tax_number'",
        [],
    )?;
    conn.execute(
        "INSERT OR IGNORE INTO settings (key, value) VALUES ('if_number', '')",
        [],
    )?;
    conn.execute(
        "INSERT OR IGNORE INTO settings (key, value) VALUES ('rc_number', '')",
        [],
    )?;
    conn.execute(
        "INSERT OR IGNORE INTO settings (key, value) VALUES ('patente', '')",
        [],
    )?;
    conn.execute("DELETE FROM settings WHERE key = 'tax_number'", [])?;

    conn.execute_batch("
        CREATE INDEX IF NOT EXISTS idx_articles_code_barre ON articles(code_barre);
        CREATE INDEX IF NOT EXISTS idx_articles_actif ON articles(actif);
        CREATE INDEX IF NOT EXISTS idx_articles_categorie ON articles(categorie_id);
        CREATE INDEX IF NOT EXISTS idx_ventes_date ON ventes(date);
        CREATE INDEX IF NOT EXISTS idx_ventes_client ON ventes(client_id);
        CREATE INDEX IF NOT EXISTS idx_ventes_caissier ON ventes(caissier_id);
        CREATE INDEX IF NOT EXISTS idx_vente_articles_vente ON vente_articles(vente_id);
        CREATE INDEX IF NOT EXISTS idx_vente_articles_article ON vente_articles(article_id);
        CREATE INDEX IF NOT EXISTS idx_achats_date ON achats(date);
        CREATE INDEX IF NOT EXISTS idx_achats_fournisseur ON achats(fournisseur_id);
        CREATE INDEX IF NOT EXISTS idx_mouvements_article ON mouvements_stock(article_id);
        CREATE INDEX IF NOT EXISTS idx_mouvements_date ON mouvements_stock(date);
        CREATE INDEX IF NOT EXISTS idx_paiements_client ON paiements(client_id);
        CREATE INDEX IF NOT EXISTS idx_journal_date ON journal_caisse(date);
        CREATE UNIQUE INDEX IF NOT EXISTS idx_articles_code_barre_unique ON articles(code_barre) WHERE code_barre IS NOT NULL AND code_barre != '';
        CREATE INDEX IF NOT EXISTS idx_audit_log_date ON audit_log(date);
        CREATE INDEX IF NOT EXISTS idx_audit_log_action ON audit_log(action);
        CREATE INDEX IF NOT EXISTS idx_inventaire_lignes_inventaire ON inventaire_lignes(inventaire_id);
    ")?;

    // Migration du stock existant vers le "Magasin Principal"
    let nb_magasins: i64 = conn.query_row("SELECT count(*) FROM magasins", [], |r| r.get(0))?;
    if nb_magasins == 0 {
        conn.execute(
            "INSERT INTO magasins (nom, adresse) VALUES ('Magasin Principal', 'Siège central')",
            [],
        )?;
        // Récupérer l'ID du magasin principal
        let magasin_id = conn.last_insert_rowid();

        // Basculer la colonne 'stock' des articles existants vers 'article_stocks'
        conn.execute(
            "INSERT INTO article_stocks (article_id, magasin_id, quantite) 
             SELECT id, ?1, stock FROM articles WHERE stock != 0",
            params![magasin_id],
        )?;

        // Lier les mouvements de stock historiques au magasin principal
        ajouter_colonne(conn, "mouvements_stock", "magasin_id", "INTEGER")?;
        conn.execute(
            "UPDATE mouvements_stock SET magasin_id = ?1 WHERE magasin_id IS NULL",
            params![magasin_id],
        )?;
    } else {
        // Au cas où le champ magasin_id manque sur les mouvements pour une DB déjà migrée
        ajouter_colonne(conn, "mouvements_stock", "magasin_id", "INTEGER")?;
    }
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS numerotation (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            ntype TEXT NOT NULL UNIQUE,
            annee INTEGER NOT NULL DEFAULT (strftime('%Y','now')),
            prefixe TEXT NOT NULL,
            dernier_numero INTEGER NOT NULL DEFAULT 0
        );
        INSERT OR IGNORE INTO numerotation (ntype, prefixe, dernier_numero)
        VALUES ('facture_client', 'FA', 0);
        INSERT OR IGNORE INTO numerotation (ntype, prefixe, dernier_numero)
        VALUES ('avoir', 'AV', 0);
    ",
    )?;

    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS numerotation_v2 (
            ntype TEXT NOT NULL,
            annee INTEGER NOT NULL,
            prefixe TEXT NOT NULL,
            dernier_numero INTEGER NOT NULL DEFAULT 0,
            PRIMARY KEY (ntype, annee)
        );
        INSERT OR IGNORE INTO numerotation_v2 (ntype, annee, prefixe, dernier_numero)
        SELECT ntype, annee, prefixe, dernier_numero FROM numerotation;
    ",
    )?;
    let vente_paiements_sql: Option<String> = conn
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'vente_paiements'",
            [],
            |r| r.get(0),
        )
        .optional()?;
    if vente_paiements_sql.is_some_and(|sql| !sql.contains("'fidelite'")) {
        conn.execute_batch("ALTER TABLE vente_paiements RENAME TO vente_paiements_old;")?;
    }
    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS vente_paiements (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            vente_id INTEGER NOT NULL,
            session_id INTEGER,
            mode TEXT NOT NULL CHECK (mode IN ('especes','carte','cb','cheque','virement','credit','fidelite')),
            montant REAL NOT NULL CHECK (montant >= 0),
            FOREIGN KEY (vente_id) REFERENCES ventes(id),
            FOREIGN KEY (session_id) REFERENCES sessions_caisse(id)
        );
        CREATE INDEX IF NOT EXISTS idx_vente_paiements_vente ON vente_paiements(vente_id);
        CREATE INDEX IF NOT EXISTS idx_vente_paiements_session ON vente_paiements(session_id, mode);
        CREATE INDEX IF NOT EXISTS idx_ventes_session ON ventes(session_id);

        INSERT INTO vente_paiements (vente_id, session_id, mode, montant)
        SELECT v.id, v.session_id, v.mode_paiement,
               MAX(0, COALESCE((SELECT SUM(va.total_ligne) FROM vente_articles va WHERE va.vente_id = v.id), 0) - v.montant_remise)
        FROM ventes v
        JOIN sessions_caisse s ON s.id = v.session_id AND s.statut = 'ouverte'
        WHERE v.statut != 'annulee'
          AND COALESCE(v.dtype, 'facture') IN ('facture', 'bl')
          AND v.mode_paiement IN ('especes','carte','cb','cheque','virement','credit')
          AND NOT EXISTS (SELECT 1 FROM vente_paiements vp WHERE vp.vente_id = v.id);
    ")?;
    let vente_paiements_old: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'vente_paiements_old'",
        [],
        |r| r.get(0),
    )?;
    if vente_paiements_old > 0 {
        conn.execute_batch(
            "
            BEGIN;
            INSERT OR IGNORE INTO vente_paiements (id, vente_id, session_id, mode, montant)
            SELECT id, vente_id, session_id, mode, montant FROM vente_paiements_old;
            DROP TABLE vente_paiements_old;
            COMMIT;
        ",
        )?;
    }

    ajouter_colonne(conn, "ventes", "montant_ht", "REAL")?;
    ajouter_colonne(conn, "ventes", "montant_tva", "REAL")?;
    ajouter_colonne(conn, "vente_articles", "montant_ht", "REAL")?;
    ajouter_colonne(conn, "vente_articles", "montant_tva", "REAL")?;
    conn.execute_batch("
        BEGIN;
        UPDATE ventes
        SET montant_total = (SELECT ROUND(SUM(va.total_ligne), 2) FROM vente_articles va WHERE va.vente_id = ventes.id)
        WHERE montant_ht IS NULL AND EXISTS (SELECT 1 FROM vente_articles va WHERE va.vente_id = ventes.id);

        UPDATE vente_articles
        SET montant_ht = ROUND(
                quantite * prix_unitaire * (1 - COALESCE(remise_ligne, 0) / 100.0)
                * (CASE WHEN total_ligne < 0 THEN -1 ELSE 1 END)
                * (SELECT CASE WHEN v.montant_total != 0 THEN (v.montant_total - v.montant_remise) / v.montant_total ELSE 1 END
                   FROM ventes v WHERE v.id = vente_articles.vente_id), 2)
        WHERE montant_ht IS NULL;
        UPDATE vente_articles SET montant_tva = ROUND(montant_ht * COALESCE(tva, 0) / 100.0, 2) WHERE montant_tva IS NULL;

        UPDATE ventes
        SET montant_ht = (SELECT ROUND(SUM(va.montant_ht), 2) FROM vente_articles va WHERE va.vente_id = ventes.id),
            montant_tva = (SELECT ROUND(SUM(va.montant_tva), 2) FROM vente_articles va WHERE va.vente_id = ventes.id)
        WHERE montant_ht IS NULL AND EXISTS (SELECT 1 FROM vente_articles va WHERE va.vente_id = ventes.id);
        COMMIT;
    ")?;

    if let Err(e) = conn.execute(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_ventes_numero_facture_unique ON ventes(numero_facture) WHERE numero_facture IS NOT NULL",
        [],
    ) {
        log::warn!("Index d'unicité des numéros de document non créé (doublons existants ?) : {}", e);
    }

    // Insert default tables if empty
    let nb_tables: i64 = conn.query_row("SELECT count(*) FROM tables_resto", [], |r| r.get(0))?;
    if nb_tables == 0 {
        for i in 1..=12 {
            conn.execute(
                "INSERT INTO tables_resto (nom) VALUES (?1)",
                params![format!("Table {}", i)],
            )?;
        }
    }

    // Insert default settings if empty
    let settings_count: i64 = conn.query_row("SELECT COUNT(*) FROM settings", [], |r| r.get(0))?;
    if settings_count == 0 {
        let defaults = vec![
            ("shop_name", "SuperCaisse"),
            ("shop_address", ""),
            ("shop_phone", ""),
            ("shop_email", ""),
            ("ice", ""),
            ("if_number", ""),
            ("rc_number", ""),
            ("patente", ""),
            ("default_tva", "20"),
            ("receipt_footer", "Merci de votre visite"),
            ("currency", "MAD"),
        ];
        for (key, value) in defaults {
            conn.execute(
                "INSERT INTO settings (key, value) VALUES (?1, ?2)",
                params![key, value],
            )?;
        }
    }

    let variante_stocks_existe: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type = 'table' AND name = 'article_variante_stocks'",
        [],
        |r| r.get(0),
    )?;
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS article_variante_stocks (
            variante_id INTEGER NOT NULL,
            magasin_id INTEGER NOT NULL,
            quantite REAL NOT NULL DEFAULT 0,
            PRIMARY KEY (variante_id, magasin_id),
            FOREIGN KEY (variante_id) REFERENCES article_variantes(id),
            FOREIGN KEY (magasin_id) REFERENCES magasins(id)
        );
        CREATE TABLE IF NOT EXISTS vente_lots (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            vente_id INTEGER NOT NULL,
            lot_id INTEGER NOT NULL,
            quantite REAL NOT NULL,
            FOREIGN KEY (vente_id) REFERENCES ventes(id),
            FOREIGN KEY (lot_id) REFERENCES article_lots(id)
        );
        CREATE INDEX IF NOT EXISTS idx_vente_lots_vente ON vente_lots(vente_id);
        INSERT OR IGNORE INTO settings (key, value) VALUES ('autoriser_stock_negatif', 'false');
    ",
    )?;
    if !variante_stocks_existe {
        conn.execute(
            "INSERT INTO article_variante_stocks (variante_id, magasin_id, quantite)
             SELECT v.id, (SELECT MIN(id) FROM magasins), v.stock_dedie
             FROM article_variantes v
             WHERE COALESCE(v.stock_dedie, 0) != 0 AND EXISTS (SELECT 1 FROM magasins)",
            [],
        )?;
    }

    ajouter_colonne(
        conn,
        "utilisateurs",
        "must_change_password",
        "INTEGER NOT NULL DEFAULT 0",
    )?;
    let nb_utilisateurs: i64 =
        conn.query_row("SELECT COUNT(*) FROM utilisateurs", [], |r| r.get(0))?;
    if nb_utilisateurs == 0 {
        conn.execute(
            "INSERT INTO utilisateurs (login, password_hash, nom, role, must_change_password) VALUES (?1, ?2, ?3, ?4, 1)",
            params![
                "admin",
                hash_password("admin").map_err(|e| rusqlite::Error::ToSqlConversionFailure(e.into()))?,
                "Administrateur",
                "admin"
            ],
        )?;
    }

    Ok(())
}

pub fn hash_password(password: &str) -> std::result::Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| {
            log::error!("Hachage argon2 impossible : {}", e);
            format!("Hachage du mot de passe impossible : {}", e)
        })
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    if hash.starts_with("$argon2") {
        match PasswordHash::new(hash) {
            Ok(h) => Argon2::default()
                .verify_password(password.as_bytes(), &h)
                .is_ok(),
            Err(e) => {
                log::error!("Empreinte de mot de passe illisible en base : {}", e);
                false
            }
        }
    } else {
        use sha2::Digest;
        let sha_hash = hex::encode(sha2::Sha256::digest(password.as_bytes()));
        egal_temps_constant(sha_hash.as_bytes(), hash.as_bytes())
    }
}

pub(crate) fn egal_temps_constant(a: &[u8], b: &[u8]) -> bool {
    let mut difference = a.len() ^ b.len();
    for i in 0..a.len().max(b.len()) {
        let x = a.get(i).copied().unwrap_or(0);
        let y = b.get(i).copied().unwrap_or(0);
        difference |= usize::from(x ^ y);
    }
    difference == 0
}

pub(crate) fn verification_factice(password: &str) {
    static EMPREINTE: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
    if let Some(empreinte) = EMPREINTE.get_or_init(|| hash_password("empreinte-factice").ok()) {
        verify_password(password, empreinte);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_comparaison_temps_constant() {
        assert!(super::egal_temps_constant(b"abc", b"abc"));
        assert!(!super::egal_temps_constant(b"abc", b"abd"));
        assert!(!super::egal_temps_constant(b"abc", b"abcd"));
        assert!(!super::egal_temps_constant(b"", b"a"));
        use sha2::Digest;
        let legacy = hex::encode(sha2::Sha256::digest(b"secret"));
        assert!(super::verify_password("secret", &legacy));
        assert!(!super::verify_password("autre", &legacy));
    }

    #[test]
    fn test_migration_unicite_renomme_les_doublons_et_resynchronise() {
        let mut conn = super::Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        super::appliquer_migrations(&mut conn, &super::MIGRATIONS[..3]).unwrap();
        conn.execute_batch(
            "
            DROP INDEX idx_ventes_numero_facture_unique;
            INSERT INTO ventes (id, montant_total, numero_facture) VALUES
                (1, 10, 'FA-2026-00012'), (2, 10, 'FA-2026-00012'), (3, 10, 'FA-2026-00003'), (4, 10, 'AV-2025-00007');
            INSERT OR REPLACE INTO numerotation_v2 (ntype, annee, prefixe, dernier_numero) VALUES ('facture_client', 2026, 'FA', 2);
            INSERT INTO clients (id, nom, code) VALUES (1, 'A', 'C001'), (2, 'B', ' C001 '), (3, 'C', ''), (4, 'D', 'C002');
            ",
        )
        .unwrap();
        super::migrer(&mut conn).unwrap();
        let numero: String = conn
            .query_row("SELECT numero_facture FROM ventes WHERE id = 2", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(numero, "FA-2026-00012-DOUBLON-2");
        let codes: Vec<Option<String>> = conn
            .prepare("SELECT code FROM clients ORDER BY id")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<_>>()
            .unwrap();
        assert_eq!(
            codes,
            vec![
                Some("C001".into()),
                Some("C001-2".into()),
                None,
                Some("C002".into())
            ]
        );
        let audit: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM audit_log WHERE action = 'correction_doublons'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(audit, 1);
        assert_eq!(
            crate::commands::next_numero_document(&conn, "facture", 2026).unwrap(),
            "FA-2026-00013"
        );
        assert_eq!(
            crate::commands::next_numero_document(&conn, "avoir", 2025).unwrap(),
            "AV-2025-00008"
        );
        assert!(conn
            .execute(
                "INSERT INTO ventes (montant_total, numero_facture) VALUES (1, 'FA-2026-00003')",
                []
            )
            .is_err());
        assert!(conn
            .execute("INSERT INTO clients (nom, code) VALUES ('E', 'C002')", [])
            .is_err());
        conn.execute(
            "INSERT INTO clients (nom, code) VALUES ('F', NULL), ('G', NULL)",
            [],
        )
        .unwrap();
    }

    #[test]
    fn test_compteur_en_retard_ne_bloque_pas_la_vente() {
        let conn = super::init_db(":memory:").unwrap();
        conn.execute_batch(
            "INSERT INTO ventes (montant_total, numero_facture) VALUES (10, 'BL-2026-00001'), (10, 'BL-2026-00002');",
        )
        .unwrap();
        assert_eq!(
            crate::commands::next_numero_document(&conn, "bl", 2026).unwrap(),
            "BL-2026-00003"
        );
        assert_eq!(
            crate::commands::next_numero_document(&conn, "bl", 2026).unwrap(),
            "BL-2026-00004"
        );
    }

    fn base_v2() -> super::Connection {
        let mut conn = super::Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        super::appliquer_migrations(&mut conn, &super::MIGRATIONS[..2]).unwrap();
        conn.execute_batch(
            "
            INSERT INTO utilisateurs (id, login, password_hash, nom, role) VALUES (2, 'c', 'x', 'Caissier', ' Caissier ');
            INSERT INTO sessions_caisse (id, caissier_id, statut, magasin_id) VALUES (1, 2, 'fermee', 42);
            INSERT INTO articles (id, designation, prix_vente) VALUES (1, 'A', 10);
            INSERT INTO ventes (id, montant_total, mode_paiement, statut, dtype, session_id, magasin_id, numero_facture)
                VALUES (1, 12, 'especes+cb', 'Validee', 'facture', 1, 99, 'FA-1'),
                       (7, 12, 'cb', 'validee', 'facture', 5, 1, 'FA-2');
            INSERT INTO vente_articles (vente_id, article_id, quantite, prix_unitaire, total_ligne, variante_id)
                VALUES (1, 1, 1, 10, 12, 77);
            INSERT INTO journal_caisse (jtype, montant, session_id) VALUES ('Entree', 5, 1);
            INSERT INTO mouvements_stock (article_id, quantite, mtype) VALUES (1, 1, 'vente');
            DELETE FROM ventes WHERE id = 7;
            ",
        )
        .unwrap();
        conn
    }

    #[test]
    fn test_migration_contraintes_normalise_et_conserve_les_donnees() {
        let mut conn = base_v2();
        super::migrer(&mut conn).unwrap();
        assert_eq!(super::version_schema(&conn).unwrap(), super::SCHEMA_VERSION);
        let cles: bool = conn
            .pragma_query_value(None, "foreign_keys", |r| r.get(0))
            .unwrap();
        assert!(cles);
        let vente: (String, String, Option<i64>, Option<i64>) = conn
            .query_row(
                "SELECT mode_paiement, statut, session_id, magasin_id FROM ventes WHERE id = 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .unwrap();
        assert_eq!(vente, ("mixte".into(), "validee".into(), Some(1), None));
        let role: String = conn
            .query_row("SELECT role FROM utilisateurs WHERE id = 2", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(role, "caissier");
        let (statut, magasin): (String, Option<i64>) = conn
            .query_row("SELECT statut, magasin_id FROM sessions_caisse", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .unwrap();
        assert_eq!((statut.as_str(), magasin), ("cloturee", None));
        let variante: Option<i64> = conn
            .query_row("SELECT variante_id FROM vente_articles", [], |r| r.get(0))
            .unwrap();
        assert_eq!(variante, None);
        let mtype: String = conn
            .query_row("SELECT mtype FROM mouvements_stock", [], |r| r.get(0))
            .unwrap();
        assert_eq!(mtype, "sortie");
        let index: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'index' AND name IN ('idx_ventes_date', 'idx_ventes_numero_facture_unique', 'idx_vente_articles_vente', 'idx_journal_date')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(index, 4);
        conn.execute("INSERT INTO ventes (montant_total) VALUES (1)", [])
            .unwrap();
        assert!(conn.last_insert_rowid() > 7);
    }

    #[test]
    fn test_contraintes_actives_apres_migration() {
        let conn = super::init_db(":memory:").unwrap();
        for sql in [
            "INSERT INTO ventes (montant_total, mode_paiement) VALUES (10, 'Especes')",
            "INSERT INTO ventes (montant_total, dtype) VALUES (10, 'ticket')",
            "INSERT INTO ventes (montant_total, statut) VALUES (10, 'ok')",
            "INSERT INTO ventes (montant_total) VALUES (-10)",
            "INSERT INTO ventes (montant_total, magasin_id) VALUES (10, 999)",
            "INSERT INTO ventes (montant_total, session_id) VALUES (10, 999)",
            "INSERT INTO journal_caisse (jtype, montant) VALUES ('autre', 1)",
            "INSERT INTO utilisateurs (login, password_hash, nom, role) VALUES ('x', 'x', 'X', 'patron')",
            "INSERT INTO mouvements_stock (article_id, quantite, mtype) VALUES (1, 1, 'vente')",
            "INSERT INTO cheques (numero, banque, montant, date_emission, date_echeance, ctype) VALUES ('1', 'B', 10, 'd', 'd', 'autre')",
            "INSERT INTO sessions_caisse (caissier_id, statut) VALUES (1, 'fermee')",
        ] {
            assert!(conn.execute(sql, []).is_err(), "accepté : {sql}");
        }
        conn.execute(
            "INSERT INTO ventes (montant_total, montant_remise, dtype) VALUES (-10, -1, 'avoir')",
            [],
        )
        .unwrap();
        conn.execute("INSERT INTO articles (id, designation) VALUES (1, 'A')", [])
            .unwrap();
        let vente = conn.last_insert_rowid();
        assert!(conn
            .execute(
                "INSERT INTO vente_articles (vente_id, article_id, quantite, prix_unitaire, total_ligne) VALUES (?1, 1, 0, 1, 1)",
                [vente],
            )
            .is_err());
    }

    #[test]
    fn test_migration_contraintes_refuse_une_valeur_inconnue_sans_rien_modifier() {
        let mut conn = base_v2();
        conn.execute("UPDATE ventes SET dtype = 'ticket' WHERE id = 1", [])
            .unwrap();
        let err = super::migrer(&mut conn).unwrap_err();
        assert!(
            err.contains("ventes.dtype") && err.contains("ticket"),
            "{err}"
        );
        assert_eq!(super::version_schema(&conn).unwrap(), 2);
        let mode: String = conn
            .query_row("SELECT mode_paiement FROM ventes WHERE id = 1", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(mode, "especes+cb");
        let cles: bool = conn
            .pragma_query_value(None, "foreign_keys", |r| r.get(0))
            .unwrap();
        assert!(cles);
    }

    #[test]
    fn test_lecteurs_paralleles_a_l_ecriture() {
        let dossier = std::env::temp_dir().join(format!("sc_lecteurs_{}", std::process::id()));
        std::fs::create_dir_all(&dossier).unwrap();
        let chemin = dossier.join("base.db");
        let conn = super::init_db(chemin.to_str().unwrap()).unwrap();
        let etat = super::DbState::avec_lecteurs(conn, &chemin, super::LECTEURS).unwrap();
        {
            let ecriture = etat.conn.lock().unwrap();
            ecriture
                .execute("INSERT INTO clients (nom) VALUES ('Visible')", [])
                .unwrap();
            let gardes: Vec<_> = (0..super::LECTEURS)
                .map(|_| etat.lecture().unwrap())
                .collect();
            let n: i64 = gardes[0]
                .query_row(
                    "SELECT COUNT(*) FROM clients WHERE nom = 'Visible'",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(n, 1);
            let refus = gardes[1].execute("INSERT INTO clients (nom) VALUES ('X')", []);
            assert!(refus.is_err());
        }
        drop(etat.lecture().unwrap());
        drop(etat);
        std::fs::remove_dir_all(&dossier).unwrap();
    }

    #[test]
    fn test_lecture_sans_lecteurs_utilise_la_connexion_principale() {
        let etat = super::DbState::new(super::init_db(":memory:").unwrap());
        let n: i64 = etat
            .lecture()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM utilisateurs", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 1);
    }

    #[test]
    fn test_migration_arrondit_les_montants_au_centime() {
        let mut conn = super::Connection::open_in_memory().unwrap();
        super::appliquer_migrations(&mut conn, &super::MIGRATIONS[..1]).unwrap();
        conn.execute_batch(
            "
            INSERT INTO clients (id, nom, credit_actuel, credit_plafond) VALUES (1, 'C', 0.1 + 0.2, NULL);
            INSERT INTO ventes (id, montant_total, montant_remise, montant_ht, montant_tva) VALUES (1, 23.999999999, 1.005, 20.004, -0.015);
            INSERT INTO sessions_caisse (id, caissier_id, fond_initial, ecart) VALUES (1, 1, 100, -3.3333);
            ",
        )
        .unwrap();
        super::migrer(&mut conn).unwrap();
        assert_eq!(super::version_schema(&conn).unwrap(), super::SCHEMA_VERSION);
        let (credit, plafond): (f64, Option<f64>) = conn
            .query_row(
                "SELECT credit_actuel, credit_plafond FROM clients",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(credit, 0.3);
        assert_eq!(plafond, None);
        let v: (f64, f64, f64, f64) = conn
            .query_row(
                "SELECT montant_total, montant_remise, montant_ht, montant_tva FROM ventes",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .unwrap();
        assert_eq!(v, (24.0, 1.01, 20.0, -0.02));
        let ecart: f64 = conn
            .query_row("SELECT ecart FROM sessions_caisse", [], |r| r.get(0))
            .unwrap();
        assert_eq!(ecart, -3.33);
        for (table, colonnes) in super::COLONNES_MONTANTS {
            for colonne in *colonnes {
                assert!(
                    super::colonne_existe(&conn, table, colonne).unwrap(),
                    "{table}.{colonne}"
                );
            }
        }
    }

    use super::*;

    fn fichier(nom: &str) -> String {
        std::env::temp_dir()
            .join(format!(
                "supercaisse_test_schema_{}_{}_{}.db",
                nom,
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ))
            .to_string_lossy()
            .to_string()
    }

    fn nettoyer(chemin: &str) {
        for suffixe in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{}", chemin, suffixe));
        }
    }

    #[test]
    fn test_nouvelle_base_au_schema_courant() {
        let conn = init_db(":memory:").unwrap();
        assert_eq!(version_schema(&conn).unwrap(), SCHEMA_VERSION);
    }

    #[test]
    fn test_base_plus_recente_refusee() {
        let chemin = fichier("recente");
        {
            let conn = Connection::open(&chemin).unwrap();
            conn.pragma_update(None, "user_version", SCHEMA_VERSION + 1)
                .unwrap();
        }
        let err = init_db(&chemin).unwrap_err();
        assert!(err.contains("plus récente"), "{err}");
        nettoyer(&chemin);
    }

    #[test]
    fn test_base_ancienne_mise_a_niveau() {
        let chemin = fichier("ancienne");
        {
            let conn = Connection::open(&chemin).unwrap();
            conn.execute_batch(
                "
                CREATE TABLE ventes (id INTEGER PRIMARY KEY AUTOINCREMENT, date TEXT NOT NULL DEFAULT (datetime('now','localtime')),
                    client_id INTEGER, caissier_id INTEGER, montant_total REAL DEFAULT 0, montant_remise REAL DEFAULT 0,
                    mode_paiement TEXT DEFAULT 'especes', statut TEXT DEFAULT 'validee');
                INSERT INTO ventes (montant_total) VALUES (10);
                CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
                INSERT INTO settings (key, value) VALUES ('tax_number', '001234567000089');
            ",
            )
            .unwrap();
        }
        let conn = init_db(&chemin).unwrap();
        assert_eq!(version_schema(&conn).unwrap(), SCHEMA_VERSION);
        for colonne in [
            "numero_facture",
            "dtype",
            "session_id",
            "magasin_id",
            "montant_ht",
            "montant_tva",
        ] {
            assert!(
                colonne_existe(&conn, "ventes", colonne).unwrap(),
                "{colonne}"
            );
        }
        let ice: String = conn
            .query_row("SELECT value FROM settings WHERE key = 'ice'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(ice, "001234567000089");
        drop(conn);
        let conn = init_db(&chemin).unwrap();
        let nb: i64 = conn
            .query_row("SELECT COUNT(*) FROM ventes", [], |r| r.get(0))
            .unwrap();
        assert_eq!(nb, 1);
        drop(conn);
        nettoyer(&chemin);
    }

    #[test]
    fn test_erreurs_de_migration_propagees() {
        let conn = Connection::open_in_memory().unwrap();
        assert!(ajouter_colonne(&conn, "table_absente", "x", "TEXT").is_err());
        conn.execute_batch("CREATE TABLE t (a TEXT)").unwrap();
        ajouter_colonne(&conn, "t", "b", "INTEGER DEFAULT 0").unwrap();
        ajouter_colonne(&conn, "t", "b", "INTEGER DEFAULT 0").unwrap();
        assert!(colonne_existe(&conn, "t", "b").unwrap());
    }

    fn m1(conn: &Connection) -> Result<()> {
        conn.execute_batch("CREATE TABLE t (a INTEGER)")
    }

    fn m2_echoue(conn: &Connection) -> Result<()> {
        conn.execute_batch("INSERT INTO t VALUES (1); INSERT INTO table_absente VALUES (2);")
    }

    fn m2(conn: &Connection) -> Result<()> {
        conn.execute_batch("INSERT INTO t VALUES (1)")
    }

    #[test]
    fn test_migration_en_echec_annulee() {
        let mut conn = Connection::open_in_memory().unwrap();
        let err = appliquer_migrations(&mut conn, &[m1, m2_echoue]).unwrap_err();
        assert!(err.starts_with("Migration v2"), "{err}");
        assert_eq!(version_schema(&conn).unwrap(), 1);
        let nb: i64 = conn
            .query_row("SELECT COUNT(*) FROM t", [], |r| r.get(0))
            .unwrap();
        assert_eq!(nb, 0);
        appliquer_migrations(&mut conn, &[m1, m2]).unwrap();
        assert_eq!(version_schema(&conn).unwrap(), 2);
        appliquer_migrations(&mut conn, &[m1, m2]).unwrap();
        let nb: i64 = conn
            .query_row("SELECT COUNT(*) FROM t", [], |r| r.get(0))
            .unwrap();
        assert_eq!(nb, 1);
    }
}
