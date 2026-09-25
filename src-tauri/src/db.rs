use rusqlite::{Connection, Result, params};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use argon2::password_hash::{SaltString, rand_core::OsRng};

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
pub struct Article {
    pub id: Option<i64>,
    pub code_barre: Option<String>,
    pub designation: String,
    pub description: Option<String>,
    pub prix_achat: f64,
    pub prix_vente: f64,
    pub tva: f64,
    pub stock: f64,
    pub stock_alerte: Option<f64>,
    pub categorie_id: Option<i64>,
    pub fournisseur_id: Option<i64>,
    pub actif: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Vente {
    pub id: Option<i64>,
    pub date: Option<String>,
    pub client_id: Option<i64>,
    pub caissier_id: Option<i64>,
    pub montant_total: f64,
    pub montant_remise: f64,
    pub mode_paiement: String,
    pub statut: String,
    pub dtype: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VenteArticle {
    pub id: Option<i64>,
    pub vente_id: i64,
    pub article_id: i64,
    pub quantite: f64,
    pub prix_unitaire: f64,
    pub tva: f64,
    pub total_ligne: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Achat {
    pub id: Option<i64>,
    pub date: Option<String>,
    pub fournisseur_id: Option<i64>,
    pub reference: Option<String>,
    pub montant_total: f64,
    pub statut: String,
    pub statut_livraison: String,
    pub statut_paiement: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AchatArticle {
    pub id: Option<i64>,
    pub achat_id: i64,
    pub article_id: i64,
    pub quantite: f64,
    pub prix_unitaire: f64,
    pub total_ligne: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Utilisateur {
    pub id: Option<i64>,
    pub login: String,
    pub nom: String,
    pub role: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Paiement {
    pub id: Option<i64>,
    pub client_id: i64,
    pub date: Option<String>,
    pub montant: f64,
    pub ptype: String,
    pub reference: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Cheque {
    pub id: Option<i64>,
    pub numero: String,
    pub banque: String,
    pub tireur: Option<String>,
    pub montant: f64,
    pub date_emission: String,
    pub date_echeance: String,
    pub statut: String, // en_attente, encaisse, impaye
    pub ctype: String, // client, fournisseur
    pub client_id: Option<i64>,
    pub fournisseur_id: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UtilisateurForm {
    pub id: Option<i64>,
    pub login: String,
    pub nom: String,
    pub role: String,
    pub password: Option<String>,
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

pub fn init_db(db_path: &str) -> Result<Connection> {
    let conn = Connection::open(db_path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;

    conn.execute_batch("
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
    ")?;

    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS permissions (
            role TEXT NOT NULL,
            module TEXT NOT NULL,
            action TEXT NOT NULL,
            allowed INTEGER NOT NULL DEFAULT 1,
            PRIMARY KEY (role, module, action)
        );
    ")?;

    {
        let modules = vec![
            "articles", "categories", "clients", "fournisseurs", "ventes",
            "achats", "stock", "inventaire", "journal", "cheques",
            "rapports", "magasins", "audit", "settings", "reappro",
        ];
        let actions = vec!["voir", "creer", "modifier", "exporter"];

        for module in &modules {
            for action in &actions {
                conn.execute(
                    "INSERT OR IGNORE INTO permissions (role, module, action, allowed) VALUES ('admin', ?1, ?2, 1)",
                    params![module, action],
                ).ok();
            }
        }

        let manager_denied = vec!["magasins", "audit", "settings"];
        for module in &modules {
            let allowed = if manager_denied.contains(module) { 0 } else { 1 };
            for action in &actions {
                conn.execute(
                    "INSERT OR IGNORE INTO permissions (role, module, action, allowed) VALUES ('manager', ?1, ?2, ?3)",
                    params![module, action, allowed],
                ).ok();
            }
        }

        let caissier_allowed: Vec<(&str, &str)> = vec![
            ("ventes", "voir"), ("ventes", "creer"), ("clients", "voir"),
        ];
        for module in &modules {
            for action in &actions {
                let allowed = if caissier_allowed.contains(&(module, action)) { 1 } else { 0 };
                conn.execute(
                    "INSERT OR IGNORE INTO permissions (role, module, action, allowed) VALUES ('caissier', ?1, ?2, ?3)",
                    params![module, action, allowed],
                ).ok();
            }
        }
    }

    // Ajout des colonnes pour la migration des bases existantes
    let _ = conn.execute("ALTER TABLE clients ADD COLUMN points_fidelite REAL DEFAULT 0", []);
    let _ = conn.execute("ALTER TABLE ventes ADD COLUMN points_utilises REAL DEFAULT 0", []);
    let _ = conn.execute("ALTER TABLE ventes ADD COLUMN points_gagnes REAL DEFAULT 0", []);

    // Configuration par défaut de la fidélité si elle n'existe pas
    conn.execute("INSERT OR IGNORE INTO settings (key, value) VALUES ('fidelite_actif', 'true')", [])?;
    conn.execute("INSERT OR IGNORE INTO settings (key, value) VALUES ('fidelite_dh_pour_1_point', '100')", [])?; // Dépenser 100 DH donne 1 point
    conn.execute("INSERT OR IGNORE INTO settings (key, value) VALUES ('fidelite_valeur_1_point', '1')", [])?;
    conn.execute("INSERT OR IGNORE INTO settings (key, value) VALUES ('idle_timeout', '300')", [])?;

    let _ = conn.execute("ALTER TABLE articles ADD COLUMN image_url TEXT", []);
    let _ = conn.execute("ALTER TABLE ventes ADD COLUMN numero_facture TEXT", []);
    let _ = conn.execute("ALTER TABLE articles ADD COLUMN divers_taux REAL DEFAULT 0", []);
    let _ = conn.execute("ALTER TABLE vente_articles ADD COLUMN remise_ligne REAL DEFAULT 0", []);
    let _ = conn.execute("ALTER TABLE vente_articles ADD COLUMN note TEXT", []);
    let _ = conn.execute("ALTER TABLE clients ADD COLUMN ice TEXT", []);
    let _ = conn.execute("ALTER TABLE ventes ADD COLUMN dtype TEXT DEFAULT 'facture'", []);
    let _ = conn.execute("ALTER TABLE achats ADD COLUMN statut_livraison TEXT DEFAULT 'recu'", []);
    let _ = conn.execute("ALTER TABLE achats ADD COLUMN statut_paiement TEXT DEFAULT 'non_paye'", []);
    let _ = conn.execute("ALTER TABLE ventes ADD COLUMN session_id INTEGER", []);
    let _ = conn.execute("ALTER TABLE journal_caisse ADD COLUMN session_id INTEGER", []);
    let _ = conn.execute("ALTER TABLE sessions_caisse ADD COLUMN magasin_id INTEGER", []);
    let _ = conn.execute("ALTER TABLE articles ADD COLUMN suivi_lot INTEGER DEFAULT 0", []);
    let _ = conn.execute("ALTER TABLE article_variantes ADD COLUMN code_barre TEXT", []);
    let _ = conn.execute("ALTER TABLE utilisateurs ADD COLUMN pin_hash TEXT", []);
    let _ = conn.execute("ALTER TABLE vente_articles ADD COLUMN variante_id INTEGER", []);
    conn.execute(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_variantes_code_barre_unique ON article_variantes(code_barre) WHERE code_barre IS NOT NULL AND code_barre != ''",
        [],
    ).ok();
    let _ = conn.execute("CREATE INDEX IF NOT EXISTS idx_article_variantes_article ON article_variantes(article_id)", []);

    let _ = conn.execute("ALTER TABLE ventes ADD COLUMN source_vente_id INTEGER", []);
    let _ = conn.execute("ALTER TABLE clients ADD COLUMN segment TEXT", []);
    let _ = conn.execute("ALTER TABLE ventes ADD COLUMN magasin_id INTEGER", []);

    // Multi-prix (public/grossiste) et produits composés (kits)
    let _ = conn.execute("ALTER TABLE articles ADD COLUMN prix_grossiste REAL", []);
    let _ = conn.execute("ALTER TABLE articles ADD COLUMN est_kit INTEGER DEFAULT 0", []);
    let _ = conn.execute("ALTER TABLE vente_articles ADD COLUMN prix_type TEXT DEFAULT 'public'", []);
    conn.execute_batch("
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
    ")?;

    conn.execute_batch("
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
    ")?;

    // Migration en-tête légal : l'ancien champ unique 'tax_number' (ICE/IF confondus)
    // devient 'ice' ; 'if_number'/'rc_number'/'patente' sont ajoutés en distinct.
    conn.execute(
        "INSERT OR IGNORE INTO settings (key, value) SELECT 'ice', value FROM settings WHERE key = 'tax_number'",
        [],
    ).ok();
    conn.execute("INSERT OR IGNORE INTO settings (key, value) VALUES ('if_number', '')", []).ok();
    conn.execute("INSERT OR IGNORE INTO settings (key, value) VALUES ('rc_number', '')", []).ok();
    conn.execute("INSERT OR IGNORE INTO settings (key, value) VALUES ('patente', '')", []).ok();
    conn.execute("DELETE FROM settings WHERE key = 'tax_number'", []).ok();

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
    let nb_magasins: i64 = conn.query_row("SELECT count(*) FROM magasins", [], |r| r.get(0)).unwrap_or(0);
    if nb_magasins == 0 {
        conn.execute("INSERT INTO magasins (nom, adresse) VALUES ('Magasin Principal', 'Siège central')", [])?;
        // Récupérer l'ID du magasin principal
        let magasin_id = conn.last_insert_rowid();
        
        // Basculer la colonne 'stock' des articles existants vers 'article_stocks'
        conn.execute(
            "INSERT INTO article_stocks (article_id, magasin_id, quantite) 
             SELECT id, ?1, stock FROM articles WHERE stock > 0", 
            params![magasin_id]
        )?;
        
        // Lier les mouvements de stock historiques au magasin principal
        let _ = conn.execute("ALTER TABLE mouvements_stock ADD COLUMN magasin_id INTEGER", []);
        conn.execute("UPDATE mouvements_stock SET magasin_id = ?1 WHERE magasin_id IS NULL", params![magasin_id])?;
    } else {
        // Au cas où le champ magasin_id manque sur les mouvements pour une DB déjà migrée
        let _ = conn.execute("ALTER TABLE mouvements_stock ADD COLUMN magasin_id INTEGER", []);
    }
    conn.execute_batch("
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
    ")?;

    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS numerotation_v2 (
            ntype TEXT NOT NULL,
            annee INTEGER NOT NULL,
            prefixe TEXT NOT NULL,
            dernier_numero INTEGER NOT NULL DEFAULT 0,
            PRIMARY KEY (ntype, annee)
        );
        INSERT OR IGNORE INTO numerotation_v2 (ntype, annee, prefixe, dernier_numero)
        SELECT ntype, annee, prefixe, dernier_numero FROM numerotation;
    ")?;
    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS vente_paiements (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            vente_id INTEGER NOT NULL,
            session_id INTEGER,
            mode TEXT NOT NULL CHECK (mode IN ('especes','carte','cb','cheque','virement','credit')),
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
    if let Err(e) = conn.execute(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_ventes_numero_facture_unique ON ventes(numero_facture) WHERE numero_facture IS NOT NULL",
        [],
    ) {
        log::warn!("Index d'unicité des numéros de document non créé (doublons existants ?) : {}", e);
    }

    // Insert default tables if empty
    let nb_tables: i64 = conn.query_row("SELECT count(*) FROM tables_resto", [], |r| r.get(0)).unwrap_or(0);
    if nb_tables == 0 {
        for i in 1..=12 {
            conn.execute("INSERT INTO tables_resto (nom) VALUES (?1)", params![format!("Table {}", i)])?;
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
            conn.execute("INSERT INTO settings (key, value) VALUES (?1, ?2)", params![key, value])?;
        }
    }

    // Create default admin if not exists
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM utilisateurs WHERE login = 'admin'",
        [],
        |r| r.get(0),
    )?;
    if count == 0 {
        let hash = hash_password("admin");
        conn.execute(
            "INSERT INTO utilisateurs (login, password_hash, nom, role) VALUES (?1, ?2, ?3, ?4)",
            params!["admin", hash, "Administrateur", "admin"],
        )?;
    }

    Ok(conn)
}

pub fn hash_password(password: &str) -> String {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    argon2
        .hash_password(password.as_bytes(), &salt)
        .expect("Erreur hachage argon2")
        .to_string()
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    if hash.starts_with("$argon2") {
        let parsed = PasswordHash::new(hash).ok();
        parsed.map(|h| Argon2::default().verify_password(password.as_bytes(), &h).is_ok()).unwrap_or(false)
    } else {
        use sha2::Digest;
        let sha_hash = hex::encode(sha2::Sha256::digest(password.as_bytes()));
        sha_hash == hash
    }
}

pub fn authenticate(conn: &mut Connection, login: &str, password: &str) -> Result<Option<Utilisateur>, String> {
    let mut stmt = conn.prepare(
        "SELECT id, login, nom, role, password_hash FROM utilisateurs WHERE login = ?1"
    ).map_err(|e| e.to_string())?;
    let result = stmt.query_row(params![login], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
        ))
    }).ok();
    match result {
        None => Ok(None),
        Some((id, ulogin, nom, role, hash)) => {
            if !verify_password(password, &hash) {
                return Ok(None);
            }
            if !hash.starts_with("$argon2") {
                let new_hash = hash_password(password);
                let _ = conn.execute(
                    "UPDATE utilisateurs SET password_hash = ?1 WHERE id = ?2",
                    params![new_hash, id],
                );
            }
            Ok(Some(Utilisateur { id: Some(id), login: ulogin, nom, role }))
        }
    }
}
