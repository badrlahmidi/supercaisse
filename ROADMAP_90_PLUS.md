# SuperCaisse — Roadmap 78 → 90+

Score actuel : **78/100** (B+). Objectif : **90+/100** (A).
Audit du 23 sept. 2026 — PR #1 merged (commit 49ad028).

---

## Contexte Technique

| Couche | Stack | Fichiers clés |
|--------|-------|---------------|
| Backend | Rust + rusqlite + Tauri v2 IPC | `src-tauri/src/commands.rs` (3049 lignes, 80 commandes), `db.rs` (schéma + migrations), `lib.rs` (handler registration) |
| Frontend | React 19 + TypeScript 6 + Vite 8 | `src/pages/` (28+ pages), `src/hooks/`, `src/store/`, `src/ui/` (13 primitives) |
| Config | `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml` | |
| Tests | Vitest (72 tests, 16 fichiers), 0 tests Rust | `src/**/*.test.{ts,tsx}` |
| CI | GitHub Actions `.github/workflows/ci.yml` | Frontend (tsc, lint, vitest, vite build) + Backend (cargo check) |

### Patterns existants

- Toutes les requêtes SQL utilisent `params![]` (pas d'injection)
- Les opérations critiques (ventes, annulations, conversions, transferts, inventaire) utilisent `conn.transaction()` + `tx.commit()`
- Le frontend utilise `invoke()` pour IPC Tauri (mocké dans `src/lib/tauri.ts` pour les tests)
- Zustand stores avec `persist` middleware (cart.ts, ui.ts)
- react-hook-form + zod pour la validation
- ErrorBoundary + Suspense sur chaque route lazy

### Commandes de validation (OBLIGATOIRES avant chaque push)

```bash
cargo check --manifest-path src-tauri/Cargo.toml
npx tsc --noEmit
npx vite build
npm test -- --run
```

### Conventions (AGENTS.md)

- Pas de commentaires inline dans le code
- Pas de barrel exports — import direct depuis le fichier
- `export default function PageName()` sur chaque page
- `navigate()` au lieu de `<a href>` (SPA)
- Delete confirmations : Dialog + AlertTriangle + bouton destructive
- Dark mode via `.dark` class sur `<html>`, contrôlé par `useUIStore`

---

## ✅ Chantier 1 — Refactorer commands.rs en modules (+3 Code Quality)

**Impact estimé : Code Quality 16→19** — **TERMINÉ** (commit 0ab321e)

> 3050 lignes → 28 modules domaine + mod.rs. 98 commandes préservées.
> Validé : cargo check, tsc, vite build, vitest 72/72.

### Problème

`src-tauri/src/commands.rs` = 3049 lignes, 80+ commandes Tauri dans un seul fichier. Aucune séparation logique. Maintenance et navigation très difficiles.

### Plan de découpage

Créer `src-tauri/src/commands/` comme module directory avec `mod.rs` qui ré-exporte tout.

#### Modules proposés et leurs commandes

**`commands/auth.rs`** (lignes 43-76)
```
login, login_pin, set_user_pin
```

**`commands/categories.rs`** (lignes 78-126)
```
get_categories, add_category, update_category, delete_category
```

**`commands/fournisseurs.rs`** (lignes 128-192)
```
get_fournisseurs, add_fournisseur, update_fournisseur, delete_fournisseur
```

**`commands/clients.rs`** (lignes 193-329)
```
get_clients, add_client, update_client, delete_client
```

**`commands/articles.rs`** (lignes 330-620)
```
get_articles, add_article, update_article, delete_article,
update_article_stock, import_articles_csv
```

**`commands/lots.rs`** (lignes ~466-530)
```
add_article_lot, get_article_lots, get_lots_peremption_proche, discard_article_lot
```

**`commands/variantes.rs`** (lignes ~531-620)
```
add_article_variante, get_article_variantes, update_article_variante,
adjust_article_variante_stock, delete_article_variante, find_variante_by_barcode
```

**`commands/composants.rs`** (lignes ~572-620)
```
add_article_composant, get_article_composants,
update_article_composant_quantite, delete_article_composant
```

**`commands/ventes.rs`** (lignes 623-1000)
```
create_vente, annuler_vente, get_ventes, get_vente_details, convert_document
```

**`commands/achats.rs`** (lignes 1188-1260)
```
create_achat, get_achats, update_achat_status
```

**`commands/paiements.rs`** (lignes 1258-1590)
```
get_cheques, add_cheque, update_cheque_status,
get_paiements, add_paiement
```

**`commands/sessions.rs`** (lignes 1289-1460)
```
get_current_session, open_session, close_session
```

**`commands/magasins.rs`** (lignes 1339-1460)
```
get_magasins, add_magasin, update_magasin, delete_magasin,
get_stock_par_magasin, get_transferts, create_transfert, validate_transfert
```

**`commands/rapports.rs`** (lignes 2310-2500+)
```
get_rapport_x, get_rapport_detaille, get_releve_client
```

**`commands/stats.rs`** (lignes 1681-1830)
```
get_stats, get_articles_stock_alerte
```

**`commands/print.rs`** (lignes 1827-1930)
```
print_ticket, print_escpos, open_cash_drawer, print_receipt, save_document_pdf
```

**`commands/settings.rs`** (lignes 2145-2220)
```
get_settings, update_settings, get_utilisateurs, add_utilisateur,
update_utilisateur, delete_utilisateur
```

**`commands/journal.rs`** (lignes ~1835-1900)
```
get_journal_caisse, add_journal_caisse, get_mouvements_stock
```

**`commands/backup.rs`** (lignes 2217-2280)
```
backup_database, export_database, import_database
```

**`commands/inventaire.rs`** (lignes 2600-2810)
```
create_inventaire, get_inventaire, get_inventaires,
update_inventaire_ligne, valider_inventaire
```

**`commands/tables.rs`**
```
get_tables, update_table_status
```

**`commands/audit.rs`**
```
get_audit_log
```

**`commands/permissions.rs`**
```
get_permissions, update_permission
```

**`commands/caisses.rs`**
```
get_caisses, open_caisse, close_caisse, get_tresorerie, compare_fournisseur_prices
```

### Étapes d'implémentation

1. Créer le dossier `src-tauri/src/commands/`
2. Créer `src-tauri/src/commands/mod.rs` :
```rust
mod auth;
mod categories;
mod fournisseurs;
mod clients;
mod articles;
mod lots;
mod variantes;
mod composants;
mod ventes;
mod achats;
mod paiements;
mod sessions;
mod magasins;
mod rapports;
mod stats;
mod print;
mod settings;
mod journal;
mod backup;
mod inventaire;
mod tables;
mod audit;
mod permissions;
mod caisses;

pub use auth::*;
pub use categories::*;
// ... etc pour chaque module
```
3. Les fonctions utilitaires partagées (`default_magasin_id`, `adjust_article_stock`, `log_audit`) doivent rester dans `mod.rs` ou dans un `commands/helpers.rs` et être `pub(crate)`
4. Chaque module utilise :
```rust
use crate::db::*;
use rusqlite::params;
use tauri::State;
```
5. `lib.rs` change `mod commands;` — le compilateur résout automatiquement `commands/mod.rs`
6. Le handler dans `lib.rs` ne change PAS — les `commands::login` etc. restent valides grâce aux `pub use`

### Validation

```bash
cargo check --manifest-path src-tauri/Cargo.toml  # doit compiler sans erreur
npx tsc --noEmit
npm test -- --run
```

---

## ✅ Chantier 2 — Activer CSP + cargo test en CI (+3 Production Ready)

**Impact estimé : Production Ready 18→21** — **TERMINÉ**

> CSP activée dans tauri.conf.json. cargo test ajouté au CI. 2 tests Rust créés (default_magasin_id, adjust_article_stock).

### 2a. Activer CSP dans tauri.conf.json

**Fichier** : `src-tauri/tauri.conf.json`

**État actuel** (ligne 26) :
```json
"security": {
  "csp": null
}
```

**Cible** :
```json
"security": {
  "csp": "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; font-src 'self' data:; connect-src 'self' ipc: http://ipc.localhost"
}
```

La politique autorise :
- Scripts et styles inline (nécessaire pour React/Vite)
- Images data: et blob: (pour logos base64 et PDF)
- Connexions IPC (obligatoire pour Tauri invoke)
- Pas de sources externes (tout est local)

### 2b. Ajouter cargo test en CI

**Fichier** : `.github/workflows/ci.yml`

Ajouter à la section backend :
```yaml
- name: Run Rust tests
  run: cargo test --manifest-path src-tauri/Cargo.toml
```

Note : il n'y a actuellement aucun test Rust. La commande passe quand même (0 tests = succès). Cela prépare le terrain pour les tests à venir.

### 2c. (Bonus) Créer un premier test Rust

**Fichier** : `src-tauri/src/commands/helpers.rs` (ou `commands.rs` si pas encore refactoré)

```rust
#[cfg(test)]
mod tests {
    use rusqlite::Connection;
    use super::*;

    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("
            CREATE TABLE magasins (id INTEGER PRIMARY KEY, nom TEXT);
            CREATE TABLE articles (id INTEGER PRIMARY KEY, stock REAL DEFAULT 0);
            CREATE TABLE article_stocks (
                id INTEGER PRIMARY KEY,
                article_id INTEGER, magasin_id INTEGER, quantite REAL DEFAULT 0,
                UNIQUE(article_id, magasin_id)
            );
            INSERT INTO magasins (id, nom) VALUES (1, 'Principal');
            INSERT INTO articles (id, stock) VALUES (1, 0);
        ").unwrap();
        conn
    }

    #[test]
    fn test_default_magasin_id() {
        let conn = setup_test_db();
        assert_eq!(default_magasin_id(&conn).unwrap(), 1);
    }

    #[test]
    fn test_adjust_article_stock() {
        let conn = setup_test_db();
        adjust_article_stock(&conn, 1, 1, 10.0).unwrap();
        let stock: f64 = conn.query_row(
            "SELECT stock FROM articles WHERE id = 1", [], |r| r.get(0)
        ).unwrap();
        assert_eq!(stock, 10.0);

        adjust_article_stock(&conn, 1, 1, -3.0).unwrap();
        let stock: f64 = conn.query_row(
            "SELECT stock FROM articles WHERE id = 1", [], |r| r.get(0)
        ).unwrap();
        assert_eq!(stock, 7.0);
    }
}
```

---

## Chantier 3 — Unifier les types TypeScript (+2 Code Quality)

**Impact estimé : Code Quality 19→21**

### Problème

Les interfaces `Article`, `Client`, `Category` sont dupliquées dans :
- `src/lib/tauri.ts` (définition + mock)
- `src/pages/POS.tsx` (Article redéfini lignes 27-38, Client lignes 65+)
- `src/pages/Clients.tsx` (Client redéfini ligne 21)
- `src/hooks/useClients.ts` (Client redéfini ligne 5)
- `src/hooks/useSales.ts` (Sale/SaleLine lignes 5-28)

### Solution

1. **Créer `src/types/index.ts`** avec toutes les interfaces partagées :

```typescript
export interface Article {
  id: number
  code_barre: string | null
  designation: string
  image_url?: string | null
  prix_achat: number
  prix_vente: number
  tva: number
  stock: number
  stock_alerte: number | null
  categorie_id: number | null
  fournisseur_id: number | null
  actif: boolean
  categorie_nom?: string
  fournisseur_nom?: string
  suivi_lot?: boolean
  prix_grossiste?: number | null
  est_kit?: boolean
  a_variantes?: boolean
}

export interface ArticleVariante {
  id: number
  taille: string | null
  couleur: string | null
  code_barre: string | null
  stock_dedie: number
}

export interface Client {
  id: number
  code: string | null
  nom: string
  adresse: string | null
  telephone: string | null
  email: string | null
  ice: string | null
  credit_plafond: number
  credit_actuel: number
  points_fidelite: number
  segment: string | null  // MANQUANT dans certaines redéfinitions
}

export interface Category {
  id: number
  nom: string
  description: string | null
}

export interface Fournisseur {
  id: number
  nom: string
  telephone: string | null
  email: string | null
  adresse: string | null
  ice: string | null
}

export interface Sale {
  id: number
  date: string
  client_nom?: string
  caissier_nom: string
  montant_total: number
  montant_remise: number
  net_paye: number
  mode_paiement: string
  statut: string
  numero_facture?: string
  dtype: string
  articles?: SaleLine[]
}

export interface SaleLine {
  id: number
  article_id: number
  designation: string
  quantite: number
  prix_unitaire: number
  tva: number
  total_ligne: number
}
```

2. **Mettre à jour les imports** dans chaque fichier :
   - `src/pages/POS.tsx` : supprimer les interfaces locales Article (L27-38), ArticleVariante (L40-46), Client (L65+), importer depuis `@/types`
   - `src/pages/Clients.tsx` : supprimer interface Client (L21), importer depuis `@/types`
   - `src/hooks/useClients.ts` : supprimer export interface Client (L5), importer depuis `@/types`
   - `src/hooks/useSales.ts` : supprimer Sale/SaleLine (L5-28), importer depuis `@/types`
   - `src/lib/tauri.ts` : supprimer les interfaces (L1-70), importer depuis `@/types`, garder seulement le mock `invoke`

3. **Vérification** : chercher `interface Article`, `interface Client`, `interface Category` dans tout `src/` — ne doit exister QUE dans `src/types/index.ts`

### Validation

```bash
npx tsc --noEmit   # toutes les pages doivent compiler
npm test -- --run  # tous les tests doivent passer
npx vite build     # le build de prod doit passer
```

---

## Chantier 4 — Transactions manquantes (+2 Code Quality, +1 Production Ready)

**Impact estimé : Code Quality 21→23, Production Ready 21→22**

### 4a. add_paiement — wrapper dans une transaction

**Fichier** : `src-tauri/src/commands.rs` (ou `commands/paiements.rs` si refactoré)
**Lignes** : 1577-1586

**État actuel** (PAS de transaction) :
```rust
pub fn add_paiement(db: State<DbState>, client_id: i64, montant: f64, ptype: String, reference: Option<String>) -> Result<i64, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO paiements (client_id, montant, type, reference) VALUES (?1, ?2, ?3, ?4)",
        params![client_id, montant, ptype, reference],
    ).map_err(|e| e.to_string())?;
    conn.execute("UPDATE clients SET credit_actuel = credit_actuel - ?1 WHERE id = ?2",
        params![montant, client_id]).map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}
```

**Cible** :
```rust
pub fn add_paiement(db: State<DbState>, client_id: i64, montant: f64, ptype: String, reference: Option<String>) -> Result<i64, String> {
    let mut conn = db.conn.lock().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    tx.execute(
        "INSERT INTO paiements (client_id, montant, type, reference) VALUES (?1, ?2, ?3, ?4)",
        params![client_id, montant, ptype, reference],
    ).map_err(|e| e.to_string())?;
    let paiement_id = tx.last_insert_rowid();
    tx.execute("UPDATE clients SET credit_actuel = credit_actuel - ?1 WHERE id = ?2",
        params![montant, client_id]).map_err(|e| e.to_string())?;
    log_audit(&tx, None, "ajouter_paiement", &format!("Paiement #{} client #{}: {} DH", paiement_id, client_id, montant), Some("paiement"), Some(paiement_id));
    tx.commit().map_err(|e| e.to_string())?;
    Ok(paiement_id)
}
```

Note : `let mut conn` est obligatoire pour `.transaction()`.

### 4b. Split payment — déplacer dans la transaction

**Fichier** : `src-tauri/src/commands.rs` (ou `commands/ventes.rs`)
**Lignes** : 794-809

**Problème** : les écritures split payment s'exécutent APRÈS `tx.commit()` sous un nouveau lock. Si le process crash entre les deux, la vente existe mais les splits journal sont perdus.

**État actuel** :
```rust
tx.commit().map_err(|e| e.to_string())?;

if let Some(ref split_list) = splits {
    if split_list.len() > 1 {
        let conn2 = db.conn.lock().map_err(|e| e.to_string())?;
        for s in split_list {
            let mode = s["mode"].as_str().unwrap_or("inconnu").to_string();
            let montant = s["montant"].as_f64().unwrap_or(0.0);
            let desc = format!("Split vente #{}: {}", vente_id, mode);
            let _ = conn2.execute(
                "INSERT INTO journal_caisse (utilisateur_id, jtype, montant, description) VALUES (?1, 'encaissement', ?2, ?3)",
                params![caissier_id, montant, desc],
            );
        }
    }
}
```

**Cible** — déplacer les splits AVANT `tx.commit()` :
```rust
if let Some(ref split_list) = splits {
    if split_list.len() > 1 {
        for s in split_list {
            let mode = s["mode"].as_str().unwrap_or("inconnu").to_string();
            let montant = s["montant"].as_f64().unwrap_or(0.0);
            let desc = format!("Split vente #{}: {}", vente_id, mode);
            tx.execute(
                "INSERT INTO journal_caisse (utilisateur_id, jtype, montant, description) VALUES (?1, 'encaissement', ?2, ?3)",
                params![caissier_id, montant, desc],
            ).map_err(|e| e.to_string())?;
        }
    }
}

tx.commit().map_err(|e| e.to_string())?;
```

Attention : supprimer le `let conn2 = db.conn.lock()` — on utilise `tx` directement. Plus besoin du second lock.

### 4c. import_articles_csv — wrapper dans une transaction

**Fichier** : `src-tauri/src/commands.rs` (ou `commands/articles.rs`)
**Lignes** : 1627-1677

**Problème** : chaque article est inséré individuellement sans transaction. Un crash au milieu laisse un import partiel sans rollback possible.

**Cible** : wrapper le loop dans `conn.transaction()`. Remplacer `let conn = db.conn.lock()` par `let mut conn = db.conn.lock()` puis `let tx = conn.transaction()`. Utiliser `tx` partout. Commit à la fin.

```rust
pub fn import_articles_csv(db: State<DbState>, csv_content: String) -> Result<String, String> {
    let mut conn = db.conn.lock().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let magasin_id = default_magasin_id(&tx)?;
    // ... boucle existante mais avec tx au lieu de conn ...
    tx.commit().map_err(|e| e.to_string())?;
    Ok(report)
}
```

---

## Chantier 5 — Confirmation suppression Articles + Error state Rapports (+2 UI/UX)

**Impact estimé : UI/UX 19→21**

### 5a. Confirmation suppression article

**Fichier** : `src/pages/Articles.tsx` — ligne 504

**État actuel** (suppression directe sans confirmation !) :
```tsx
<Button variant="ghost" size="icon" onClick={() => deleteMutation.mutate(article.id)} title="Supprimer">
  <Trash2 className="h-4 w-4 text-destructive" />
</Button>
```

**Cible** — utiliser le pattern ConfirmDialog existant (`src/components/ConfirmDialog.tsx`) :

1. Ajouter un state pour l'article à supprimer :
```tsx
const [articleToDelete, setArticleToDelete] = useState<number | null>(null)
```

2. Remplacer le onClick :
```tsx
<Button variant="ghost" size="icon" onClick={() => setArticleToDelete(article.id)} title="Supprimer">
  <Trash2 className="h-4 w-4 text-destructive" />
</Button>
```

3. Ajouter le ConfirmDialog (avant la fermeture du fragment principal) :
```tsx
<ConfirmDialog
  open={articleToDelete !== null}
  onOpenChange={(open) => { if (!open) setArticleToDelete(null) }}
  title="Supprimer l'article"
  description="Cette action est irréversible. L'article sera définitivement supprimé."
  onConfirm={() => {
    if (articleToDelete) deleteMutation.mutate(articleToDelete)
    setArticleToDelete(null)
  }}
/>
```

4. Importer ConfirmDialog :
```tsx
import ConfirmDialog from "@/components/ConfirmDialog"
```

### 5b. Error state sur Rapports

**Fichier** : `src/pages/Rapports.tsx`

La page utilise `useQuery` mais n'affiche rien en cas d'erreur.

Ajouter après le check de loading :
```tsx
if (query.isError) {
  return (
    <div className="p-8 text-center">
      <PageHeader title="Rapports" />
      <Card className="mt-4">
        <CardContent className="py-8">
          <p className="text-destructive font-medium">Erreur de chargement du rapport</p>
          <p className="text-sm text-muted-foreground mt-1">{String(query.error)}</p>
          <Button className="mt-4" onClick={() => query.refetch()}>Réessayer</Button>
        </CardContent>
      </Card>
    </div>
  )
}
```

---

## Chantier 6 — Audit log élargi (+1 Production Ready)

**Impact estimé : Production Ready 22→23**

### Problème

`log_audit` n'est appelé que 8 fois sur 80+ commandes. Les opérations sensibles suivantes ne sont PAS auditées :

### Commandes à auditer

| Commande | Action audit | Priorité |
|----------|-------------|----------|
| `create_vente` | `"creer_vente"` | Haute |
| `add_paiement` | `"ajouter_paiement"` | Haute |
| `delete_client` | `"supprimer_client"` | Haute |
| `delete_category` | `"supprimer_categorie"` | Moyenne |
| `delete_fournisseur` | `"supprimer_fournisseur"` | Moyenne |
| `add_utilisateur` | `"ajouter_utilisateur"` | Haute |
| `update_utilisateur` | `"modifier_utilisateur"` | Haute |
| `delete_utilisateur` | `"supprimer_utilisateur"` | Haute |
| `open_session` | `"ouvrir_session"` | Haute |
| `close_session` | `"fermer_session"` | Haute |
| `validate_transfert` | `"valider_transfert"` | Haute |
| `import_articles_csv` | `"importer_csv"` | Moyenne |
| `import_database` | `"importer_base"` | Haute |
| `update_permission` | `"modifier_permission"` | Haute |
| `login` (succès) | `"connexion"` | Moyenne |
| `login` (échec) | `"echec_connexion"` | Haute |

### Exemple pour create_vente

Dans `create_vente`, juste avant `tx.commit()` :
```rust
log_audit(&tx, caissier_id, "creer_vente",
    &format!("Vente #{} - {} DH ({}) - {}", vente_id, montant_total, dtype.as_deref().unwrap_or("facture"), mode_paiement),
    Some("vente"), Some(vente_id));
```

### Pattern

```rust
log_audit(&conn_or_tx, utilisateur_id, "action_name", "description lisible", Some("reference_type"), Some(reference_id));
```

Note : `log_audit` utilise `let _ =` (fire-and-forget). C'est intentionnel — un échec d'audit ne doit pas bloquer l'opération métier.

---

## Chantier 7 (Bonus) — Tests frontend manquants (+1 Code Quality)

**Impact estimé : Code Quality 23→24**

### Pages sans test

Les pages suivantes n'ont PAS de fichier `.test.tsx` :

| Page | Complexité | Priorité |
|------|-----------|----------|
| `POS.tsx` | 990 lignes, la plus complexe | Haute |
| `Rapports.tsx` | 335 lignes | Moyenne |
| `Magasins.tsx` | 500 lignes | Moyenne |
| `AuditLog.tsx` | | Basse |
| `PeremptionsStock.tsx` | | Basse |
| `Boutiques.tsx` | | Basse |
| `Inventaire.tsx` | | Basse |
| `RapprochementBancaire.tsx` | | Basse |
| `VeilleDGI.tsx` | | Basse |

### Template de test pour une page

```tsx
import { render, screen } from "@testing-library/react"
import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import { MemoryRouter } from "react-router-dom"
import { vi, describe, it, expect, beforeEach } from "vitest"

vi.mock("@/lib/tauri", () => import("@/lib/tauri"))

const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } })
function wrap(ui: React.ReactElement) {
  return (
    <QueryClientProvider client={qc}>
      <MemoryRouter>{ui}</MemoryRouter>
    </QueryClientProvider>
  )
}

// import dynamique pour React.lazy compatibilité
describe("PageName", () => {
  beforeEach(() => { qc.clear() })

  it("renders without crashing", async () => {
    const { default: Page } = await import("./PageName")
    render(wrap(<Page />))
    expect(screen.getByText(/expected text/i)).toBeInTheDocument()
  })
})
```

Note : le mock `@/lib/tauri` pointe vers le fichier existant `src/lib/tauri.ts` qui contient déjà un mock complet de `invoke()`.

---

## Résumé des gains attendus

| Chantier | Code Quality | Fonctions Metier | UI/UX | Production Ready | Total |
|----------|:----------:|:----------:|:-----:|:----------:|:-----:|
| 1. Modules Rust | +3 | — | — | — | +3 |
| 2. CSP + cargo test | — | — | — | +3 | +3 |
| 3. Types unifiés | +2 | — | — | — | +2 |
| 4. Transactions | +2 | — | — | +1 | +3 |
| 5. UI fixes | — | — | +2 | — | +2 |
| 6. Audit log | — | — | — | +1 | +1 |
| 7. Tests (bonus) | +1 | — | — | — | +1 |
| **Total** | **+8** | **—** | **+2** | **+5** | **+15** |
| **Nouveau score** | **24** | **25** | **21** | **23** | **93** |

### Ordre d'exécution recommandé

1. **Chantier 3** (Types) — rapide, peu de risque, débloque la clarté
2. **Chantier 5** (UI fixes) — rapide, 2 fichiers
3. **Chantier 4** (Transactions) — moyen, touche au backend critique
4. **Chantier 6** (Audit log) — moyen, ajouts ponctuels
5. **Chantier 1** (Modules) — le plus gros, à faire proprement
6. **Chantier 2** (CSP + CI) — config, rapide
7. **Chantier 7** (Tests bonus) — si le temps le permet

### Commandes de validation finale

```bash
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
npx tsc --noEmit
npx vite build
npm test -- --run
```

Tous doivent passer avant chaque push.
