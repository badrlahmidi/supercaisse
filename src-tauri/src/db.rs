use argon2::password_hash::{rand_core::OsRng, SaltString};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use rusqlite::{params, Connection, OptionalExtension, Result};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

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
}

pub const SCHEMA_VERSION: i64 = MIGRATIONS.len() as i64;

type Migration = fn(&Connection) -> Result<()>;

const MIGRATIONS: &[Migration] = &[migration_001_base];

pub fn init_db(db_path: &str) -> std::result::Result<Connection, String> {
    let mut conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
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
            let tx = conn.transaction().map_err(|e| e.to_string())?;
            migration(&tx).map_err(|e| format!("Migration v{} : {}", cible, e))?;
            tx.pragma_update(None, "user_version", cible)
                .map_err(|e| e.to_string())?;
            tx.commit().map_err(|e| e.to_string())?;
        }
        log::info!("Schéma de base migré en v{}", cible);
    }
    Ok(())
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
    let nb_magasins: i64 = conn
        .query_row("SELECT count(*) FROM magasins", [], |r| r.get(0))
        .unwrap_or(0);
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
    let nb_tables: i64 = conn
        .query_row("SELECT count(*) FROM tables_resto", [], |r| r.get(0))
        .unwrap_or(0);
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
        let parsed = PasswordHash::new(hash).ok();
        parsed
            .map(|h| {
                Argon2::default()
                    .verify_password(password.as_bytes(), &h)
                    .is_ok()
            })
            .unwrap_or(false)
    } else {
        use sha2::Digest;
        let sha_hash = hex::encode(sha2::Sha256::digest(password.as_bytes()));
        sha_hash == hash
    }
}

#[cfg(test)]
mod tests {
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
