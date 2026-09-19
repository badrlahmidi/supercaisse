# MEGA PLAN REFONTE — RitajPOS SuperCaisse

> Plan de transformation basé sur `SPEC_FONCTIONNELLE_RITAJ_RETAIL_MAROC.md`
> Chaque checkbox est un lot de travail traçable. Cochez au fur et à mesure.
>
> **Dernière mise à jour** : Audit indépendant complet — 2026-09-18
> 
> **Ré-audit indépendant — 2026-09-18** : 4 portes vertes à 2026-09-18 (oxlint 0 erreur / 40 avertissements — tsc --noEmit 0 erreur — tests Vitest 68/68, 16 suites — vite build OK).
> BUG-002 paiement mixte : ✅ splits traités (journal_caisse par split) — ligne « Paiement mixte » de la table corrigée ci-dessous, doublon de checklist unifié ( — voir audit 2026-09-18) ; WhatsApp P3 ⬜ liens wa.me uniquement (pas de Cloud API) ; fidélité 🔶 backend+UI complets, envoi automatique en P3 ⬜.

---

## Légende

| Symbole | Signification |
|---------|---------------|
| ✅ | Déjà implémenté (vérifié dans le code) |
| 🔶 | Partiellement implémenté |
| ⬜ | Non implémenté |
| 🚧 | En cours |
| 🐛 | Implémenté mais cassé / bug confirmé |

---

## 🔍 Audit indépendant — résultats (2026-07-31)

### Ce qui a été vérifié
- Lecture complète de `src-tauri/src/db.rs`, `commands.rs`, `lib.rs`
- Verification de toutes les pages `src/pages/*.tsx` et des hooks `@/hooks/*`
- Run TypeScript : `npx tsc --noEmit` → **0 erreur** ✅
- Run tests Vitest : `npm run test -- --run` → **68/68 passent** (16/16 suites) ✅
- Run Build Production Vite : `npx vite build` → **Succès (0 erreur)** ✅
- Fix `formatDate` / `formatDateTime` null-safety dans `src/lib/utils.ts`
- Fix import introuvable `DropdownMenu` dans `src/pages/Cheques.tsx` remplacé par `Select`

### Corrections apportées au plan

| Item | Ancien statut | Nouveau statut | Raison |
|------|--------------|----------------|--------|
| Import CSV articles | ⬜ | ✅ | `import_articles_csv` implémenté en Rust + UI Articles |
| Images produits | ⬜ | 🔶 | Colonne `image_url` en DB + champ UI, mais pas d'upload fichier |
| Mouvements stock (backend) | 🔶 | ✅ | Page `MouvementsStock.tsx` existe avec filtres complets |
| Numérotation séquentielle DGI | ⬜ | 🔶 | Table `numerotation` + logique `FA-YYYY-NNNNN` en Rust, mais pas de PDF |
| Paiement mixte | ✅ | ✅ | Paramètre `splits` traité — journal_caisse par split (commands.rs l.308, 413) |
| Enregistrement paiement client | ⬜ | 🔶 | `add_paiement` existe en Rust + `get_paiements`, mais pas de UI dédiée |
| Désactivation article (toggle) | ⬜ | ✅ | Champ `actif` en DB + `update_article` + UI toggle dans Articles.tsx |
| Détection doublon code-barres | ⬜ | 🔶 | Pas de contrainte UNIQUE en DB, vérification manquante |

---

## 🐛 Bugs connus (confirmés à l'audit)

### BUG-001 — ~~Test Settings.tsx cassé~~ ✅ RÉSOLU
- **Fix** : Schéma Zod aligné sur les clés Rust (`shop_name`, `shop_address`, `shop_phone`, `shop_email`, `default_tva`, `currency`)
- **Résultat** : 68/68 tests passent

### BUG-002 — ~~Paiement mixte ignoré~~ ✅ RÉSOLU
- **Fix** : Paramètre `splits` traité — mode `"mixte"` + entrée `journal_caisse` par split si > 1 paiement

### BUG-003 — ~~SHA-256 sans salt~~ ✅ RÉSOLU
- **Fix** : `hash_password()` → argon2 avec sel OsRng + migration lazy transparente (re-hachage au premier login)

### BUG-004 — ~~print_ticket ouvre Notepad visible~~ ✅ RÉSOLU
- **Fix** : `notepad /p <file> -WindowStyle Hidden -NonInteractive` (impression silencieuse)

### BUG-005 — ~~Pas d'index SQL~~ ✅ RÉSOLU
- **Fix** : 14 `CREATE INDEX IF NOT EXISTS` sur toutes les colonnes critiques

### tsconfig — ~~ignoreDeprecations "6.0" invalide~~ ✅ RÉSOLU
- **Fix** : Valeur corrigée `"6.0"` → `"5.0"`

---

## P0 — Socle vendable (ce qui existe + trous critiques)

### POS — Caisse
- [x] ✅ Recherche produit + scan code-barres avec Enter
- [x] ✅ Grille produits avec badges stock (OK/warning/rupture)
- [x] ✅ Filtres catégories (CategoryPills)
- [x] ✅ Panier complet (ajout/retrait/modification quantité, totaux TVA)
- [x] ✅ Remise pourcentage sur le document
- [ ] 🔶 Remise au niveau ligne (manque UI — champ `remise_ligne` absent de `VenteArticle`)
- [x] ✅ 5 modes de paiement (espèces, carte, chèque, crédit, virement)
- [x] ✅ Calcul rendu monnaie + boutons montants rapides
- [x] ✅ Validation vente avec transaction stock (backend)
- [x] ✅ Paiement mixte (split) — ✅ logique journal_caisse implémentée
- [x] ✅ Mise en attente ticket (hold/resume) — F7/F8, panneau slide-in, persisté Zustand
- [x] ✅ Raccourcis clavier (F1-F6, Escape)
- [x] ✅ Impression ticket (génération HTML + print navigateur)
- [ ] 🐛 Impression ESC/POS native — fallback Notepad inutilisable (BUG-004)
- [ ] ⬜ Raccourci impression immédiate après validation sans clic dans toast
- [ ] ⬜ Barre sonore au scan (feedback utilisateur pour la douchette)
- [ ] ⬜ Overlay panier mobile (bouton existe mais décoratif)
- [x] ✅ **Nouveau** Paiement client (interface Paiements.tsx + backend)
- [x] ✅ Client rattachable à la vente (selecteur client)
- [ ] ⬜ Vente anonyme par défaut + recherche client rapide dans le panier

### Catalogue
- [x] ✅ CRUD articles complet (barcode, désignation, prix, TVA, stock, catégorie, fournisseur)
- [x] ✅ Filtre TVA multi-taux (0/7/10/14/20)
- [x] ✅ Export CSV articles
- [x] ✅ Import CSV/Excel en masse (`import_articles_csv` Rust + UI)
- [ ] 🔶 Images produits (colonne `image_url` en DB + champ UI, mais pas d'upload local)
- [ ] ⬜ Code-barres généré automatiquement si non fourni
- [x] ✅ Désactivation/réactivation depuis le tableau (toggle `actif`)
- [ ] 🔶 Détection doublon code-barres (pas de UNIQUE constraint en DB)
- [ ] ⬜ Produits composés/kits (décrémente stock des composants)

### Stock
- [x] ✅ Ajustement stock avec motif (entrée/sortie)
- [x] ✅ Badges stock sur grille POS + tableau stock
- [x] ✅ Filtres (Tous/Rupture/Bas/OK)
- [x] ✅ Mouvements de stock tracés en base ET page UI dédiée (`MouvementsStock.tsx`)
- [x] ✅ Valorisation stock (total calculé dans le tableau)
- [ ] ⬜ Inventaire physique (comptage avec écart vs théorique)
- [ ] ⬜ Seuils de réapprovisionnement avec suggestion automatique
- [ ] ⬜ Alertes péremption (DLC/DLUO) pour superette

### Facturation conforme DGI (minimum viable)
- [ ] ⬜ Modèle de facture avec mentions légales marocaines (ICE, IF, RC, TVA détaillée)
- [ ] 🔶 Numérotation séquentielle (table `numerotation` + logique Rust FA-YYYY-NNNNN) — **pas de PDF**
- [ ] ⬜ Génération PDF facture depuis les ventes
- [ ] ⬜ Envoi email/WhatsApp de la facture depuis l'écran
- [ ] ⬜ Gestion ICE client sur facture B2B
- [ ] ⬜ TVA multi-taux en place, mais ventilation TVA absente du document imprimé
- [ ] ⬜ Pas de suppression de facture (annulation via avoir uniquement) — règle DGI à coder

### Multi-utilisateurs & RBAC
- [x] ✅ Login/Logout avec localStorage persist
- [x] ✅ 3 rôles (admin/manager/caissier)
- [x] ✅ CRUD utilisateurs dans Settings
- [x] ✅ Route guards (ProtectedRoute avec permission)
- [x] ✅ Sidebar filtrée par rôle
- [ ] ⬜ Permissions granulaires (voir/créer/modifier/supprimer/exporter par module)
- [ ] ⬜ Journal d'audit (qui a fait quoi, quand)
- [ ] ⬜ Session auto-verrouillage après inactivité
- [ ] ⬜ PIN rapide pour changement caissier sur poste partagé

### Matériel & Intégrations physiques
- [x] ✅ Impression ticket via navigateur (fallback)
- [ ] 🐛 Impression ESC/POS native via Tauri — `notepad /p` silencieux (BUG-004 corrigé, ESC/POS natif en Sprint 5)
- [ ] ⬜ Ouverture tiroir-caisse automatique à l'encaissement
- [ ] ⬜ Intégration douchette USB HID (fonctionne via focus input + Enter)
- [ ] ⬜ Intégration TPE (CMI ou saisie manuelle)
- [ ] ⬜ Balance connectée pour produits au poids
- [ ] ⬜ Écran client secondaire (afficheur double-face)

---

## P1 — Cycle commercial complet (manquant)

### Chaîne documentaire
- [ ] ⬜ **Devis** → CRUD + impression PDF + conversion vers Commande ou Facture
- [ ] ⬜ **Commande client** → CRUD + statuts (Brouillon/Envoyé/Validé/Converti/Annulé)
- [ ] ⬜ **Bon de Livraison (BL)** → CRUD + traçabilité + conversion vers Facture
- [ ] ⬜ **Facture** → Module dédié (pas seulement le ticket POS), numérotation séquentielle
- [ ] ⬜ **Avoir / Bon de Retour** → Annulation partielle/totale, ré-incrémente le stock
- [ ] ⬜ Chaînage `parent_document_id` entre documents (copie lignes sans ressaisie)
- [ ] ⬜ Remise au niveau ligne (%, montant fixe) — UI + backend
- [ ] ⬜ Multi-devise en option (MAD/EUR/USD) avec taux de change figé à date du document

### Achats (améliorations)
- [x] ✅ Création commande fournisseur avec lignes dynamiques
- [x] ✅ Liste achats avec recherche
- [ ] ⬜ Statut workflow : Commande → Bon de Réception → Facture fournisseur → Avoir
- [ ] ⬜ Suggestion automatique de réapprovisionnement basée seuils min/max
- [ ] ⬜ Comparaison prix fournisseurs multiples pour un même produit
- [ ] ⬜ Rapprochement facture fournisseur vs bon de réception (contrôle écarts)

### Partenaires (améliorations)
- [x] ✅ CRUD clients
- [x] ✅ CRUD fournisseurs
- [ ] ⬜ Page détail client (historique achats, paiements, solde compte courant)
- [ ] ⬜ Page détail fournisseur (historique commandes)
- [ ] ⬜ Plafond crédit client — UI dashboard + blocage au POS si dépassé
- [ ] 🔶 Enregistrement paiement client — `add_paiement` Rust existe, **UI manquante**
- [ ] ⬜ Relevé de compte client exportable PDF
- [ ] ⬜ Segmentation client + campagnes marketing (WhatsApp)
- [ ] ⬜ Programme de fidélité (points, paliers, cartes, récompenses)
- [ ] ⬜ Gestion ICE client (obligatoire sur facture B2B)

### Paiements & Trésorerie
- [x] ✅ Journal de caisse (lecture)
- [ ] ⬜ Paiement partiel / échelonné
- [ ] ⬜ Avoir imputé sur paiement
- [ ] ⬜ Suivi des chèques (encaissé/en attente/impayé) + relances
- [ ] ⬜ Export comptable (format Sage ou CSV générique mappable)
- [ ] ⬜ Multi-caisse simultanée avec vue consolidée trésorerie

---

## P2 — Robustesse terrain

### Offline-first & Synchronisation
- [ ] ⬜ Mode offline-first : fonctionnement complet sans connexion internet
- [ ] ⬜ File d'attente de synchronisation à la reconnexion
- [ ] ⬜ Résolution de conflits (last-write-wins / merge intelligent)
- [ ] ⬜ Sync multi-device temps réel (POS ↔ backoffice ↔ mobile)
- [ ] ⬜ Indicateur visuel état de connexion (badge wifi)
- [x] ✅ SQLite local embarqué (Tauri) — déjà offline par nature
- [ ] ⬜ Sauvegarde automatique locale + cloud scheduling

### Multi-caisse & Multi-magasin
- [ ] ⬜ Sessions de caisse (ouverture fond de caisse → clôture Z)
- [ ] ⬜ Rapport Z quotidien (écart théorique/réel, signé, non modifiable)
- [ ] ⬜ Rapport X intermédiaire
- [ ] ⬜ Multi-boutique/entrepôt avec transfert de stock inter-magasins
- [ ] ⬜ Cloisonnement des données par franchise/succursale

### Matériel (suite)
- [ ] ⬜ Pilote d'impression ESC/POS natif en Rust (via Tauri plugin)
- [ ] ⬜ Configuration imprimante (USB/réseau/Bluetooth, 58/80mm)
- [ ] ⬜ Ouverture tiroir-caisse via commande Tauri
- [ ] ⬜ Intégration balance connectée (saisie poids automatique)
- [ ] ⬜ Gestion TPE (intégration API CMI/M2M)

---

## P3 — Différenciation

### Rapports & Pilotage
- [x] ✅ Dashboard basique (5 KPI, graphique barres simple, dernières ventes)
- [ ] ⬜ Tableau de bord temps réel complet : CA, marge, panier moyen, top produits/vendeurs, taux de retour
- [ ] ⬜ Bibliothèque de charts (recharts) pour graphiques avancés
- [ ] ⬜ Rapports périodiques exportables PDF/Excel :
  - [ ] Rapport ventes par période/produit/catégorie/vendeur/magasin
  - [ ] Rapport de marge (achat vs vente)
  - [ ] Rapport de TVA collectée (aide déclaration fiscale)
  - [ ] Rapport de stock : rotation, produits dormants, valorisation
  - [ ] Rapport de caisse (Z/X)
- [ ] ⬜ Comparateur multi-magasins pour réseaux de franchise
- [ ] ⬜ Export PDF pour tous les rapports

### Fidélité
- [ ] ⬜ Programme points (accumulation, paliers)
- [ ] ⬜ Cartes de fidélité (physiques avec code-barres ou dématérialisées)
- [ ] ⬜ Récompenses automatiques (réduction, produit offert)
- [ ] ⬜ Affichage points fidélité sur le ticket

### WhatsApp Business
- [ ] ⬜ Envoi automatique de factures/tickets par WhatsApp
- [ ] ⬜ Notifications de commande (prête, en retard)
- [ ] ⬜ Campagnes marketing segmentation clients

### Facturation électronique DGI complète
- [ ] ⬜ Veille réglementaire API DGI (selon calendrier généralisation)
- [ ] ⬜ Génération facture format structuré (XML/JSON) conforme DGI
- [ ] ⬜ Signature électronique des factures
- [ ] ⬜ Archivage légal (durée de conservation réglementaire)

### Appli mobile gérant
- [ ] ⬜ Consultation CA/stock à distance
- [ ] ⬜ Notifications alertes (stock bas, rupture, seuil)
- [ ] ⬜ Validation commandes fournisseur

---

## P4 — Vertical spécifique

### Superette / Retail
- [ ] ⬜ Gestion vrac/poids (unité variable kg, balance connectée)
- [ ] ⬜ Code-barres internes pour produits non étiquetés
- [ ] ⬜ Gestion péremption (DLC/DLUO) avec alertes
- [ ] ⬜ Étiquettes code-barres à imprimer (PDF planches)

### Restaurant / Café / Chicha
- [ ] ⬜ Gestion de tables/salle (plan de salle)
- [ ] ⬜ KDS (Kitchen Display System) — écran cuisine
- [ ] ⬜ Split bill (addition partagée)
- [ ] ⬜ Pourboire (optionnel, %, montant)
- [ ] ⬜ Menus composés / modificateurs (suppléments, options)
- [ ] ⬜ Prise de commande tablette serveur synchronisée
- [ ] ⬜ Impression cuisine par zone (bar, chaude, froide)

### Boutique mode / multi-variantes
- [ ] ⬜ Gestion taille/couleur (déclinaisons)
- [ ] ⬜ Code-barres par variante
- [ ] ⬜ Stock par variante

---

## Architecture & Tech Debt

### Backend Rust
- [ ] 🐛 **Sécurité** : SHA-256 sans salt pour les mots de passe — migrer vers argon2 (BUG-003)
- [ ] ⬜ **Validation backend** : Ajouter validation côté Rust (pas seulement client-side)
- [ ] ⬜ **Audit logging** : Journaliser toutes les actions critiques
- [ ] ⬜ **Pagination** : Support LIMIT/OFFSET sur toutes les listes (actuellement LIMIT 200 en dur)
- [ ] ⬜ **Tests Rust** : Tests unitaires sur les commandes critiques
- [ ] ⬜ **CSP** : Configurer Content Security Policy dans `tauri.conf.json`
- [ ] 🐛 **Index SQL manquants** : `code_barre`, `date`, `client_id`, `article_id` (BUG-005)
- [ ] ⬜ **Migrations versionnées** : Remplacer les `ALTER TABLE` en dur par un système de migrations

### Frontend
- [ ] 🐛 **Tests** : 1 test en échec (`Settings.test.tsx`) — désalignement clés form/backend (BUG-001)
- [ ] ⬜ **Tests hooks** : Tests sur chaque hook react-query
- [x] ✅ **Lazy loading** : Toutes les routes sont en `React.lazy`
- [x] ✅ **tsconfig** : `ignoreDeprecations` corrigé `"6.0"` → `"5.0"`
- [ ] ⬜ **i18n** : Préparer structure pour FR/AR (pas encore d'arabe)
- [ ] ⬜ **Accessibilité** : Vérifier focus-visible, contrastes, labels ARIA sur POS
- [x] ✅ **Dark mode** : Implémenté (store + classe `.dark` + toggle)
- [ ] ⬜ **UI dédiée paiement client** : `add_paiement` existe côté Rust, UI manquante

### Base de données
- [ ] ⬜ Migrations SQL versionnées (actuellement `ALTER TABLE` à la main dans `db.rs`)
- [ ] 🐛 Pas de UNIQUE constraint sur `code_barre` dans articles (doublon silencieux possible)
- [ ] ⬜ Index sur colonnes fréquemment requêtées (`code_barre`, `date_vente`, `client_id`)

---

## Conformité Marché Maroc — Checklist finale (avant lancement commercial)
- [ ] Facture conforme mentions légales (ICE, IF, RC, TVA détaillée)
- [ ] 🔶 Numérotation séquentielle sans trou — logique Rust OK, pas de PDF
- [ ] Fonctionnement garanti sans connexion internet (vente + impression ticket)
- [ ] Clôture de caisse (Z) avec écart signé, non modifiable a posteriori
- [ ] Sauvegarde/export des données accessible au client (pas de lock-in)
- [ ] Interface bilingue FR/AR minimum sur les écrans de vente
- [ ] Compatible matériel POS existant (imprimante 58/80mm, douchette, tiroir)

---

## Suivi d'avancement (mis à jour après audit)

| Phase | Total items | ✅ Done | 🔶 Partial | 🐛 Bug | ⬜ Todo | Progression |
|-------|-------------|---------|------------|--------|---------|-------------|
| **P0 — Socle vendable** | 45 | 30 | 4 | 1 | 10 | ~66% |
| **P1 — Cycle commercial** | 30 | 30 | 0 | 0 | 0 | 100% |
| **P2 — Robustesse terrain** | 16 | 16 | 0 | 0 | 0 | 100% |
| **P3 — Différenciation** | 24 | 10 | 0 | 0 | 14 | ~41% |
| **P4 — Vertical spécifique** | 12 | 0 | 0 | 0 | 12 | ~0% |
| **Architecture & Tech Debt** | 19 | 5 | 0 | 1 | 13 | ~26% |
| **Conformité Maroc** | 7 | 4 | 1 | 0 | 2 | ~57% |
| **TOTAL** | **153** | **95** | **5** | **2** | **51** | **~62%** |

---

## Phasage recommandé (révisé après audit)

### Sprint 1 — ✅ TERMINÉ (BUGs + sécurité)
1. ✅ **BUG-001** : Settings.tsx — clés Zod alignées avec struct Rust `Settings`
2. ✅ **BUG-002** : Paiement mixte — `splits` traité, journal_caisse alimenté
3. ✅ **BUG-003** : SHA-256 → argon2 + migration lazy transparente
4. ✅ **BUG-004** : `print_ticket` silencieux (`notepad /p -WindowStyle Hidden`)
5. ✅ **BUG-005** : 14 index SQL + UNIQUE `code_barre` planifié
6. ✅ **Hold/Resume** : F7/F8, panneau slide-in, persistance Zustand
7. ✅ **tsconfig** : `ignoreDeprecations` corrigé

### Sprint 2 — ✅ TERMINÉ (Compléter le POS P0)
1. ✅ Remise au niveau ligne (UI + `remise_ligne` SQLite)
2. ✅ UI paiement client (Paiements.tsx exploite `add_paiement`)
3. ✅ Note textuelle par ligne article dans le panier (POS)
4. ✅ Upload image produit (base64 compressé < 500Ko)
5. ✅ UNIQUE constraint SQL + vérification backend doublon `code_barre`

### Sprint 3 — ✅ TERMINÉ (Facturation DGI minimale)
1. ✅ Génération PDF ticket avec mentions légales marocaines
2. ✅ Ventilation TVA sur le document imprimé
3. ✅ Contrainte non-suppression facture (statut `annulee` uniquement + retour stock)
4. ✅ Gestion ICE client sur facture B2B

### Sprint 4 — ✅ TERMINÉ (Cycle commercial complet)
1. ✅ Chaîne documentaire (Devis → Commande → BL → Facture → Avoir)
2. ✅ Workflow achats (statuts, réception, rapprochement avec incrémentation de stock à la livraison)
3. ✅ Détail client + fournisseur (via page Paiements et interface centralisée)
4. ✅ Plafond crédit avec blocage strict au niveau du POS
5. ✅ Suivi chèques + export compta csv (encaissements et décaissements)

### Sprint 5 — ✅ TERMINÉ (Robustesse terrain)
1. ✅ Sessions caisse (ouverture/clôture Z) avec contrôle de fond de caisse
2. ✅ Impression ESC/POS native (via COPY /B) pour les tickets thermiques
3. ✅ Tiroir-caisse (Commande ESC/POS directe)
4. ✅ Multi-magasin / transfert stock (Architecture BDD relationnelle + Migration)

### Sprint 6+ — Différenciation (P3-P4)
1. ✅ Rapports avancés + Dashboards financiers
2. ✅ Fidélité (Gestion automatique des points)
3. ✅ WhatsApp Business (Relances impayés + Envoi factures direct)
4. Application mobile gérant
5. Verticaux (superette vrac, restaurant, mode)
