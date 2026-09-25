# SuperCaisse — Audit Pre-Production Complet

## Prompt pour Claude Opus 5.5

Copier-coller ce prompt dans une nouvelle session Claude Opus 5.5 avec le repo `badrlahmidi/supercaisse` (branche `claude/audit-retail-maroc-architecture-o8lhtj`).

---

```
Tu es un auditeur technique senior spécialisé retail/POS. Réalise un audit complet pré-production de l'application SuperCaisse — un système de point de vente (POS) pour le marché marocain.

## Contexte projet

- **Stack** : Tauri v2 (Rust backend + SQLite via rusqlite) + React 19 + TypeScript + Vite 8
- **Frontend** : react-router-dom v7 (28 lazy routes), @tanstack/react-query v5, Zustand (cart + UI stores), react-hook-form + zod, shadcn/ui-style, Tailwind CSS v4
- **Backend** : 97 commandes Tauri (IPC), 30 tables SQLite, Argon2 pour les mots de passe
- **Taille** : ~4100 lignes Rust, ~10200 lignes TSX (pages), 30 tables BDD
- **CI** : GitHub Actions — tsc, lint, vitest, vite build (frontend) + cargo check, cargo test (backend)
- **Domaine** : POS retail Maroc — factures, devis, BL, avoirs, crédit client, multi-magasin, inventaire, fidélité, sessions caisse, transferts stock, cheques, audit log

## Structure du code

### Backend (src-tauri/src/)
- `lib.rs` (136 lignes) — point d'entrée, enregistrement des 97 commandes, init DB
- `db.rs` (739 lignes) — structs Rust (Category, Fournisseur, Client, Article, Utilisateur, Settings), init_db() avec 30 CREATE TABLE + 27 ALTER TABLE migrations, hash_password/verify_password
- `commands/mod.rs` (126 lignes) — helpers partagés (default_magasin_id, adjust_article_stock, log_audit) + 2 tests unitaires
- `commands/ventes.rs` (573 lignes) — create_vente, annuler_vente, convert_document (le plus complexe)
- `commands/articles.rs` (211 lignes) — CRUD articles, import CSV
- `commands/` — 28 modules au total (auth, sessions, magasins, inventaire, achats, paiements, etc.)

### Frontend (src/)
- `pages/` — 28 pages (POS.tsx 990 lignes, le plus complexe)
- `hooks/` — 16 custom hooks (useProducts, useClients, useSales, etc.)
- `store/` — cart.ts (panier Zustand persisté), ui.ts (dark mode, sidebar)
- `types/index.ts` — interfaces TypeScript canoniques (14 types)
- `lib/tauri.ts` — wrapper invoke() avec mock data pour dev hors Tauri
- `ui/` — composants shadcn/ui (Button, Card, Dialog, Table, etc.)
- `components/` — Layout, PageHeader, EmptyState, ErrorBoundary, SuspensePage
- `routes/router.tsx` — configuration routes lazy-loaded
- Tests : 19 fichiers, 81 tests (Vitest + testing-library)

### Base de données (30 tables)
categories, fournisseurs, clients, articles, utilisateurs, ventes, vente_articles, achats, achat_articles, paiements, mouvements_stock, journal_caisse, settings, sessions_caisse, cheques, magasins, article_stocks, transferts_stock, transfert_lignes, mouvements_fidelite, tables_resto, article_variantes, audit_log, inventaires, inventaire_lignes, permissions, article_composants, caisses, article_lots, numerotation

### Migrations
27 ALTER TABLE ADD COLUMN exécutés via `let _ = conn.execute(...)` (fire-and-forget, pas de versioning). Aucun schema_version ni PRAGMA user_version.

### Sécurité actuelle
- CSP activée : `default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; font-src 'self' data:; connect-src 'self' ipc: http://ipc.localhost`
- Mots de passe : Argon2 (avec migration automatique depuis SHA256)
- Audit log : log_audit() sur 16+ commandes sensibles
- DB locale SQLite (pas de réseau, pas d'API externe)

## INSTRUCTIONS D'AUDIT

Réalise un audit structuré en 6 couches. Pour chaque couche, lis les fichiers pertinents, identifie les problèmes, classe-les par sévérité (CRITIQUE / MAJEUR / MINEUR / SUGGESTION), et fournis le fix concret.

### Couche 1 — Sécurité & Authentification
Fichiers : `src-tauri/src/commands/auth.rs`, `src-tauri/src/db.rs` (hash_password, verify_password), `src/context/AuthContext.tsx`, `src-tauri/src/commands/permissions.rs`

Vérifier :
- [ ] Robustesse Argon2 (paramètres, salt, timing attacks)
- [ ] Gestion des sessions (tokens, expiration, invalidation)
- [ ] Contrôle d'accès par rôle (RBAC) — est-il appliqué côté backend ou uniquement frontend ?
- [ ] Injection SQL — tous les params! sont-ils sécurisés ? Y a-t-il des string formatting dans les queries ?
- [ ] CSP : `unsafe-inline` pour scripts/styles — risque XSS ?
- [ ] PIN login : brute-force protection ?
- [ ] Stockage secrets (pas de hardcoded credentials ?)
- [ ] Permissions : vérifiées avant chaque commande Tauri ou uniquement côté UI ?

### Couche 2 — Intégrité des données & Transactions
Fichiers : `src-tauri/src/commands/ventes.rs`, `commands/paiements.rs`, `commands/articles.rs`, `commands/magasins.rs`, `commands/inventaire.rs`, `commands/sessions.rs`

Vérifier :
- [ ] Toutes les opérations multi-tables utilisent-elles des transactions ?
- [ ] Quelles commandes font encore `conn.execute` au lieu de `tx.execute` pour des opérations liées ?
- [ ] Race conditions sur `db.conn.lock()` — le Mutex global est-il suffisant ?
- [ ] Calculs financiers : f64 pour les montants — erreurs de précision ? Devrait-on utiliser des entiers (centimes) ?
- [ ] Stock négatif : est-il possible ? Est-ce vérifié ?
- [ ] Crédit client : le plafond est-il toujours vérifié atomiquement ?
- [ ] Numérotation factures : garantie d'unicité sous concurrence ?
- [ ] Cohérence stock articles vs article_stocks (double tracking ?)

### Couche 3 — Migrations & Schéma BDD
Fichier : `src-tauri/src/db.rs` (lignes 170-640)

Vérifier :
- [ ] Absence de versioning (pas de schema_version, pas de PRAGMA user_version)
- [ ] 27 migrations via `let _ = conn.execute(ALTER TABLE...)` — erreurs silencieuses ?
- [ ] Colonnes ajoutées sans DEFAULT qui pourraient casser les INSERT existants
- [ ] Colonnes dupliquées (mouvements_stock.magasin_id ajouté 2 fois, lignes 624 et 628)
- [ ] Index manquants sur les colonnes de recherche/jointure fréquentes
- [ ] Contraintes d'intégrité (FOREIGN KEY, NOT NULL, CHECK, UNIQUE) — quelles tables en manquent ?
- [ ] PRAGMA foreign_keys = ON — est-il activé ?
- [ ] Tables sans audit de qui/quand a créé/modifié les enregistrements (created_at, updated_at)

### Couche 4 — Architecture Backend Rust
Fichiers : `src-tauri/src/lib.rs`, `commands/mod.rs`, tous les modules commands/

Vérifier :
- [ ] Gestion d'erreurs : `unwrap()`, `unwrap_or()`, `.ok()` silencieux — où sont-ils dangereux ?
- [ ] Tous les `let _ =` — quels échecs silencieux sont acceptables vs dangereux ?
- [ ] Sérialisation : `serde_json::Value` vs structs typées — quelles commandes retournent du JSON non typé ?
- [ ] Performance : N+1 queries, requêtes sans LIMIT, full table scans
- [ ] Gestion mémoire : le Mutex global `Arc<Mutex<Connection>>` — goulot d'étranglement ?
- [ ] Commandes enregistrées dans lib.rs vs commandes existantes dans commands/ — y a-t-il des commandes non enregistrées ?
- [ ] Code mort : fonctions/structs inutilisées dans db.rs (authenticate() ?)
- [ ] Duplication de logique entre modules

### Couche 5 — Frontend React/TypeScript
Fichiers : `src/pages/POS.tsx`, `src/store/cart.ts`, `src/context/AuthContext.tsx`, `src/hooks/`, `src/types/index.ts`, `src/lib/tauri.ts`

Vérifier :
- [ ] État du panier : persistance Zustand — données sensibles dans localStorage ?
- [ ] Gestion d'erreurs : toutes les mutations ont-elles onError avec toast ?
- [ ] Formulaires : validation zod côté client — validation serveur correspondante ?
- [ ] Types : cohérence entre les interfaces TS et les structs Rust / retours JSON
- [ ] Performance : re-renders inutiles, composants non memoizés, queries sans staleTime
- [ ] Accessibilité : labels manquants, navigation clavier, contraste
- [ ] Routes protégées : vérification du rôle sur chaque route ?
- [ ] XSS : dangerouslySetInnerHTML, insertion de HTML non échappé ?
- [ ] Offline : que se passe-t-il si Tauri IPC échoue ? Erreurs silencieuses ?
- [ ] Mock data dans tauri.ts : risque de fuite en production ?

### Couche 6 — Production Readiness
Fichiers : `tauri.conf.json`, `.github/workflows/ci.yml`, `package.json`, `Cargo.toml`

Vérifier :
- [ ] Version 0.1.0 — prêt pour un tag de release ?
- [ ] CI : tests suffisants ? Pas de e2e, pas de lint Rust (clippy)
- [ ] Build de production : taille du binaire, code splitting effectif ?
- [ ] Backup/restore : testé ? Le import_database écrase-t-il tout sans confirmation ?
- [ ] Logging : structuré ? Suffisant pour diagnostiquer en production ?
- [ ] Error reporting : crash handler, error boundaries sur toutes les routes ?
- [ ] DGI compliance Maroc : format facture, numérotation séquentielle, champs obligatoires (ICE, TVA)
- [ ] Données de test/mock : présentes en production ?
- [ ] Update mechanism : comment l'app se met-elle à jour ?

## FORMAT DE SORTIE

Pour chaque finding :

```
### [CRITIQUE|MAJEUR|MINEUR|SUGGESTION] — Titre court

**Fichier** : `path/to/file.rs:ligne`
**Risque** : Description du problème et de son impact en production
**Fix** :
\`\`\`rust|tsx
// Code correctif
\`\`\`
```

Termine par un tableau récapitulatif :

| # | Sévérité | Couche | Problème | Fichier | Effort |
|---|----------|--------|----------|---------|--------|
| 1 | CRITIQUE | Sécu   | ...      | ...     | 1h     |

Et une note finale /100 avec la répartition :
- Sécurité : /25
- Intégrité données : /20
- Schéma BDD : /15
- Architecture backend : /15
- Frontend : /15
- Production readiness : /10
```

---
