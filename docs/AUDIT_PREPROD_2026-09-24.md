# SuperCaisse — Audit pré-production

- **Date** : 2026-09-24
- **Périmètre** : branche `claude/audit-retail-maroc-architecture-o8lhtj`, commit `ffaf8fa`
- **Méthode** : lecture intégrale du backend Rust (`lib.rs`, `db.rs`, 28 modules `commands/`) et des fichiers frontend clés (`POS.tsx`, `AuthContext.tsx`, `cart.ts`, `lib/tauri.ts`, `lib/receipt.ts`, `router.tsx`, hooks, `Settings.tsx`, `Articles.tsx`).
- **Vérifications exécutées** :
  - `npm ci`, `tsc --noEmit` (OK), `vitest` (19 fichiers, 81 tests, tous verts), `vite build` (87 chunks) et `oxlint` (2 warnings).
  - Reproductions SQLite (Python) de la numérotation et de la requête de clôture de session.
  - Lecture des sources `tauri-macros 2.5` / `tauri 2.8` pour confirmer la convention de nommage des arguments IPC.

> Correction du brief : la colonne `mouvements_stock.magasin_id` n'est **pas** ajoutée deux fois (`db.rs:624` et `db.rs:628` sont dans les deux branches d'un `if/else`, une seule s'exécute). Le vrai problème de cette migration est ailleurs (voir M-12).

---

## Synthèse exécutive

En l'état, **SuperCaisse ne peut pas partir en production.** Quatre défauts bloquent le fonctionnement de base ou la conformité, indépendamment de toute attaque :

1. **Toute vente échouera à partir du 1er janvier 2027** : la numérotation des factures ne gère pas le changement d'année (C-3, reproduit).
2. **Les erreurs du backend sont masquées par des données fictives** : `invoke()` rattrape *toute* erreur Rust et renvoie la réponse mock. Une vente refusée, par exemple pour plafond de crédit dépassé, s'affiche comme réussie avec un faux numéro `FA-2026-xxxxx` (C-1).
3. **Plusieurs écrans n'enregistrent rien en production** : ils envoient des clés `snake_case` alors que Tauri v2 attend du `camelCase`, et le mock masque l'erreur (C-2). Sont touchés :
   - la création d'article ;
   - les paramètres boutique (ICE/IF/RC/Patente) ;
   - la restauration de sauvegarde ;
   - les lots et les variantes ;
   - les chèques ;
   - les points fidélité ;
   - le plafond de crédit client.
4. **Aucun contrôle d'accès côté backend** : les 97 commandes IPC acceptent n'importe quel appelant, et le rôle vient du `localStorage` (C-5).

Points positifs :
- Aucune injection SQL : toutes les valeurs utilisateur passent par `params!`, et les `format!` n'interpolent que des constantes.
- Les opérations multi-tables critiques (`create_vente`, `annuler_vente`, `convert_document`, `validate_transfert`, `add_paiement`, `import_articles_csv`) sont transactionnelles.
- Argon2id est utilisé avec les paramètres par défaut du crate (m=19 MiB, t=2, p=1, sel aléatoire `OsRng`), ce qui est conforme aux recommandations OWASP.
- Le code splitting fonctionne (87 chunks).

**Note globale : 33 / 100** (détail en fin de document).

---

## Couche 1 — Sécurité & Authentification

### [CRITIQUE] C-5 — Aucune autorisation côté backend ; identité et rôle fournis par le client

> **Statut : corrigé** sur `claude/hopeful-clarke-4uflms` (couvre aussi M-10).
> - **Sessions :** `login`/`login_pin` ouvrent une session serveur (`src-tauri/src/session.rs`) et renvoient un jeton aléatoire de 256 bits. Le jeton expire après 1 h d'inactivité ou 16 h au total ; `logout` le révoque.
> - **Contrôle par commande :** les 96 commandes protégées exigent le jeton et vérifient l'accès (connecté, module/action de la table `permissions`, ou administrateur). Le rôle est relu en base à chaque appel ; un utilisateur supprimé est déconnecté et ses sessions sont fermées si l'admin change son mot de passe.
> - **Réservé à l'administrateur, codé en dur :** gestion des utilisateurs et des permissions, PIN, restauration de base. Un manager ne peut donc pas s'octroyer de droits.
> - **Identité :** l'identifiant de caissier ou d'utilisateur vient de la session et non plus du frontend (`create_vente`, sessions de caisse, journal, inventaire, transferts, `change_password`). L'audit est attribué à l'auteur. Clôture et rapport X d'une session d'un autre caissier exigent la permission `journal`.
> - **Frontend :** le jeton reste en mémoire (plus rien dans `localStorage`, redémarrer impose de se reconnecter). `invoke()` l'injecte et déconnecte à l'expiration. Le tableau de bord est réservé à admin/manager, un caissier arrive sur la caisse.
> - **Tests :** 8 tests Rust, dont un qui échoue si une commande ne vérifie pas la session, et 9 tests Vitest. Vérifié en lançant l'application : connexion admin, changement de mot de passe forcé, connexion caissier, ouverture de caisse et vente attribuée au caissier de la session.

**Fichier** : `src-tauri/src/commands/*.rs` (97 commandes), `src/context/AuthContext.tsx:58-62`, `src/routes/router.tsx`
**Risque** :
- `login` renvoie un objet `Utilisateur` sans jeton. Le frontend le stocke dans `localStorage["supercaisse_user"]` et le relit au démarrage sans le revalider.
- Aucune commande Rust ne vérifie qui appelle. `caissier_id` et `utilisateur_id` sont des paramètres fournis par le client.
- La table `permissions` n'est lue que par l'UI (`get_permissions`). `update_permission`, `update_utilisateur` (changement de rôle), `import_database`, `annuler_vente` et `update_settings` sont appelables par n'importe quel code JS de la webview.

Conséquences :
- Un caissier peut s'attribuer le rôle admin en modifiant le `localStorage` (DevTools si activés, ou XSS, voir M-7).
- Via n'importe quelle faille, on peut appeler directement `annuler_vente` ou `update_utilisateur`.
- L'audit log est falsifiable, puisque l'`utilisateur_id` vient du client.

**Fix** : gérer la session dans l'état Rust et vérifier la permission dans chaque commande.

```rust
// src-tauri/src/auth_state.rs
use std::{collections::HashMap, sync::Mutex, time::{Duration, Instant}};

pub struct SessionInfo { pub user_id: i64, pub role: String, pub last_seen: Instant }
#[derive(Default)]
pub struct AuthState { pub sessions: Mutex<HashMap<String, SessionInfo>> }

const IDLE: Duration = Duration::from_secs(15 * 60);

pub fn require(auth: &AuthState, conn: &rusqlite::Connection, token: &str, module: &str, action: &str)
    -> Result<SessionInfo, String>
{
    let mut map = auth.sessions.lock().map_err(|e| e.to_string())?;
    let s = map.get_mut(token).ok_or("Session invalide")?;
    if s.last_seen.elapsed() > IDLE { map.remove(token); return Err("Session expirée".into()); }
    s.last_seen = Instant::now();
    if s.role != "admin" {
        let allowed: bool = conn.query_row(
            "SELECT allowed FROM permissions WHERE role=?1 AND module=?2 AND action=?3",
            rusqlite::params![s.role, module, action], |r| r.get(0),
        ).unwrap_or(false);
        if !allowed { return Err("Accès refusé".into()); }
    }
    Ok(SessionInfo { user_id: s.user_id, role: s.role.clone(), last_seen: s.last_seen })
}

// login : générer un jeton aléatoire (rand::rngs::OsRng, 32 octets hex), l'insérer dans AuthState, le renvoyer.
// Chaque commande :
#[tauri::command]
pub fn annuler_vente(db: State<DbState>, auth: State<AuthState>, token: String, vente_id: i64, motif: String) -> Result<(), String> {
    let mut conn = db.conn.lock().map_err(|e| e.to_string())?;
    let me = require(&auth, &conn, &token, "ventes", "annuler")?;
    // utiliser me.user_id pour caissier_id / log_audit — ne plus accepter caissier_id du client
    ...
}
```

Côté frontend, conserver le jeton **en mémoire** (et non en `localStorage`) et l'injecter dans le wrapper `invoke`.

### [CRITIQUE] C-7 — Compte `admin/admin` recréé automatiquement s'il est renommé ou supprimé

> **Statut : corrigé** sur `claude/hopeful-clarke-4uflms`.
> - Le compte `admin/admin` n'est créé que si la table `utilisateurs` est vide, marqué `must_change_password`.
> - Toute connexion avec un mot de passe faible (« admin », identique au login, moins de 8 caractères) active ce marqueur, y compris sur les bases existantes.
> - L'application bloque toutes les pages derrière un écran de changement de mot de passe (commande `change_password`).
> - Il est impossible de supprimer ou rétrograder le dernier administrateur.
> - L'indication « admin / admin » de l'écran de connexion n'apparaît plus qu'en développement.
> - Corrige aussi la connexion, qui contournait `AuthContext` (renvoi vers /login jusqu'au rechargement, permissions non chargées).
> - Le marqueur reste contournable via `localStorage` tant que C-5 n'est pas traité.

**Fichier** : `src-tauri/src/db.rs:674-685`
**Risque** : au démarrage, s'il n'existe aucun utilisateur dont le **login** vaut `admin`, un compte `admin` / mot de passe `admin` est créé. Un gérant qui renomme le compte administrateur par sécurité se retrouve donc avec un nouvel admin/admin au redémarrage suivant. Rien n'impose non plus de changer le mot de passe initial.
**Fix** :

```rust
let nb_users: i64 = conn.query_row("SELECT COUNT(*) FROM utilisateurs", [], |r| r.get(0))?;
if nb_users == 0 {
    let _ = conn.execute("ALTER TABLE utilisateurs ADD COLUMN must_change_password INTEGER DEFAULT 0", []);
    conn.execute(
        "INSERT INTO utilisateurs (login, password_hash, nom, role, must_change_password) VALUES ('admin', ?1, 'Administrateur', 'admin', 1)",
        params![hash_password("admin")],
    )?;
}
// login : si must_change_password = 1, renvoyer un statut qui force l'écran de changement de mot de passe.
```

### [MAJEUR] M-7 — XSS dans le ticket HTML, escaladable en accès IPC complet

> **Statut : corrigé** sur `claude/hopeful-clarke-4uflms`. Toutes les valeurs texte du ticket HTML sont échappées (`esc`), le logo n'est inséré que s'il est du base64 valide (HTML et PDF). En production, la CSP n'autorise plus `'unsafe-inline'` pour les scripts et ajoute `object-src 'none'`, `base-uri`, `form-action` et `frame-ancestors`. Le développement garde une `devCsp` permissive pour Vite. Couvert par 3 tests Vitest et vérifié sur un build release (l'application s'affiche normalement).

**Fichier** : `src/lib/receipt.ts:50,105-114,158`, `src-tauri/tauri.conf.json:26`
**Risque** :
- `generateReceiptHTML` interpole sans échappement `item.designation`, `shopName`, `shopAddress`, `receiptHeader`, etc.
- Le résultat est injecté via `document.write` dans une fenêtre `about:blank`, qui hérite de l'origine et de la CSP de l'app.
- La CSP autorise `script-src 'unsafe-inline'`, donc un attribut du type `<img src=x onerror=...>` s'exécute.
- Une désignation piégée, importée par exemple d'un CSV fournisseur (`import_articles_csv` ne filtre rien), exécute alors du JS avec accès à `__TAURI_INTERNALS__.invoke`. Combiné à C-5, cela donne un accès total à la base.

**Fix** :

```ts
const esc = (s: unknown) => String(s ?? "").replace(/[&<>"']/g, (c) =>
  ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]!))
// ...
<td>${esc(item.designation)}${item.remise_ligne ? ` (-${esc(item.remise_ligne)}%)` : ""}</td>
<div class="center header">${esc(data.shopName)}</div>
```

Retirer ensuite `'unsafe-inline'` de `script-src` : Vite ne génère pas de script inline en build. Il faut aussi valider `logoBase64` avec `/^[A-Za-z0-9+/=]+$/`.

### [MAJEUR] M-8 — Injection de commande Windows via `printer_name`

> **Statut : corrigé** sur `claude/hopeful-clarke-4uflms`. `valider_imprimante` n'accepte qu'un nom simple (lettres, chiffres, espace, `-_.`), un partage `\\hôte\partage` ou un périphérique `/dev/(usb/)lpN|ttyUSBN|ttyACMN|ttySN`. Plus aucun shell : écriture directe sur le partage ou le périphérique, ou `lp` avec arguments séparés et `--`. Le nom est aussi validé à l'enregistrement des paramètres. Couvert par 2 tests Rust, dont 14 tentatives d'injection.

**Fichier** : `src-tauri/src/commands/print.rs:44-54`
**Risque** : `printer_name` provient des settings, modifiables par n'importe quel appelant IPC (C-5). Il est concaténé dans une chaîne passée à `cmd /c`, donc une valeur comme `POS" & powershell -enc ... & "` exécute du code arbitraire sur le poste de caisse.
**Fix** : ne jamais passer par un shell. Valider le nom et écrire directement sur le partage d'impression.

```rust
if !printer_name.chars().all(|c| c.is_alphanumeric() || " -_.\\".contains(c)) {
    return Err("Nom d'imprimante invalide".into());
}
let printer_path = if printer_name.starts_with(r"\\") { printer_name.clone() } else { format!(r"\\localhost\{}", printer_name) };
std::fs::write(&printer_path, &bytes).map_err(|e| format!("Erreur impression: {e}"))?;
```

### [MAJEUR] M-9 — PIN : brute-force illimité, collisions, coût O(n × Argon2)

**Fichier** : `src-tauri/src/commands/auth.rs:47-67`, `src/components/IdleLock.tsx:62`
**Risque** :
- `login_pin` n'applique aucune limite de tentatives. Un PIN à 4 chiffres, soit 10 000 possibilités, se brute-force en quelques minutes par IPC.
- `set_user_pin` ne vérifie pas l'unicité : deux utilisateurs avec le même PIN, et le premier trouvé gagne. Un caissier peut ainsi déverrouiller sous l'identité du gérant.
- Chaque tentative calcule un hash Argon2 par utilisateur ayant un PIN, sur le **thread principal** (voir M-15).
- Après `login_pin`, `loginAs` ne recharge pas les permissions : le nouvel utilisateur hérite de la `PermissionsMap` du précédent (`AuthContext.tsx:110-113`).

**Fix** :
- Exiger `login` + PIN (et non le PIN seul), ou un PIN de 6 chiffres au minimum.
- Compter les échecs par utilisateur, avec verrouillage de 5 minutes après 5 échecs.
- Refuser un PIN déjà utilisé par un autre utilisateur.
- Dans `loginAs`, appeler `loadPermissions(userData.role)`.

```rust
// table utilisateurs : ADD COLUMN pin_echecs INTEGER DEFAULT 0, ADD COLUMN pin_bloque_jusqua TEXT
pub fn login_pin(db: State<DbState>, login: String, pin: String) -> Result<Option<Utilisateur>, String> {
    // SELECT ... WHERE login = ?1 ; si pin_bloque_jusqua > now → Err("Compte verrouillé")
    // verify_password ; si échec → pin_echecs += 1, bloquer si >= 5 ; si succès → pin_echecs = 0
}
```

### [MINEUR] m-1 — Énumération des logins par timing ; comparaison SHA-256 legacy non constante

**Fichier** : `src-tauri/src/commands/auth.rs:13-23`, `src-tauri/src/db.rs:703-707`
**Risque** : un login inexistant répond immédiatement (pas de calcul Argon2) alors qu'un login existant prend environ 50 ms. La comparaison des hash SHA-256 legacy (non salés) utilise `==`.
**Fix** : pour un login inconnu, vérifier contre un hash Argon2 factice précalculé. Utiliser `subtle::ConstantTimeEq` pour le legacy, puis forcer une réinitialisation des comptes encore en SHA-256 au bout de N jours.

### [MINEUR] m-2 — Deux mécanismes de verrouillage incohérents

**Fichier** : `src/context/AuthContext.tsx:27` (15 min codés en dur) vs `src/components/IdleLock.tsx:19` (`idle_timeout` des settings)
**Fix** : supprimer `AUTO_LOCK_MS` et faire expirer la session côté Rust (C-5) selon `idle_timeout`.

---

## Couche 2 — Intégrité des données & Transactions

### [CRITIQUE] C-3 — Numérotation : toutes les ventes échouent à partir du 1er janvier 2027

> **Statut : corrigé** sur `claude/hopeful-clarke-4uflms`. Nouvelle table `numerotation_v2` à clé `(ntype, annee)` alimentée depuis l'ancienne table au démarrage, helper unique `next_numero_document` (incrément atomique `RETURNING`) utilisé par `create_vente` et `convert_document`, type de document inconnu refusé, index unique sur `ventes.numero_facture` (S-4). Couvert par 7 tests Rust.

**Fichier** : `src-tauri/src/commands/ventes.rs:90-106` et `:446-462`, `src-tauri/src/db.rs:631-642`
**Risque** : `numerotation.ntype` est `UNIQUE`, mais la ligne est filtrée par `(ntype, annee)`. Au changement d'année :
- l'`INSERT ... ON CONFLICT(ntype)` ne fait rien, car la ligne 2026 existe ;
- l'`UPDATE ... WHERE annee = 2027` touche 0 ligne ;
- le `SELECT ... WHERE annee = 2027` renvoie `QueryReturnedNoRows`, donc `create_vente` renvoie `Err`.

Combiné à C-1, le caissier voit en plus une « vente réussie » fictive. Reproduction : `2026 rows updated 1 [(1,)]` puis `2027 rows updated 0 []`.
**Fix** : clé composite `(ntype, annee)` et incrément atomique, dans un helper partagé.

```rust
// migration
conn.execute_batch("
  CREATE TABLE IF NOT EXISTS numerotation_v2 (
    ntype TEXT NOT NULL, annee INTEGER NOT NULL, prefixe TEXT NOT NULL,
    dernier_numero INTEGER NOT NULL DEFAULT 0, PRIMARY KEY (ntype, annee));
  INSERT OR IGNORE INTO numerotation_v2 SELECT ntype, annee, prefixe, dernier_numero FROM numerotation;
")?;

// commands/mod.rs
pub(crate) fn next_numero(tx: &Connection, dtype: &str) -> Result<String, String> {
    let prefixe = match dtype { "devis" => "DE", "commande" => "CO", "bl" => "BL", "avoir" => "AV", _ => "FA" };
    let annee: i64 = chrono::Local::now().format("%Y").to_string().parse().map_err(|e: std::num::ParseIntError| e.to_string())?;
    let n: i64 = tx.query_row(
        "INSERT INTO numerotation_v2 (ntype, annee, prefixe, dernier_numero) VALUES (?1, ?2, ?3, 1)
         ON CONFLICT(ntype, annee) DO UPDATE SET dernier_numero = dernier_numero + 1
         RETURNING dernier_numero",
        params![format!("{dtype}_client"), annee, prefixe], |r| r.get(0),
    ).map_err(|e| e.to_string())?;
    Ok(format!("{prefixe}-{annee}-{n:05}"))
}
```

Ajouter également `CREATE UNIQUE INDEX idx_ventes_numero ON ventes(numero_facture)` (voir S-4). Les transactions SQLite sont sérialisées par le Mutex, ce qui garantit l'unicité sous concurrence une fois la clé corrigée.

### [CRITIQUE] C-4 — Clôture de session : l'espèces attendu ignore toutes les ventes

> **Statut : corrigé** sur `claude/hopeful-clarke-4uflms`. Nouvelle table `vente_paiements` (mode, montant, session) alimentée par `create_vente` pour les factures et BL. Le POS envoie toujours la ventilation `{mode, montant}`, monnaie rendue déduite. `close_session` calcule l'espèces attendu à partir des paiements espèces de la session hors ventes annulées, et propage les erreurs SQL. Le rapport X ventile par mode réel. Les ventes des sessions ouvertes au moment de la mise à jour sont reprises. Corrige aussi le paiement fractionné à crédit : clé `amount` au lieu de `montant`, donc plafond contourné et dette non enregistrée. Couvert par 7 tests Rust et 4 tests Vitest.

**Fichier** : `src-tauri/src/commands/sessions.rs:75-79`
**Risque** :
- La requête filtre `paiements.ptype`, alors que la colonne s'appelle `type`. L'erreur SQL `no such column: ptype` (reproduite) est avalée par `unwrap_or(0.0)`.
- Même corrigée, la table `paiements` ne contient que les **règlements de crédit client**, pas les ventes au comptant.
- Les journaux « split » sont insérés avec `jtype='encaissement'` et **sans `session_id`** (`ventes.rs:188-191`), donc ignorés eux aussi.

Résultat : `ecart` = total des ventes espèces de la session. Chaque Z de caisse est faux, et un vol réel devient indétectable dans le bruit.
**Fix** : enregistrer les encaissements par mode dans une table dédiée, puis calculer à partir de la session.

```rust
// db.rs
CREATE TABLE IF NOT EXISTS vente_paiements (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  vente_id INTEGER NOT NULL REFERENCES ventes(id),
  session_id INTEGER REFERENCES sessions_caisse(id),
  mode TEXT NOT NULL CHECK (mode IN ('especes','cb','cheque','virement','credit')),
  montant_centimes INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_vente_paiements_session ON vente_paiements(session_id, mode);

// create_vente : une ligne par split (ou une seule ligne si pas de split), somme == net TTC sinon Err.
// close_session :
let ventes_especes: i64 = tx.query_row(
    "SELECT COALESCE(SUM(vp.montant_centimes),0) FROM vente_paiements vp
     JOIN ventes v ON v.id = vp.vente_id
     WHERE vp.session_id = ?1 AND vp.mode = 'especes' AND v.statut = 'validee'",
    params![session_id], |r| r.get(0)).map_err(|e| e.to_string())?;
```

### [CRITIQUE] C-8 — Montants HT/TTC mélangés : CA, caisse et TVA incohérents

> **Statut : corrigé** sur `claude/hopeful-clarke-4uflms` (couvre aussi une partie de M-2).
> - `create_vente` relit prix et TVA dans le catalogue (public ou grossiste), valide remises et quantités, et calcule par ligne (arrondi au centime) : `total_ligne` TTC après remise ligne, `montant_ht`/`montant_tva` nets après remise document.
> - Le document stocke `montant_total` (TTC brut), `montant_remise`, `montant_ht` et `montant_tva`. `montant_total - montant_remise` vaut donc partout le net TTC.
> - Une facture ou un BL est refusé si les paiements ne totalisent pas le net recalculé.
> - Les points fidélité deviennent un mode de paiement `fidelite`, contrôlé contre le solde du client ; les points gagnés sont calculés côté serveur.
> - Le POS utilise le même calcul (`src/lib/totaux.ts`), avec un test de parité au centime avec `commands/calcul.rs`. Les remises ligne étaient affichées mais non déduites du net.
> - Tickets et PDF : ventilation TVA par taux sur les montants nets stockés.
> - Rapports : TVA collectée et marge sur les montants HT/TVA stockés.
> - Les ventes existantes sont recalculées une fois au démarrage.
> - Couvert par 16 tests Rust et 6 tests Vitest ; migration vérifiée en lançant l'application.

**Fichier** : `src-tauri/src/commands/ventes.rs:38-46,145-146`, `src/pages/POS.tsx:131-151`, `commands/caisses.rs`, `commands/rapports.rs`, `commands/stats.rs`
**Risque** :
- Le POS traite `prix_vente` comme **HT** et ajoute la TVA (`totalTTC = subtotal + totalTVA`). La remise globale est calculée sur le **TTC**.
- Le backend stocke `ventes.montant_total = Σ qte × pu`, soit du **HT sans remise ligne**, alors que `vente_articles.total_ligne` est **TTC après remise ligne**.
- Tous les rapports calculent `montant_total - montant_remise`, c'est-à-dire du HT moins une remise TTC.
- `close_caisse` et `get_tresorerie` comparent ce montant aux espèces réellement encaissées en TTC : l'écart systématique vaut environ la TVA (20 %).
- Les remises ligne (`remise_ligne`) sont ignorées dans `subtotal` côté POS **et** dans `montant_total` côté Rust.
- La remise fidélité n'est enregistrée nulle part.

**Fix** : recalculer **côté serveur** à partir des prix catalogue et persister HT, TVA et TTC explicitement.

```rust
// Pour chaque ligne : lire prix_vente/prix_grossiste/tva depuis `articles` (ne jamais faire confiance au pu client,
// sauf dérogation tracée avec permission "ventes"/"modifier_prix").
let ht_ligne  = round2(qte * pu_ht * (1.0 - remise_ligne / 100.0));
let tva_ligne = round2(ht_ligne * tva / 100.0);
total_ht += ht_ligne; total_tva += tva_ligne;
// ventes : ADD COLUMN montant_ht, montant_tva, montant_ttc (ou en centimes, cf. M-1)
// montant_remise appliquée sur le TTC → net_a_payer = montant_ttc - remise - remise_fidelite
```

Tous les rapports doivent ensuite lire `montant_ttc - montant_remise` (ou `montant_ht` pour le CA fiscal) de façon cohérente.

### [MAJEUR] M-1 — Montants en `f64`

> **Statut : corrigé** sur `claude/hopeful-clarke-4uflms`, avec une approche différente du stockage en `INTEGER`. Invariant retenu : tout montant stocké (totaux, lignes, paiements, crédit, caisse, chèques, journal) est un nombre exact de centimes, et tout calcul sur des montants se fait en centimes entiers.
>
> - **Rust** (`calcul.rs`) : `vers_centimes` / `en_dh` / `somme_dh`. Les totaux de document, l'attendu et l'écart de clôture, le plafond de crédit, les paiements et les achats sont calculés en `i64`. L'arrondi se fait au demi-centime loin de zéro, avec une correction de la représentation binaire (`1,005` → `1,01`, `3 × 8,335` → `25,01`).
> - **Montants saisis** (paiement client, chèque, journal de caisse, fonds de caisse, espèces déclarées, plafond de crédit) : validés par `montant_saisi` / `montant_positif` (NaN et infini refusés, négatifs refusés quand ils n'ont pas de sens), puis arrondis.
> - **SQL** : les 24 agrégats monétaires (CA, recettes, TVA, marge, crédit, espèces de session) additionnent des centimes (`SUM(ROUND(x * 100)) / 100.0`). Ils sont donc exacts quel que soit le nombre de lignes.
> - **Migration v2** : arrondit au centime les montants existants des 25 colonnes monétaires. Le nombre de valeurs corrigées est écrit dans le journal.
> - **Prix unitaires** : ils gardent leur précision, car un prix HT à 4 décimales est nécessaire pour obtenir un TTC rond. C'est le montant de chaque ligne qui est arrondi, conformément à la cohérence exigée par la DGI.
> - **Frontend** : `versCentimes` / `sommeDH` suivent la même règle. Ils sont utilisés au POS et dans les totaux affichés (ventes, journal, tableau de bord, caisses).
>   - Bug réel corrigé au passage : trois règlements de 33,33 + 33,33 + 33,34 étaient refusés au POS (« Montant insuffisant », car la somme flottante vaut 99,99999…).
>
> Le stockage n'a pas été basculé en `INTEGER`. Cela aurait changé le contrat IPC et 25 colonnes à la fois, pour une garantie identique dès lors que l'invariant ci-dessus est respecté.
>
> Couvert par des tests Rust et Vitest identiques des deux côtés. Un test de clôture après 300 ventes à 0,10 DH vérifie un écart exactement nul (il échouait avec l'ancien `SUM`). La migration a été vérifiée sur une base réelle dans l'application.

**Fichier** : toutes les colonnes `REAL` monétaires ; `create_vente`, `close_session`, `add_paiement`
**Risque** :
- Accumulation d'erreurs d'arrondi (0,1 + 0,2 ≠ 0,3) dans les SUM, les écarts de caisse et le crédit client.
- Pas de règle d'arrondi à 2 décimales par ligne, alors que la DGI exige la cohérence du total TTC avec Σ lignes.

**Fix** : stocker les montants en **centimes (`INTEGER`)**. Pour les quantités (poids), garder `REAL` ou utiliser des millièmes en `INTEGER`. À court terme, arrondir chaque ligne à 2 décimales avant de sommer.

```rust
fn round2(x: f64) -> f64 { (x * 100.0).round() / 100.0 }
// cible : i64 centimes + rust_decimal pour les calculs de TVA
```

### [MAJEUR] M-2 — Prix, remises, points et splits entièrement fournis par le client

**Fichier** : `src-tauri/src/commands/ventes.rs:39-43,60-66,116-134,139-146,182-194`
**Risque** :
- `prix_unitaire`, `tva`, `remise_ligne`, `montant_remise`, `points_utilises`/`points_gagnes` et les montants de split sont acceptés tels quels.
- Aucun contrôle que Σ splits = net à payer, ni que le client possède les points qu'il dépense (`MAX(0, …)` masque le dépassement).
- Un appel IPC peut vendre à 0,01 DH, s'attribuer des points arbitraires ou déclarer 100 % « cb » pour une vente payée en espèces.

**Fix** :
- Recalculer les prix côté serveur (C-8).
- Calculer `points_gagnes` côté serveur à partir des settings fidélité.
- Vérifier `points_utilises <= points_fidelite`, sinon `Err`.
- Vérifier `|Σ splits − net| < 0,01`, sinon `Err`.
- Plafonner les remises selon le rôle (permission `ventes/remise`).

### [MAJEUR] M-3 — Crédit client : double comptabilisation et annulation non reversée

> **Statut : corrigé** sur `claude/hopeful-clarke-4uflms`.
> - `credit_porte` calcule le crédit porté par un document : paiements `credit` enregistrés, sinon mode `credit`. Une facture issue d'un BL hérite du crédit du BL au lieu de le recompter.
> - La conversion devis → facture/BL à crédit vérifie le plafond. Un avoir déduit au plus le crédit porté par sa source.
> - `annuler_vente` refuse les documents déjà convertis (émettre un avoir), reverse le crédit client (y compris l'annulation d'un avoir) et les points fidélité gagnés ou dépensés, et enregistre un motif optionnel saisi dans l'écran Ventes.
> - Couvert par 5 tests Rust.

**Fichier** : `src-tauri/src/commands/ventes.rs:82-84,206-284,546-554`
**Risque** :
- Un BL à crédit incrémente `credit_actuel` à sa création, puis la conversion BL → facture l'incrémente **une seconde fois** (`convert_document`, `target_type == "facture"`, sans tester `source_dtype`).
- La conversion devis → facture à crédit n'applique **aucun contrôle de plafond**.
- `annuler_vente` ne décrémente pas `credit_actuel`, ne reverse pas les points fidélité et ne touche ni à la session ni au journal.
- `annuler_vente` accepte une vente `convertie` : annuler une facture déjà couverte par un avoir remet le stock **deux fois**.

**Fix** :

```rust
// convert_document
let credit_deja_compte = matches!(source_dtype.as_str(), "facture" | "bl");
if mode_paiement == "credit" && matches!(target_type.as_str(), "facture" | "bl") && !credit_deja_compte {
    verifier_plafond(&tx, cid, net_amount)?;
    tx.execute("UPDATE clients SET credit_actuel = credit_actuel + ?1 WHERE id = ?2", params![net_amount, cid])?;
}
// annuler_vente
if statut != "validee" { return Err(format!("Vente au statut '{statut}' : annulation impossible (émettre un avoir)")); }
if mode_paiement == "credit" && stock_was_deducted { /* credit_actuel -= net */ }
// reverser mouvements_fidelite (gain → dépense, dépense → gain), exiger un motif et l'utilisateur de session
```

### [MAJEUR] M-4 — Stock négatif non contrôlé ; lots et variantes hors du stock magasin

> **Statut : corrigé** sur `claude/hopeful-clarke-4uflms`.
> - Nouveau module `commands/mouvements.rs`, utilisé par la vente, l'annulation et la conversion (la logique variante/kit/article, dupliquée trois fois, n'existe plus qu'une fois).
> - Stock négatif refusé par défaut, par magasin, pour les ventes (factures/BL, composants de kits), conversions devis → facture/BL, transferts, ajustements manuels et mises au rebut de lots. Paramètre « Autoriser la vente en stock négatif » dans Paramètres.
> - Variantes : stock par magasin (`article_variante_stocks`), `stock_dedie` devient le total. Le stock existant est repris dans le magasin par défaut.
> - Lots : consommation FEFO à la vente (lots périmés ignorés), tracée dans `vente_lots`. Annulation, avoir, annulation d'avoir et facture issue d'un BL restituent ou reprennent exactement les mêmes lots.
> - Transferts : source ≠ destination, magasins et articles existants, quantités > 0, stock source vérifié à la validation.
> - Couvert par 10 tests Rust.

**Fichier** : `src-tauri/src/commands/mod.rs:8-19`, `ventes.rs:152-179`, `variantes.rs`, `lots.rs`, `magasins.rs:113-142`
**Risque** :
- `adjust_article_stock` accepte n'importe quel delta : ventes, transferts et démontages de kits peuvent passer le stock sous zéro sans alerte.
- Trois stocks coexistent :
  - `article_stocks` par magasin, source de vérité ;
  - `articles.stock`, cache ;
  - `article_variantes.stock_dedie`, sans magasin, jamais agrégé.
- Une vente de variante ne touche pas `article_stocks`.
- Une vente d'article suivi par lot ne décrémente aucun lot, donc pas de FEFO et des quantités de lots fausses.
- `create_transfert` n'interdit ni `source_id == dest_id`, ni une quantité ≤ 0, ni une quantité supérieure au disponible.

**Fix** :

```rust
pub(crate) fn adjust_article_stock(conn: &Connection, article_id: i64, magasin_id: i64, delta: f64, autoriser_negatif: bool) -> Result<(), String> {
    if delta < 0.0 && !autoriser_negatif {
        let dispo: f64 = conn.query_row(
            "SELECT COALESCE(quantite,0) FROM article_stocks WHERE article_id=?1 AND magasin_id=?2",
            params![article_id, magasin_id], |r| r.get(0)).unwrap_or(0.0);
        if dispo + delta < -1e-9 { return Err(format!("Stock insuffisant (article {article_id}, dispo {dispo})")); }
    }
    ...
}
```

Rendre le stock négatif configurable (setting `autoriser_stock_negatif`). Rattacher les variantes au magasin (`article_variante_stocks(variante_id, magasin_id, quantite)`) et décrémenter les lots en FEFO dans `create_vente`.

### [MAJEUR] M-5 — Inventaire : les mouvements survenus pendant le comptage sont écrasés

> **Statut : corrigé** sur `claude/hopeful-clarke-4uflms`.
> - Au comptage d'une ligne, le stock théorique est relevé à cet instant et l'écart calculé par rapport à lui.
> - La validation applique l'écart (`adjust_article_stock`) au lieu d'écraser le stock : les ventes et mouvements postérieurs au comptage sont conservés.
> - Lignes verrouillées après validation, quantité comptée ≥ 0, un seul inventaire en cours par magasin, création transactionnelle.
> - Couvert par 3 tests Rust.

**Fichier** : `src-tauri/src/commands/inventaire.rs:147-158`
**Risque** :
- `valider_inventaire` fait `SET quantite = stock_compte`. Toute vente, réception ou transfert survenu entre `create_inventaire` et la validation est perdu, puisque le mouvement est enregistré mais le stock écrasé.
- `update_inventaire_ligne` reste possible après validation.
- `create_inventaire` n'est pas transactionnel.

**Fix** : appliquer l'**écart**, et non la valeur absolue, et verrouiller les lignes.

```rust
adjust_article_stock(&tx, *article_id, magasin_id, *ecart, true)?;
// update_inventaire_ligne : vérifier que l'inventaire parent est 'en_cours'
// create_inventaire : conn.transaction()
```

### [MAJEUR] M-6 — Rapports : devis et documents convertis comptés dans le CA

> **Statut : corrigé** sur `claude/hopeful-clarke-4uflms`.
> - Filtre unique `filtre_ca!()` : factures et avoirs, et BL non convertis en facture, hors documents annulés. Les devis et commandes sont exclus, et un BL converti n'est plus compté deux fois. Les quantités des avoirs sont soustraites (`quantite_signee!`).
> - Le filtre est appliqué au rapport X, au rapport détaillé, au tableau de bord et à l'écran Caisses/Trésorerie.
> - Caisses/Trésorerie : les recettes viennent des paiements enregistrés (les paiements d'un BL sont rattachés à sa facture, `carte` compté avec `cb`), sinon du mode de paiement du document. Les remboursements d'avoirs sont déduits.
> - Même règle côté frontend (`compteDansCA`) pour le total de l'écran Ventes et le graphique du tableau de bord.
> - Couvert par 2 tests Rust et 2 tests Vitest.

**Fichier** : `src-tauri/src/commands/rapports.rs:15-49,76-180`, `stats.rs:8-24`
**Risque** :
- Le filtre `statut != 'annulee'` inclut `dtype IN ('devis','commande')`.
- Il inclut aussi le document source `convertie` **et** la facture issue de la conversion : un devis converti est compté deux fois.
- Le rapport X, le tableau de bord et le rapport détaillé sont gonflés.
- La marge utilise le `prix_achat` **actuel** au lieu du coût au moment de la vente.

**Fix** : filtre uniforme `v.statut = 'validee' AND v.dtype IN ('facture','bl','avoir')`, avec les avoirs en négatif. Stocker `prix_achat_unitaire` dans `vente_articles` au moment de la vente.

---

## Couche 3 — Migrations & Schéma BDD

### [MAJEUR] S-1 — Migrations non versionnées et erreurs silencieuses

> **Statut : corrigé** sur `claude/hopeful-clarke-4uflms`.
> - Le schéma est versionné par `PRAGMA user_version`. L'initialisation existante devient la migration v1 (idempotente, elle met à niveau toute base existante).
> - Les migrations suivantes s'ajoutent à la liste `MIGRATIONS` ; chacune s'exécute dans une transaction avec la mise à jour de version, et n'est jamais modifiée après publication.
> - Une base créée par une version plus récente est refusée avec un message clair.
> - Les 32 `ALTER TABLE` passent par `ajouter_colonne`, qui vérifie la colonne et propage les vraies erreurs. Plus aucun `let _ =` ni `.ok()` dans l'initialisation.
> - Couvert par 5 tests Rust ; migration v0 → v1 vérifiée en lançant l'application sur une base existante.

**Fichier** : `src-tauri/src/db.rs:491-629`
**Risque** :
- 27 `let _ = conn.execute("ALTER TABLE …")` sont exécutés à chaque démarrage, et toute erreur est ignorée : disque plein, base verrouillée, faute de frappe.
- Aucun `PRAGMA user_version`, donc impossible de savoir dans quel état est une base client, ni de faire une migration de données (renommage, contrainte, backfill) de façon fiable.

**Fix** :

```rust
const MIGRATIONS: &[&str] = &[
    /* 1 */ "ALTER TABLE clients ADD COLUMN points_fidelite REAL DEFAULT 0;",
    /* … une entrée par évolution, jamais modifiée après release … */
    /* n */ "CREATE TABLE numerotation_v2 (...); INSERT INTO numerotation_v2 SELECT ...;",
];

fn migrate(conn: &mut Connection) -> rusqlite::Result<()> {
    let v: usize = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
    for (i, sql) in MIGRATIONS.iter().enumerate().skip(v) {
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", (i + 1) as i64)?;
        tx.commit()?;
    }
    Ok(())
}
```

Pour les bases existantes, une migration « 0 » idempotente doit détecter les colonnes présentes via `PRAGMA table_info` avant de fixer `user_version = 27`.

### [MAJEUR] S-2 — Colonnes ajoutées sans clé étrangère ni contrainte

> **Statut : corrigé** sur `claude/hopeful-clarke-4uflms`. La migration v3 reconstruit 7 tables selon la procédure SQLite. Les clés étrangères sont désactivées pendant la migration puis restaurées, et `foreign_key_check` s'exécute avant le commit. Les index et la séquence `AUTOINCREMENT` sont conservés : un identifiant supprimé n'est jamais réutilisé.
>
> - **Clés étrangères ajoutées :**
>   - `ventes` : `session_id`, `magasin_id`, `source_vente_id` ;
>   - `vente_articles.variante_id` ;
>   - `journal_caisse.session_id` ;
>   - `sessions_caisse.magasin_id` ;
>   - `mouvements_stock.magasin_id`.
> - **`CHECK` sur les énumérations :**
>   - `ventes` : `statut`, `dtype`, `mode_paiement` ;
>   - `journal_caisse.jtype` et `mouvements_stock.mtype` ;
>   - `utilisateurs.role` ;
>   - `cheques` : `ctype`, `statut` ;
>   - `sessions_caisse.statut` ;
>   - `vente_articles.prix_type`.
> - **`CHECK` sur les valeurs :**
>   - quantité de ligne > 0, TVA et remise de ligne entre 0 et 100 ;
>   - total ≥ 0 sauf pour un avoir ;
>   - fonds de caisse et montant de chèque ≥ 0 ;
>   - points utilisés ≥ 0.
> - **Données existantes :**
>   - Elles sont d'abord normalisées : casse et espaces, `'especes+cb'` → `mixte`, `fermee` → `cloturee` pour une session, `vente` / `achat` → `sortie` / `entree` pour les mouvements.
>   - Les références orphelines des nouvelles colonnes sont remises à `NULL`, et leur nombre est journalisé.
>   - Si une valeur reste inconnue, la migration est entièrement annulée avec un message qui nomme la table, la colonne et les valeurs à corriger.
> - **Code :**
>   - `create_vente` enregistre le mode unique ou `mixte`. Le POS envoyait `"especes+cb"`, qui aurait violé la contrainte ; le détail reste dans `vente_paiements`.
>   - Rôle, type de journal, type et statut de chèque sont validés avec un message clair.
>   - La suppression d'un magasin, d'un article, d'un client, d'une catégorie, d'un fournisseur, d'un utilisateur ou d'une variante encore utilisé renvoie « Suppression impossible : … est utilisé par d'autres enregistrements ». `delete_article_variante` s'exécute maintenant dans une transaction.
>
> Couvert par des tests Rust :
> - migration d'une base v2 avec données fautives, orphelines et une séquence plus haute que le plus grand id ;
> - 12 insertions invalides refusées ;
> - migration annulée sur une valeur inconnue, base laissée intacte.
>
> Vérifié dans l'application (xvfb) : migration v2 → v3 d'une base existante, ouverture de session, puis vente en paiement fractionné espèces + carte, enregistrée en `mixte` avec ses deux règlements.

**Fichier** : `src-tauri/src/db.rs:511-531`
**Risque** : `ventes.session_id`, `ventes.magasin_id`, `ventes.source_vente_id`, `vente_articles.variante_id`, `journal_caisse.session_id` et `sessions_caisse.magasin_id` n'ont aucune FK, alors que `foreign_keys=ON`. `delete_magasin` peut ainsi laisser des ventes orphelines.

Aucun `CHECK` sur les énumérations (`statut`, `dtype`, `mtype`, `jtype`, `role`, `ctype`) ni sur les quantités ou montants. Une faute de frappe côté client, par exemple `'especes '` ou `'Especes'`, crée un mode de paiement fantôme qui disparaît des rapports.

**Fix** : reconstruire les tables concernées dans une migration versionnée (`CREATE TABLE ventes_new … ; INSERT … SELECT ; DROP ; ALTER RENAME`) en y ajoutant :

```sql
magasin_id INTEGER REFERENCES magasins(id),
session_id INTEGER REFERENCES sessions_caisse(id),
source_vente_id INTEGER REFERENCES ventes(id),
statut TEXT NOT NULL DEFAULT 'validee' CHECK (statut IN ('validee','annulee','convertie')),
dtype  TEXT NOT NULL DEFAULT 'facture' CHECK (dtype IN ('facture','bl','devis','commande','avoir')),
mode_paiement TEXT NOT NULL CHECK (mode_paiement IN ('especes','cb','cheque','virement','credit','mixte'))
```

### [MINEUR] S-3 — Index manquants

**Fichier** : `src-tauri/src/db.rs:588-607`
**Fix** :

```sql
CREATE INDEX IF NOT EXISTS idx_ventes_session      ON ventes(session_id);
CREATE INDEX IF NOT EXISTS idx_ventes_source       ON ventes(source_vente_id);
CREATE INDEX IF NOT EXISTS idx_ventes_statut_dtype ON ventes(statut, dtype, date);
CREATE INDEX IF NOT EXISTS idx_sessions_caissier   ON sessions_caisse(caissier_id, statut);
CREATE INDEX IF NOT EXISTS idx_mouvements_magasin  ON mouvements_stock(magasin_id, date);
CREATE INDEX IF NOT EXISTS idx_journal_session     ON journal_caisse(session_id);
CREATE INDEX IF NOT EXISTS idx_paiements_date      ON paiements(date);
CREATE INDEX IF NOT EXISTS idx_fidelite_client     ON mouvements_fidelite(client_id, date);
CREATE INDEX IF NOT EXISTS idx_article_stocks_mag  ON article_stocks(magasin_id);
```

### [MAJEUR] S-4 — Pas d'unicité sur les numéros de document ni sur les codes client

> **Statut : corrigé** sur `claude/hopeful-clarke-4uflms`. La migration v4 rend les deux contraintes obligatoires. Jusque-là, l'index sur `numero_facture` était ignoré, avec un simple avertissement, dès qu'il existait des doublons.
>
> - **Doublons existants :** le premier document garde son numéro, les suivants reçoivent le suffixe `-DOUBLON-<id>`. Les codes client sont d'abord nettoyés des espaces (un code vide devient `NULL`), puis les doublons reçoivent le suffixe `-<id>`. Chaque correction est journalisée et tracée dans l'audit (`correction_doublons`).
> - **Index uniques** créés sur `ventes(numero_facture)` et `clients(code)`.
> - **Compteurs de numérotation** resynchronisés sur le plus grand numéro existant, par type et par année. Si un compteur est en retard (restauration partielle, remise à zéro), `next_numero_document` le resynchronise au lieu de bloquer la vente ou d'attribuer un doublon.
> - **Codes client :**
>   - code saisi nettoyé des espaces ;
>   - message clair si le code existe déjà (« Le code client « X » est déjà utilisé ») ;
>   - code laissé vide : attribution automatique de `CLI-<id>`, que l'interface annonçait sans que rien ne le fasse, avec un suffixe en cas de collision.
> - **Bug trouvé en testant, et corrigé :** le formulaire Clients (et Fournisseurs) refusait un email vide (« Email invalide »). Il était donc impossible de créer un client sans email.
>
> Couvert par des tests Rust (migration avec doublons de numéros et de codes, compteur en retard, codes générés) et par un test Vitest (création sans email, qui échouait avant le correctif). Vérifié dans l'application : migration v3 → v4, vente numérotée `FA-2026-00003`, doublon de code refusé avec le bon message, code `CLI-00002` généré.

**Fichier** : `src-tauri/src/db.rs` (table `ventes`, `clients`)
**Risque** : rien n'empêche deux factures portant le même `numero_facture`, par exemple après une restauration partielle ou une remise à zéro manuelle du compteur. C'est une non-conformité DGI.
**Fix** : `CREATE UNIQUE INDEX idx_ventes_numero ON ventes(numero_facture) WHERE numero_facture IS NOT NULL;` et `CREATE UNIQUE INDEX idx_clients_code ON clients(code) WHERE code IS NOT NULL AND code != '';`.

### [MINEUR] S-5 — Pas de traçabilité created_at / updated_at / created_by

**Fichier** : tables `articles`, `clients`, `fournisseurs`, `categories`, `utilisateurs`, `magasins`, `settings`, `permissions`
**Fix** : `ALTER TABLE … ADD COLUMN created_at TEXT DEFAULT (datetime('now','localtime'))`, plus `updated_at` et `updated_by`, maintenus par le code des commandes `update_*` (l'utilisateur vient de la session, voir C-5).

### [MINEUR] M-12 — Migration du stock initial : stocks négatifs ou nuls perdus

> **Statut : corrigé** sur `claude/hopeful-clarke-4uflms` (`stock != 0`).

**Fichier** : `src-tauri/src/db.rs:617-621`
**Risque** : `WHERE stock > 0` ignore les articles à stock négatif. Au premier `adjust_article_stock`, `articles.stock` est recalculé depuis `article_stocks` et la dette de stock disparaît.
**Fix** : `WHERE stock != 0`.

---

## Couche 4 — Architecture Backend Rust

### [MAJEUR] M-15 — Commandes synchrones exécutées sur le thread principal

> **Statut : corrigé** sur `claude/hopeful-clarke-4uflms`.
>
> - **Hors du thread principal :** les 101 commandes sont déclarées `#[tauri::command(async)]`. Tauri les exécute alors sur son pool de threads au lieu du thread principal, sans changer leur signature. L'interface ne se fige donc plus pendant un hachage Argon2, un rapport, une impression ou une sauvegarde. Le test de contrôle de session échoue si une commande reste synchrone.
> - **Lectures parallèles :** `DbState` ajoute un pool de 3 connexions en lecture seule (`SQLITE_OPEN_READ_ONLY` + `query_only`) sur la même base WAL. Les 39 commandes qui ne font que lire l'utilisent via `db.lecture()` : listes, rapports, statistiques, trésorerie, journal, audit, sauvegarde et export. Elles ne sont plus bloquées par une écriture en cours et ne la bloquent pas. Les sauvegardes copient un instantané cohérent depuis un lecteur.
> - **Écritures :** elles restent sérialisées sur la connexion principale, avec `busy_timeout` à 5 s.
>
> Couvert par des tests Rust : lecteurs actifs pendant une écriture, écriture refusée sur un lecteur, restauration et sauvegarde avec des lecteurs ouverts. Vérifié dans l'application (xvfb) : connexion, tableau de bord et pages de liste.
>
> Au passage, un bug de filtre a été corrigé : `get_ventes`, `get_journal_caisse` et `get_mouvements_stock` comparaient la date de fin sans l'heure. Les opérations du dernier jour étaient donc exclues, et la page Ventes n'affichait pas les ventes du jour. La fonction commune `fin_de_journee` est maintenant utilisée partout.

**Fichier** : toutes les commandes (`pub fn`, non `async`)
**Risque** : dans Tauri v2, une commande non `async` s'exécute sur le thread principal. Argon2 (environ 50 à 100 ms par hash, multiplié par le nombre d'utilisateurs dans `login_pin`), les rapports sans `LIMIT` (`get_articles`, `get_rapport_detaille`, `compare_fournisseur_prices`, `get_transferts`) et les sauvegardes figent l'UI. Le Mutex global sérialise toutes les lectures derrière la moindre écriture.
**Fix** : déclarer les commandes `async` et exécuter le travail bloquant avec `tauri::async_runtime::spawn_blocking`. Ouvrir une connexion en lecture séparée (WAL autorise les lectures concurrentes), ou utiliser `r2d2_sqlite` avec un pool d'une connexion en écriture et N en lecture.

```rust
#[tauri::command]
pub async fn get_rapport_detaille(db: State<'_, DbState>, debut: Option<String>, fin: Option<String>) -> Result<serde_json::Value, String> {
    let conn = db.read_pool.clone();
    tauri::async_runtime::spawn_blocking(move || { let c = conn.get().map_err(|e| e.to_string())?; rapport_impl(&c, debut, fin) })
        .await.map_err(|e| e.to_string())?
}
```

### [MAJEUR] M-16 — Erreurs avalées (`unwrap_or`, `.ok()`, `let _ =`)

> **Statut : corrigé** sur `claude/hopeful-clarke-4uflms`.
>
> - **Audit :** `log_audit` renvoie un `Result` et ses 25 appels le propagent. Les 16 opérations qui écrivaient hors transaction avant l'audit passent désormais dans une transaction, si bien qu'un audit impossible annule l'opération : suppressions, permissions, paramètres, ouverture de session, utilisateurs, connexion, changement de mot de passe. Pour la restauration, l'audit fait partie de la chaîne protégée par le retour arrière. Les tentatives échouées (connexion, mot de passe) restent journalisées avant le refus.
> - **Requêtes SQL :** les agrégats de `rapports` et `stats` n'utilisent plus `unwrap_or(0)`, et les 12 `filter_map(Result::ok)` sur des lignes ont disparu. Une erreur SQL remonte au lieu d'afficher « 0 DH » ou une liste incomplète. Même chose pour les requêtes ponctuelles :
>   - le contrôle de double conversion (`already_converted`) ;
>   - la session et le magasin de la vente ;
>   - le numéro de facture annulé ;
>   - la session du journal de caisse ;
>   - les compteurs de `delete_magasin` et `open_session` ;
>   - les compteurs des migrations.
> - **« Introuvable » et « erreur » distingués :** les recherches qui pouvaient légitimement ne rien trouver passent à `.optional()?`. Cela concerne les noms avant suppression, les anciens prix d'un article et le nom de l'imprimante.
> - **Impression :** `print_ticket` et `print_receipt` vérifient le code de sortie de `lp`, PowerShell et `xdg-open`, et signalent l'échec (également écrit dans le journal).
> - **Entrées :** une ligne d'achat sans quantité ou sans prix valide est refusée au lieu de valoir 0. À l'import CSV, une valeur numérique illisible produit une erreur de ligne au lieu d'un 0 silencieux. Une empreinte de mot de passe illisible est journalisée.
> - **Restent volontairement :** les valeurs par défaut de paramètres ou de champs JSON optionnels, le listage des fichiers de sauvegarde et le nettoyage des fichiers temporaires.
>
> Couvert par des tests Rust : audit bloqué par un trigger qui annule une suppression, une modification d'utilisateur ou une clôture de caisse ; lignes d'achat invalides refusées. Vérifié dans l'application : connexion (auditée), tableau de bord et rapports.

**Fichier** : `sessions.rs:75-91`, `caisses.rs:59-81`, `rapports.rs` (partout), `stats.rs` (partout), `ventes.rs:96,127-133,406-410`, `mod.rs:24-28` (`log_audit`), `auth.rs:29-32`
**Risque** :
- `unwrap_or(0.0)` sur des agrégats financiers transforme une erreur SQL en « 0 DH ». C'est exactement le mécanisme qui masque C-4.
- `.ok()` sur la mise à jour des points fidélité fait qu'une vente est validée sans mouvement de points.
- `log_audit` ignore ses échecs : l'audit peut manquer silencieusement.
- `already_converted.unwrap_or(false)` peut autoriser une double conversion.

Liste acceptable : `ALTER TABLE ADD COLUMN` en attendant S-1, et `create_dir_all` sur un dossier existant. Tout le reste doit propager l'erreur.
**Fix** : remplacer par `.map_err(|e| e.to_string())?`. Faire renvoyer `Result` à `log_audit` et l'appeler avec `?` dans les transactions : si l'audit échoue, l'opération échoue.

### [MINEUR] M-17 — `serde_json::Value` en entrée et en sortie

**Fichier** : `create_vente(articles: Vec<serde_json::Value>, splits: …)`, `create_achat`, `create_transfert`, et plus de 40 commandes renvoyant `serde_json::Value`
**Risque** : aucun typage des lignes de vente. Une clé mal orthographiée devient `unwrap_or(0.0)`, soit une ligne à 0 DH ou une quantité nulle acceptée. Aucun contrat partagé avec `src/types/index.ts`.
**Fix** :

```rust
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LigneVenteInput { pub article_id: i64, pub variante_id: Option<i64>, pub quantite: f64,
    pub remise_ligne: Option<f64>, pub note: Option<String>, pub prix_type: Option<String> }
```

Générer ensuite les types TS avec `ts-rs` ou `specta` (et `tauri-specta` pour des bindings `invoke` typés, ce qui éliminerait aussi C-2).

### [MINEUR] M-18 — Code mort et doublons

**Fichier** : `src-tauri/src/db.rs:710-739`, `commands/caisses.rs` vs `commands/sessions.rs`, `ventes.rs` (bloc numérotation dupliqué, et bloc « stock kit/variante/simple » dupliqué 3 fois)
**Risque** :
- `authenticate()` est inutilisé.
- `caisses` et `sessions_caisse` sont deux systèmes de caisse parallèles aux calculs différents, avec des totaux divergents.
- La logique de sortie de stock est copiée trois fois, et c'est précisément là que M-3 et M-4 divergent.

**Fix** :
- Supprimer `authenticate`.
- Converger vers `sessions_caisse`.
- Extraire `fn mouvement_ligne(tx, article_id, variante_id, qte, sens, magasin_id, ref_type, ref_id)` et `next_numero` (C-3).

Toutes les commandes de `commands/` sont bien enregistrées dans `lib.rs` (97/97, vérifié).

### [MINEUR] M-19 — Requêtes sans pagination et troncature silencieuse

**Fichier** : `get_articles` (sans LIMIT), `get_transferts`, `compare_fournisseur_prices` (scan complet, filtre en Rust) ; `get_ventes`, `get_journal_caisse`, `get_mouvements_stock` (`LIMIT 200` sans indication)
**Risque** : un supermarché avec 20 000 références charge tout le catalogue à chaque ouverture du POS. Les historiques sont tronqués à 200 lignes sans que l'utilisateur le sache. Les filtres `date <= fin` excluent la journée de fin (`'2026-09-24 10:00' > '2026-09-24'`), alors que `rapports.rs` et `audit.rs` ajoutent bien `23:59:59`.
**Fix** : pagination `LIMIT ?/OFFSET ?` avec `total` renvoyé, filtre SQL `WHERE aa.article_id = ?` dans `compare_fournisseur_prices`, et `date < date(?, '+1 day')` partout.

---

## Couche 5 — Frontend React/TypeScript

### [CRITIQUE] C-1 — `invoke()` remplace toute erreur backend par une réponse mock

**Fichier** : `src/lib/tauri.ts:320-333`
**Risque** : le `try/catch` englobe **l'appel Tauri lui-même**. Toute erreur Rust est donc rattrapée, et si un mock existe pour la commande, une fausse réponse de succès est renvoyée. Exemples : plafond de crédit, erreur SQL, numérotation 2027, argument manquant.

Concrètement en production :
- `create_vente` en erreur renvoie `{id: 101, numero_facture: "FA-2026-00101"}`. Le ticket s'imprime et le panier est vidé, mais **rien n'est enregistré** : ni vente, ni stock, ni crédit.
- `login` en erreur fait passer par `mockData.login`, où les comptes `admin`, `manager` et `caissier` acceptent le mot de passe `admin`.
- Le bundle de production embarque les mocks (constaté dans `dist/assets/tauri-*.js`).

**Fix** :

```ts
// src/lib/tauri.ts
import { invoke as tauriInvoke, isTauri } from "@tauri-apps/api/core"

export async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (isTauri()) return tauriInvoke<T>(cmd, args)
  if (import.meta.env.DEV) {
    const { mockInvoke } = await import("./tauri.mock")
    return mockInvoke<T>(cmd, args)
  }
  throw new Error(`Application hors Tauri : commande "${cmd}" indisponible`)
}
```

Déplacer ensuite `mockData` dans `src/lib/tauri.mock.ts`. L'import dynamique sous `import.meta.env.DEV` est éliminé du build de production.

### [CRITIQUE] C-2 — Clés `snake_case` : échec des commandes, masqué par C-1

> **Statut : corrigé** sur `claude/hopeful-clarke-4uflms`. Les clés de premier niveau sont converties en camelCase dans `invoke()`, la restauration de sauvegarde demande désormais un chemin avec confirmation, et un test de contrat (`src/lib/tauri.test.ts`) vérifie chaque appel littéral contre les signatures Rust.

**Fichier** :

| Appel | Clés en cause | Conséquence en production |
|---|---|---|
| `src/hooks/useProducts.ts:46` `add_article` | `prix_achat`, `prix_vente` | clé requise absente → erreur → mock : **aucun article créé** |
| `src/pages/Settings.tsx:201` `update_settings` | `shop_name`, `default_tva` | clé requise absente : **ICE/IF/RC/Patente jamais enregistrés** |
| `src/pages/Settings.tsx:956` `import_database` | `path` absent | **la restauration ne fait rien** |
| `src/pages/Articles.tsx:87,99,134,146`, `POS.tsx:113,251`, `Stock.tsx:53,68` | `article_id`, `code_barre`… | variantes, kits et lots toujours vides ; ajout impossible |
| `src/hooks/useCheques.ts:32` `add_cheque` | `date_emission`… | chèque jamais enregistré |
| `src/pages/POS.tsx:412-413` `create_vente` | `points_utilises`, `points_gagnes` | reçus comme `None` : **fidélité inerte**, alors que la remise fidélité est accordée à l'écran |
| `src/hooks/useClients.ts:20,29` `add/update_client` | `credit_plafond` | `None` : **plafond de crédit jamais enregistré, donc crédit illimité** |
| `src/components/pos/CartPanel.tsx:120` `update_table_status` | `ticket_id` | ticket de table jamais lié |
| `src/pages/Achats.tsx:65` `create_achat` | `fournisseur_id` | achat enregistré **sans fournisseur** |

Tauri v2 convertit les noms d'arguments Rust en `camelCase` par défaut (vérifié dans `tauri-macros 2.5`, `wrapper.rs:50,461-462`) :
- clé requise absente : erreur `missing required key` ;
- clé `Option<T>` absente : `None` silencieux.

**Fix** : normaliser les clés de premier niveau dans le wrapper. Les objets imbriqués (lignes d'articles) restent en snake_case, car Rust les lit via `a["article_id"]`.

```ts
const toCamel = (k: string) => k.replace(/_([a-z])/g, (_, c: string) => c.toUpperCase())
const camelizeTopLevel = (args?: Record<string, unknown>) =>
  args && Object.fromEntries(Object.entries(args).map(([k, v]) => [toCamel(k), v]))

if (isTauri()) return tauriInvoke<T>(cmd, camelizeTopLevel(args))
```

Ajouter aussi un test de contrat Vitest qui parse les signatures `#[tauri::command]` des fichiers Rust et vérifie que chaque appel `invoke` fournit les clés requises. À terme, passer à `tauri-specta` (M-17).

### [MAJEUR] M-10 — Rôle et permissions de l'UI modifiables par l'utilisateur

**Fichier** : `src/context/AuthContext.tsx:58-62,110-113,119-128`, `src/routes/router.tsx`
**Risque** :
- `ProtectedRoute` ne vérifie que le rôle lu en `localStorage` (voir C-5).
- La table `permissions`, pourtant éditable par l'admin, n'est **pas** utilisée par le routeur. Retirer `ventes/voir` au rôle manager ne l'empêche donc pas d'ouvrir `/ventes`.
- `/dashboard` (CA, crédit total, top clients) est accessible aux caissiers.

**Fix** : `ProtectedRoute module="ventes" action="voir"`, qui s'appuie sur `hasModulePermission`. Recharger l'utilisateur et les permissions depuis le backend au démarrage (`whoami(token)`) au lieu du `localStorage`.

### [MINEUR] F-1 — Panier persisté en `localStorage`

**Fichier** : `src/store/cart.ts:188-191`
**Risque** : faible. Le panier ne contient ni données de paiement ni PII hormis un id client. En revanche :
- les tickets en attente (`heldCarts`) persistent entre utilisateurs du même poste ;
- `prix_unitaire` est modifiable (`setLinePrice`) sans contrôle de permission, et le serveur fait confiance à ce prix (M-2) ;
- `activeTableId` et `activeTableNom` sont déclarés deux fois dans l'interface.

**Fix** : préfixer la clé de persistance par l'id utilisateur ou vider au logout. Faire contrôler les prix côté serveur (M-2).

### [MINEUR] F-2 — Gestion d'erreurs et cache hétérogènes

**Fichier** : `src/hooks/*`, `src/pages/*`
**Risque** : 74 `useMutation` pour environ 27 `onError` dans les hooks, et 18 `staleTime` seulement. Certaines mutations définies dans les pages n'ont aucun retour d'erreur. Les requêtes lourdes (`get_articles`) sont refetchées à chaque focus.
**Fix** : handler global dans le `QueryClient` :

```ts
new QueryClient({
  defaultOptions: { queries: { staleTime: 30_000, refetchOnWindowFocus: false } },
  mutationCache: new MutationCache({ onError: (e, _v, _c, m) => { if (!m.options.onError) toast.error(String(e)) } }),
})
```

### [SUGGESTION] F-3 — Accessibilité et lint

**Fichier** : `src/pages/Inventaire.tsx:16`, `src/pages/AuditLog.tsx:10`
**Fix** :
- Corriger les 2 warnings oxlint (imports inutilisés).
- Ajouter `eslint-plugin-jsx-a11y` (ou les règles a11y d'oxlint) en CI.
- Vérifier le contraste des badges en mode sombre et les raccourcis clavier du POS (F-keys) avec `aria-keyshortcuts`.

---

## Couche 6 — Production Readiness

### [CRITIQUE] C-6 — Base de données dans le répertoire courant (`./data`)

> **Statut : corrigé** sur `claude/hopeful-clarke-4uflms`. Base dans `app_data_dir()` (`%APPDATA%\com.supercaisse.pos` sous Windows), sauvegardes dans son sous-dossier `backups`, exports et PDF dans `Documents\SuperCaisse`. Au premier lancement, une ancienne base `./data/supercaisse.db` (dossier courant ou dossier de l'exécutable) est copiée via l'API de sauvegarde SQLite, sans jamais écraser une base existante ; l'ancien fichier est conservé. Couvert par 3 tests Rust et vérifié en lançant l'application (xvfb).

**Fichier** : `src-tauri/src/lib.rs:127-136`, `commands/backup.rs:12,28`, `commands/print.rs:120`
**Risque** : `std::env::current_dir()/data/supercaisse.db`.
- Sous Windows, une app installée et lancée par raccourci a pour CWD `C:\Program Files\supercaisse`, qui n'est pas inscriptible : **panic au démarrage** (`expect("Failed to initialize database")`).
- Sinon, la base suit le CWD : une base différente selon la façon dont l'app est lancée, donc des « pertes » de données apparentes.
- Les sauvegardes et les PDF subissent le même problème.

**Fix** :

```rust
.setup(|app| {
    let dir = app.path().app_data_dir()?;           // %APPDATA%\com.supercaisse.pos
    std::fs::create_dir_all(&dir)?;
    let conn = init_db(dir.join("supercaisse.db").to_str().unwrap())
        .map_err(|e| format!("Initialisation BDD: {e}"))?;
    app.manage(DbState { conn: Arc::new(Mutex::new(conn)) });
    Ok(())
})
// backups : app.path().app_data_dir()?.join("backups") ; documents : app.path().document_dir()?
```

Prévoir une migration unique au premier lancement : si `./data/supercaisse.db` existe et que la nouvelle base n'existe pas, la copier.

### [MAJEUR] P-1 — `import_database` : aucune validation ni sauvegarde préalable

> **Statut : corrigé** sur `claude/hopeful-clarke-4uflms`. Avant tout écrasement, `import_database` (admin uniquement) contrôle le fichier : `integrity_check`, présence des tables clés, version de schéma inférieure ou égale à celle de l'application, au moins un administrateur actif. Il copie ensuite la base actuelle dans `backups/avant_restauration_*.db`, restaure via l'API de sauvegarde SQLite, puis applique les migrations. Si une étape échoue, la copie est remise en place et la base actuelle est conservée. Après une restauration réussie, l'action est tracée dans le journal d'audit, toutes les sessions sont fermées et l'utilisateur est déconnecté. Une sauvegarde automatique `auto_*.db` est faite au démarrage, au plus une par jour, avec rotation sur 30 fichiers. Les copies sont écrites en `journal_mode=DELETE`, donc en un seul fichier sans `-wal`. Côté interface, un bouton « Créer une sauvegarde » a été ajouté, et la boîte de restauration liste les sauvegardes et exports disponibles (`list_backups`) en plus d'une saisie manuelle du chemin. Couvert par 4 tests Rust et vérifié de bout en bout dans l'application (xvfb) : un client créé après la sauvegarde disparaît à la restauration, se retrouve dans la copie de sécurité, et l'utilisateur est bien déconnecté. Un sélecteur de fichier natif (`plugin-dialog`) reste à ajouter.

**Fichier** : `src-tauri/src/commands/backup.rs:40-52`, `src/pages/Settings.tsx:956`
**Risque** :
- Le frontend l'appelle sans `path`, donc sans effet (C-2).
- Une fois corrigé, n'importe quel fichier SQLite écrase la base en production sans vérification d'intégrité, de version de schéma ni sauvegarde préalable.
- Aucune rotation ni automatisation des sauvegardes.

**Fix** :

```rust
pub fn import_database(app: AppHandle, db: State<DbState>, auth: State<AuthState>, token: String, path: String) -> Result<(), String> {
    // require(..., "settings", "modifier")
    let src = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(|e| e.to_string())?;
    let ok: String = src.query_row("PRAGMA integrity_check", [], |r| r.get(0)).map_err(|e| e.to_string())?;
    if ok != "ok" { return Err("Fichier corrompu".into()); }
    let v: i64 = src.pragma_query_value(None, "user_version", |r| r.get(0)).map_err(|e| e.to_string())?;
    if v > CURRENT_SCHEMA_VERSION { return Err("Sauvegarde issue d'une version plus récente".into()); }
    backup_database_impl(&app, &db)?;              // sauvegarde automatique avant écrasement
    /* Backup::new(&src, &mut conn) ... puis migrate(&mut conn) */
}
```

Côté UI, utiliser `@tauri-apps/plugin-dialog` pour choisir le fichier, avec une confirmation `Dialog` destructive. Ajouter une sauvegarde automatique quotidienne à la clôture de session, avec rotation sur 30 jours.

### [MAJEUR] P-2 — Aucun log en production ; pas de crash handler

> **Statut : corrigé** sur `claude/hopeful-clarke-4uflms`. `tauri_plugin_log` est désormais actif dans toutes les versions, y compris en release. Il écrit dans le dossier de logs de l'OS (`%LOCALAPPDATA%\com.supercaisse.pos\logs\supercaisse.log` sous Windows), à l'heure locale, en rotation (5 Mo par fichier, 10 fichiers conservés), et affiche aussi sur la console en debug. `tao` et `wry` sont limités au niveau `warn`.
>
> Un hook de panique écrit le message et la backtrace dans le journal. Les deux `expect()` ont été retirés : `hash_password` renvoie une erreur au lieu de planter, et une erreur de démarrage est journalisée avant `exit(1)`.
>
> Les commandes critiques journalisent leur issue (succès ou échec, avec la cause) via `tracer()` : vente, annulation, conversion de document, clôture de session, validation d'inventaire et de transfert, sauvegarde, restauration.
>
> Côté frontend, les erreurs JS non gérées, les promesses rejetées et les erreurs capturées par l'`ErrorBoundary` sont transmises au même fichier par la commande `journaliser_frontend`. Cette commande ne demande pas d'être connecté, car une erreur peut survenir avant la connexion. Chaque message est limité à 4 000 caractères et nettoyé des caractères de contrôle, et le flux est plafonné à 30 messages par minute.
>
> Couvert par des tests Rust et Vitest, et vérifié dans l'application (xvfb) : le démarrage et une restauration refusée apparaissent dans `supercaisse.log`.

**Fichier** : `src-tauri/src/lib.rs:15-21`
**Risque** : `tauri_plugin_log` n'est activé qu'en `debug_assertions`, donc aucune trace en release. Les `expect()` de `run()` et `hash_password` font planter sans laisser de trace. Impossible de diagnostiquer un incident chez un client.
**Fix** :

```rust
.plugin(tauri_plugin_log::Builder::default()
    .level(log::LevelFilter::Info)
    .targets([Target::new(TargetKind::LogDir { file_name: Some("supercaisse".into()) })])
    .rotation_strategy(RotationStrategy::KeepSome(10))
    .max_file_size(5_000_000)
    .build())
// + std::panic::set_hook(Box::new(|info| log::error!("PANIC: {info}")));
// + log::error! dans chaque map_err des commandes critiques (create_vente, close_session, import_database)
```

### [MAJEUR] P-3 — Conformité DGI Maroc : points à corriger

> **Statut : corrigé** sur `claude/hopeful-clarke-4uflms`, sous réserve de la validation du modèle de facture par l'expert-comptable. C-2, C-3, C-8 et S-4 étaient déjà corrigés ; s'y ajoutent :
>
> - **Mentions du vendeur obligatoires :** aucune facture ni aucun avoir n'est émis tant que l'ICE (15 chiffres), l'IF et le RC ne sont pas renseignés (`fiscal::verifier_mentions_vendeur`, appelé par `create_vente` et `convert_document`). Le POS affiche un bandeau tant qu'ils manquent. `update_settings` refuse un ICE qui n'a pas 15 chiffres et un IF non numérique, et le formulaire affiche l'erreur.
> - **ICE client :** le format est validé à la saisie. L'ICE est exigé pour une facture adressée à un client des segments Professionnel, Grossiste ou Revendeur, à la vente comme à la conversion en facture.
> - **Annulation :**
>   - une facture ou un avoir ne peut être annulé directement que dans les 15 minutes suivant son émission, et avec un motif obligatoire ;
>   - au-delà, le serveur refuse, et l'écran Ventes propose « Émettre un avoir » ;
>   - BL, devis et commandes restent annulables librement.
> - **Ticket = facture :** la vente au POS émet une facture numérotée `FA-`. Le ticket imprimé est donc une facture simplifiée qui porte toutes les mentions. Ont été corrigés :
>   - l'intitulé du document avec son numéro fiscal, au lieu de l'id interne (« Facture #12 ») ;
>   - le Total HT, la TVA par taux et le Total TTC, sur le ticket HTML, en ESC/POS et dans le PDF ;
>   - l'IF, le RC et la patente du vendeur ;
>   - la mention « Sur facture N° … » sur un avoir ;
>   - l'en-tête « PU HT » du PDF, qui affichait « PU TTC » pour un prix HT.
> - **Conservation :** la suppression d'un article déjà vendu invite à le désactiver ; celle d'un client rappelle l'obligation de conserver les pièces pendant 10 ans.
> - **Bug trouvé en testant, et corrigé :** les paramètres ne pouvaient pas être enregistrés tant qu'un champ optionnel était vide, car `get_settings` renvoie `null` et le formulaire le refusait en silence. Les champs invalides sont maintenant signalés par un toast.
>
> Couvert par des tests Rust (formats, mentions obligatoires, ICE client B2B, délai d'annulation) et Vitest (ticket, règles fiscales, formulaire de paramètres).
>
> Vérifié dans l'application :
> - facture refusée sans mentions, avec le bandeau et le message du serveur ;
> - ICE invalide refusé, puis mentions enregistrées ;
> - facture `FA-2026-00004` émise, avec un PDF qui porte ICE/IF/RC/Patente, PU HT, Total HT, TVA 20 % et Total TTC ;
> - annulation d'une facture de plus de 15 minutes remplacée par l'avoir `AV-2026-00001`, dont le PDF indique « Sur facture N° FA-2026-00001 » ;
> - motif exigé pour une facture récente.
>
> **Reste hors code :** faire valider le modèle de facture par l'expert-comptable, et suivre le calendrier de la facturation électronique DGI.

**Fichier** : `src/lib/receipt.ts`, `commands/ventes.rs`, `commands/settings.rs`
**Risque** :
- Les mentions légales du vendeur (ICE, IF, RC, Patente) ne peuvent pas être enregistrées en production (C-2).
- Les montants HT/TVA/TTC stockés sont incohérents (C-8), alors que la facture doit détailler HT, taux et montant de TVA par taux, et TTC.
- La numérotation casse à chaque changement d'année (C-3), sans unicité garantie (S-4).
- `annuler_vente` passe une **facture** au statut `annulee`, alors que la pratique attendue est l'émission d'un **avoir** numéroté qui la référence. L'annulation directe devrait être réservée aux documents non fiscaux (devis, tickets non facturés) ou à une fenêtre très courte avant impression, avec motif obligatoire.
- L'ICE du client n'est pas exigé pour les factures B2B (client avec `segment` pro).
- Les tickets de caisse consomment la séquence `FA-` : il faut décider si le ticket est une facture (et donc porte toutes les mentions) ou un ticket distinct avec facture sur demande.
- La conservation des pièces comptables sur 10 ans est incompatible avec `delete_client` et `delete_article` sans archivage (bloqués aujourd'hui par les FK, mais sans message métier).

**Fix** :
- Corriger C-2, C-3, C-8 et S-4.
- Interdire `annuler_vente` sur `dtype='facture'` au-delà de X minutes et proposer à la place « Créer un avoir ».
- Rendre ICE/IF/RC obligatoires dans `update_settings` avant la première vente.
- Valider le format ICE (15 chiffres : `^\d{15}$`) côté Rust.
- Faire valider le modèle de facture par l'expert-comptable du client, et suivre le calendrier de la facturation électronique DGI (page `VeilleDGI`).

### [MAJEUR] P-4 — CI insuffisante pour une release

> **Constat ajouté pendant les corrections :** l'étape `npx tsc --noEmit` de la CI ne vérifie aucun fichier, car le `tsconfig.json` racine a `"files": []` et seulement des références. `npx tsc --noEmit -p tsconfig.app.json` remonte 56 erreurs de typage préexistantes. Il faut les corriger, puis passer la CI à `tsc -b` (ou `-p tsconfig.app.json`).
>
> **Statut : corrigé en partie** sur `claude/hopeful-clarke-4uflms`.
> - Les 56 erreurs de typage sont corrigées : types alignés sur les retours Rust, et composant `Tabs` réparé.
> - La CI lance `tsc -b`, `cargo fmt --check` et `cargo clippy --all-targets -D warnings` (code formaté, 26 avertissements clippy corrigés).
> - Reste à faire : tests d'intégration IPC de bout en bout et job de bundle `tauri build`.

**Fichier** : `.github/workflows/ci.yml`
**Risque** : aucun `cargo clippy`, `cargo fmt --check`, test d'intégration IPC ni build `tauri build`. Les 2 tests Rust existants ne couvrent ni `create_vente`, ni la numérotation, ni la clôture de session. Tous les défauts critiques ci-dessus passent la CI au vert.
**Fix** :

```yaml
      - run: cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
      - run: cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
      - run: cargo test --manifest-path src-tauri/Cargo.toml
  bundle:
    needs: [frontend, backend]
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with: { node-version: 20, cache: npm }
      - uses: dtolnay/rust-toolchain@stable
      - run: npm ci
      - uses: tauri-apps/tauri-action@v0
```

Tests Rust à ajouter en priorité, sur base en mémoire et avec `init_db` factorisé pour accepter une `Connection` :
- numérotation sur changement d'année ;
- vente à crédit, puis BL → facture, avec le crédit compté une seule fois ;
- `close_session` avec ventes espèces ;
- annulation d'une vente à crédit ;
- validation d'inventaire avec vente intercalée.

### [MINEUR] P-5 — Versioning et mécanisme de mise à jour

**Fichier** : `package.json:4` (`0.0.0`), `src-tauri/Cargo.toml:3` et `tauri.conf.json:4` (`0.1.0`), absence de `tauri-plugin-updater`
**Risque** : les versions sont désynchronisées, et il n'y a aucun canal de mise à jour : chaque correctif impose une réinstallation manuelle sur chaque caisse. Ce point est bloquant compte tenu de l'échéance du 1er janvier 2027 (C-3).
**Fix** :
- Aligner les trois versions (script `npm version`, qui met à jour `tauri.conf.json`).
- Ajouter `tauri-plugin-updater` avec des artefacts signés (clé `TAURI_SIGNING_PRIVATE_KEY` en secret CI) et un endpoint JSON statique.
- Afficher la version dans « À propos » et dans l'en-tête des logs.

### [MINEUR] P-6 — Bundle et impression

**Fichier** : `dist/assets` (jspdf 400 kB, html2canvas 200 kB, chunk `index` 298 kB), `commands/print.rs:8,39,85`
**Risque** : les librairies PDF sont correctement isolées en chunks, mais `index` reste lourd. Les fichiers temporaires d'impression ont des noms fixes (`ticket_escpos.bin`), d'où une collision si deux impressions sont simultanées. `print_ticket` et `print_receipt` ignorent les échecs (`let _ =`) et renvoient `Ok(())`.
**Fix** : `tempfile::NamedTempFile`, propagation des erreurs d'impression, `manualChunks` pour `@radix-ui` et `date-fns`.

---

## Tableau récapitulatif

| # | Sévérité | Couche | Problème | Fichier | Effort |
|---|----------|--------|----------|---------|--------|
| C-1 | CRITIQUE | Frontend | `invoke()` renvoie un mock sur toute erreur backend (ventes fantômes, login mock) | `src/lib/tauri.ts:320` | 2h |
| C-2 | CRITIQUE | Frontend | Clés snake_case : articles, settings DGI, restauration, lots, variantes, chèques, fidélité et plafond crédit non enregistrés | `src/lib/tauri.ts`, hooks, pages | 3h |
| C-3 | CRITIQUE | Intégrité | Numérotation : toutes les ventes en erreur dès le 01/01/2027 | `commands/ventes.rs:90` | 3h |
| C-4 | CRITIQUE | Intégrité | Clôture de session : colonne `ptype` inexistante, espèces des ventes ignorées | `commands/sessions.rs:75` | 1j |
| C-5 | CRITIQUE | Sécu | Aucune autorisation backend, rôle en localStorage, `caissier_id` fourni par le client | `commands/*`, `AuthContext.tsx` | 3-4j |
| C-6 | CRITIQUE | Prod | BDD, sauvegardes et PDF dans le CWD : panic sous Program Files | `lib.rs:127` | 4h |
| C-7 | CRITIQUE | Sécu | admin/admin recréé si l'admin est renommé ; pas de changement forcé | `db.rs:674` | 2h |
| C-8 | CRITIQUE | Intégrité | HT/TTC mélangés : CA, écarts de caisse et TVA faux | `ventes.rs`, `POS.tsx`, rapports | 2j |
| M-1 | MAJEUR | Intégrité | Montants en f64 sans arrondi | tout le schéma | 3j |
| M-2 | MAJEUR | Intégrité | Prix, remises, points et splits fournis par le client | `ventes.rs:39` | 1j |
| M-3 | MAJEUR | Intégrité | Crédit compté deux fois (BL → facture), annulation non reversée | `ventes.rs:206,546` | 1j |
| M-4 | MAJEUR | Intégrité | Stock négatif libre ; variantes et lots hors stock magasin | `mod.rs:8`, `variantes.rs`, `lots.rs` | 2j |
| M-5 | MAJEUR | Intégrité | Inventaire écrase les mouvements intermédiaires | `inventaire.rs:147` | 2h |
| M-6 | MAJEUR | Intégrité | Devis et documents convertis comptés dans le CA | `rapports.rs`, `stats.rs` | 3h |
| M-7 | MAJEUR | Sécu | XSS ticket HTML + `unsafe-inline`, donc accès IPC | `receipt.ts:50`, `tauri.conf.json` | 2h |
| M-8 | MAJEUR | Sécu | Injection de commande via `printer_name` | `print.rs:50` | 1h |
| M-9 | MAJEUR | Sécu | PIN : brute-force, collisions, permissions non rechargées | `auth.rs:47`, `AuthContext.tsx:110` | 4h |
| M-10 | MAJEUR | Frontend | Routes protégées par rôle seulement, table `permissions` ignorée | `router.tsx` | 3h |
| M-15 | MAJEUR | Backend | Commandes sync sur le thread principal + Mutex unique | toutes | 2j |
| M-16 | MAJEUR | Backend | Erreurs avalées (`unwrap_or(0.0)`, `.ok()`, `log_audit`) | `sessions`, `caisses`, `rapports`, `stats` | 1j |
| S-1 | MAJEUR | Schéma | Migrations non versionnées, erreurs ignorées | `db.rs:491` | 1j |
| S-2 | MAJEUR | Schéma | FK et CHECK absents sur les colonnes migrées et les énumérations | `db.rs:511` | 1j |
| S-4 | MAJEUR | Schéma | `numero_facture` non unique | `db.rs` | 30min |
| P-1 | MAJEUR | Prod | `import_database` sans validation ni sauvegarde préalable | `backup.rs:40` | 4h |
| P-2 | MAJEUR | Prod | Aucun log en release, pas de panic hook | `lib.rs:15` | 2h |
| P-3 | MAJEUR | Prod | Conformité DGI (mentions, avoir vs annulation, ICE client) | `ventes.rs`, `receipt.ts` | 2j |
| P-4 | MAJEUR | Prod | CI sans clippy, fmt, tests métier ni bundle | `ci.yml` | 1j |
| m-1 | MINEUR | Sécu | Énumération des logins par timing, SHA-256 legacy | `auth.rs`, `db.rs:703` | 1h |
| m-2 | MINEUR | Sécu | Deux verrous d'inactivité incohérents | `AuthContext.tsx:27` | 30min |
| S-3 | MINEUR | Schéma | Index manquants (session, source, statut, magasin) | `db.rs:588` | 30min |
| S-5 | MINEUR | Schéma | Pas de created_at, updated_at ni updated_by | tables maîtres | 3h |
| M-12 | MINEUR | Schéma | Migration du stock initial ignore les stocks ≤ 0 | `db.rs:619` | 10min |
| M-17 | MINEUR | Backend | `serde_json::Value` non typé en entrée et en sortie | `ventes.rs`, `achats.rs`… | 2j |
| M-18 | MINEUR | Backend | Code mort (`authenticate`), double système caisse/session, logique stock dupliquée | `db.rs:710`, `caisses.rs` | 1j |
| M-19 | MINEUR | Backend | Pas de pagination, LIMIT 200 silencieux, filtre `<= fin` | `ventes.rs:287`, `journal.rs`, `stock.rs` | 4h |
| F-1 | MINEUR | Frontend | Tickets en attente partagés entre utilisateurs, prix ligne modifiable | `cart.ts` | 1h |
| F-2 | MINEUR | Frontend | onError et staleTime hétérogènes | hooks | 1h |
| P-5 | MINEUR | Prod | Versions désynchronisées, pas d'updater | `package.json`, `tauri.conf.json` | 1j |
| P-6 | MINEUR | Prod | Fichiers temporaires d'impression à nom fixe, échecs ignorés | `print.rs` | 1h |
| F-3 | SUGGESTION | Frontend | a11y et warnings lint | pages | 2h |

## Plan de remédiation recommandé

1. **Sprint 0, avant tout déploiement (environ 3 jours)** :
   - C-1 et C-2 d'abord, car ils masquent tout le reste.
   - Ensuite C-3, C-6, C-7, M-8 et M-7.
   - Ajouter les tests Rust de numérotation et de vente.
2. **Sprint 1 (environ 1,5 semaine)** : C-4, C-8, M-2, M-3, M-5, M-6, S-1, S-4, P-1, P-2 et P-4.
3. **Sprint 2 (environ 1,5 semaine)** : C-5 (sessions et autorisations backend), M-9, M-10, M-15 et M-16.
4. **Sprint 3** : M-1 (centimes), M-4, S-2, P-3 (validation expert-comptable), P-5 (updater), puis les MINEURS.

## Note finale : 33 / 100

| Axe | Note | Justification |
|-----|------|---------------|
| Sécurité | **6 / 25** | Argon2id correct et aucune injection SQL, mais aucune autorisation backend, rôle client-side, admin/admin récurrent, XSS vers IPC, injection de commande |
| Intégrité données | **5 / 20** | Transactions présentes, mais numérotation cassée en 2027, HT/TTC incohérents, caisse fausse, crédit double, prix fournis par le client |
| Schéma BDD | **7 / 15** | FK activées et index de base présents ; pas de versioning, colonnes migrées sans FK ni CHECK, pas d'unicité des numéros |
| Architecture backend | **7 / 15** | Découpage modulaire propre, 97/97 commandes enregistrées ; thread principal, erreurs avalées, JSON non typé, duplication |
| Frontend | **5 / 15** | Code splitting, zod et React Query en place ; fallback mock en production et incohérence camelCase qui cassent des fonctions majeures |
| Production readiness | **3 / 10** | CI de base verte (tsc, lint, 81 tests, build) ; chemin BDD, logs, sauvegardes, updater et conformité DGI non prêts |
