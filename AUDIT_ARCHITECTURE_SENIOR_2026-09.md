# Audit Architecture Senior — SuperCaisse / RitajPOS
## 1er audit indépendant "prêt marché Maroc" — par cas d'usage retail

> Auditeur : revue technique indépendante (architecte senior), basée sur lecture intégrale du code source (`src-tauri/src/*.rs`, `src/**`), du schéma SQLite, et confrontation avec les 4 documents de cadrage existants (`SPEC_FONCTIONNELLE_RITAJ_RETAIL_MAROC.md`, `MEGA_PLAN_REFONTE.md`, `FRONTEND_CONTEXT_RITAJPOS_SUPERETTE.md`, `QA_POS_Supermarche.md`).
> Date : 2026-09-20. Portée : architecture technique, couverture fonctionnelle métier, conformité marché marocain, aptitude à servir 4 verticaux retail (supermarché, pharmacie/parapharmacie, prêt-à-porter, matériel & outils pâtisserie).
> Méthode : lecture directe du code (pas de confiance aveugle dans les checklists existantes) — chaque constat ci-dessous est vérifiable par référence `fichier:ligne`.

---

## 0. Verdict exécutif

**Le produit n'est pas "prêt marché" au sens où `MEGA_PLAN_REFONTE.md` l'affirme.** Le socle technique (Tauri + React + SQLite) est sain et le POS mono-caisse/mono-boutique pour un commerce généraliste (type épicerie/supermarché de proximité) est solide. Mais trois problèmes structurels empêchent une mise sur le marché en l'état :

1. **Dérive documentation ↔ code** : `MEGA_PLAN_REFONTE.md` déclare "TERMINÉ" des lots entiers (chaîne documentaire Devis→Commande→BL→Facture→Avoir, multi-magasin, fidélité, WhatsApp, facturation DGI) qui n'existent pas ou sont des coquilles vides côté code. C'est le constat le plus grave : la documentation ne peut plus servir de base de décision commerciale ou d'audit sans revérification systématique du code.
2. **Conformité fiscale marocaine incomplète et non fiable** : pas de PDF facture, pas de RC/Patente sur l'en-tête légal, un seul champ texte libre confondant ICE et IF, pas de règle de non-suppression réellement appliquée au niveau UI pour tous les documents.
3. **Zéro spécialisation métier réelle par vertical**, alors que c'est l'objet même de cette demande d'audit : le sélecteur "Secteur d'activité" dans Settings ne pilote qu'un seul comportement (mode table resto), les 3 autres verticaux demandés (pharmacie, prêt-à-porter, pâtisserie/matériel) ne sont pas couverts, dont un (pharmacie) absent même du sélecteur.

**Note globale : 5/10 — bon socle technique généraliste, non industrialisable en l'état sur les 4 verticaux demandés, conformité DGI à sécuriser avant toute commercialisation.**

> **Addendum 2026-09-20 (soir) — suivi des correctifs.** Ce document reste tel qu'audité :
> les constats ci-dessous ne sont pas réécrits a posteriori, par principe d'audit. Trois points
> ont depuis été corrigés et validés (`cargo check`, `tsc --noEmit`, `oxlint`, `vitest run`
> 68/68, `vite build`) : le double modèle de stock (§2, §5 item 1), l'en-tête légal ICE/IF/RC/
> Patente (§3.3, §5 item 2) et les 9 `.unwrap()` Rust sur chemins critiques (§3.2, §5 item 8).
> Une CI GitHub Actions a aussi été ajoutée. Une erreur de cet audit a été identifiée en cours
> de correction : la contrainte `UNIQUE` sur `code_barre` existait déjà (`db.rs:430`) — non
> détectée initialement faute d'avoir cherché `CREATE INDEX` en plus de `CREATE TABLE`. L'état
> courant, vérifié et tenu à jour au fil de l'eau, vit dans `ROADMAP_STATUS.md` — ce fichier-ci
> reste la photographie du 2026-09-20 matin.

---

## 1. Vue d'ensemble technique

| Couche | Choix | Appréciation |
|---|---|---|
| Frontend | React 19 + TypeScript + Vite 8, Tailwind v4, shadcn-style UI | Moderne, cohérent, bonnes pratiques (lazy routes, react-query, zod) |
| État | Zustand (cart, ui) + react-query (server state) | Séparation correcte client-state / server-state |
| Backend | Rust + Tauri v2, IPC via `invoke()`, SQLite (`rusqlite`) | Pertinent pour du desktop offline-first ; bon choix vs Electron pour la légèreté |
| Persistance | 1 fichier SQLite local, pas de service réseau | Cohérent avec un besoin mono-poste ; **incompatible avec les promesses multi-caisse/multi-magasin synchronisées du cahier des charges** (voir §3.1 et §3.4) |
| Tests | Vitest + Testing Library, colocated `*.test.tsx` | Bonne discipline de test frontend ; **aucun test Rust** (`#[cfg(test)]` absent de `commands.rs`/`db.rs`) alors que c'est là que vit la logique métier critique (transactions stock/vente) |
| CI/CD | **Absente** (`.github/workflows` inexistant) | Les "portes vertes" citées dans `MEGA_PLAN_REFONTE.md` (oxlint 0 erreur, tsc 0 erreur, 68/68 tests) ne sont vérifiées par aucune automatisation — c'est une affirmation non enforced, donc non fiable dans la durée |

Le code Rust est globalement propre : usage systématique de requêtes paramétrées (`params![...]`, pas de concaténation de valeurs utilisateur dans le SQL — pas d'injection SQL identifiée), transactions (`conn.transaction()`) sur les écritures multi-tables critiques (vente, achat, transfert, clôture session). C'est le point le plus solide de la base.

---

## 2. Constat central : la documentation ment sur l'état du code

`MEGA_PLAN_REFONTE.md` affiche "Sprint 3/4/5/6 — ✅ TERMINÉ" pour : facturation PDF conforme DGI, chaîne documentaire Devis→Commande→BL→Facture→Avoir, multi-magasin fonctionnel, ESC/POS natif, fidélité automatique, WhatsApp Business. Vérification code par code :

| Affirmation MEGA_PLAN | Réalité du code | Preuve |
|---|---|---|
| "Génération PDF ticket/facture avec mentions légales" — Sprint 3 TERMINÉ | **Aucune génération PDF nulle part.** `grep -ri pdf` sur tout `src/` et `src-tauri/src/` ne retourne aucun résultat. Le "ticket" est un fichier HTML imprimé via une popup navigateur ou un `.txt` envoyé à `notepad.exe` | `src/lib/receipt.ts`, `src-tauri/src/commands.rs:1028` (`print_ticket`), `:1416` (`print_receipt`) |
| "Chaîne documentaire Devis→Commande→BL→Facture→Avoir, chaînage parent_document_id" — Sprint 4 TERMINÉ | Il n'existe **aucune table** `devis`, `commandes_client`, `bons_livraison`, ni colonne `parent_document_id`. Toute la "chaîne" tient dans un unique champ `ventes.dtype` (facture/devis/bl/avoir) sur la même table que le ticket de caisse, sans statuts (Brouillon/Envoyé/Validé), sans écran de conversion, sans page dédiée dans le routeur | `src-tauri/src/db.rs:216-229` (table `ventes`), `src/routes/router.tsx` (aucune route `/devis`, `/commandes`, `/bl`) |
| "Multi-magasin / transfert stock (architecture BDD relationnelle)" — Sprint 5 TERMINÉ | Le transfert de stock écrit bien dans `article_stocks` (par magasin), **mais la vente et l'affichage stock (POS, page Stock, alertes) lisent/écrivent exclusivement la colonne globale unique `articles.stock`**, jamais `article_stocks`. Deux modèles de stock coexistent, déconnectés : une vente au "Magasin B" décrémente le même compteur global que le "Magasin A" voit. Le multi-magasin est un artefact de schéma, pas une fonctionnalité utilisable sans survente garantie | `src-tauri/src/commands.rs:401-402` (`UPDATE articles SET stock = stock - ?1`, sans filtre `magasin_id`) vs `:1234-1253` (`article_stocks`, seulement touché par `create_transfert`/`validate_transfert`) |
| "Fidélité — gestion automatique des points" — Sprint 6 TERMINÉ | Le calcul de points existe côté SQL (`mouvements_fidelite`), mais **aucune commande Tauri exposée** pour lire/gérer un programme de fidélité (paliers, récompenses, cartes), aucune page dédiée. Fonctionne uniquement comme sous-effet de bord de `create_vente`, sans configuration ni visibilité pour le gérant | `src-tauri/src/commands.rs:380-386` ; absence de `get_fidelite`/`add_recompense` dans la liste des commandes exposées |
| "WhatsApp Business — Relances impayés + Envoi factures direct" — Sprint 6 TERMINÉ | Implémenté comme un simple lien `wa.me` ouvrant l'app WhatsApp du poste avec un texte pré-rempli — **aucune intégration Cloud API, aucun envoi automatique, aucune pièce jointe (pas de PDF à joindre de toute façon)** | `src/pages/Ventes.tsx:67` (`sendWhatsAppInvoice`), `src/pages/Clients.tsx:59` (`sendWhatsAppReminder`) |
| "ESC/POS natif (COPY /B)" | Implémenté, mais **exclusivement via PowerShell/`notepad.exe`/`Start-Process`, donc Windows-only** — aucun chemin Linux/macOS malgré Tauri qui est multiplateforme par nature | `src-tauri/src/commands.rs:1028-1040`, `:1416-1427` |

**Recommandation immédiate** : ne plus utiliser `MEGA_PLAN_REFONTE.md` comme source de vérité pour des décisions commerciales ("c'est fait, on peut vendre") tant qu'il n'est pas regénéré à partir d'une revue de code réelle, idéalement automatisée (CI qui grep les routes/commandes réellement câblées). Le risque business est direct : vendre une promesse ("multi-magasin", "facture DGI conforme") non honorée expose l'éditeur et le client à un litige et, pour la conformité fiscale, à un risque réglementaire.

---

## 3. Analyse par domaine transverse

### 3.1 Cohérence du modèle de données
- **Stock à deux vitesses** (détaillé ci-dessus, §2) : c'est le bug d'architecture le plus grave du projet — à corriger avant tout déploiement avec plus d'un point de vente.
- `numerotation` (séquence légale par type/année) existe et est correctement incrémentée dans une transaction (`db.rs:455-465`, `commands.rs` autour de `numero_facture`), c'est une bonne base pour la contrainte DGI de numérotation sans trou — **mais rien n'empêche aujourd'hui la suppression physique d'une vente côté UI/API** au-delà de `annuler_vente` qui change le statut (`commands.rs:436-465`) ; il faudrait vérifier qu'aucune commande `delete_vente` n'existe (confirmé absente) — c'est bon sur ce point précis.
- Pas de système de **migrations versionnées** : le schéma évolue via des `ALTER TABLE ... ADD COLUMN` exécutés au démarrage et avalés par `.ok()`/erreurs ignorées (`db.rs`, multiples occurrences). Fonctionne tant que le schéma ne diverge pas entre postes, mais devient vite ingérable dès qu'il faut débugger un client en production avec une base dans un état intermédiaire inconnu.

### 3.2 Sécurité
Points positifs, vérifiés :
- Mots de passe : Argon2id avec sel aléatoire (`OsRng`), migration transparente depuis l'ancien SHA-256 au premier login réussi (`db.rs:1-31`, `:513-526`) — **conforme à l'état de l'art**, bien implémenté.
- Requêtes paramétrées partout — pas d'injection SQL trouvée.
- RBAC "grossier" fonctionnel : `ProtectedRoute` avec `allowedRoles` au niveau routeur (`router.tsx`), sidebar filtrée par rôle.

Points bloquants pour une mise en production commerciale :
- **CSP désactivée** : `"csp": null` dans `tauri.conf.json:26` — sur une webview qui affiche du contenu généré dynamiquement (designations produits, notes libres, import CSV), c'est une surface XSS/injection de contenu non négligeable pour une appli qui va gérer de l'argent.
- **Aucun journal d'audit** (qui a annulé quoi, qui a fait une remise exceptionnelle, qui a changé un prix) — obligatoire dès qu'on parle de contrôle interne en environnement caisse/retail, cité dans le cahier des charges (`SPEC_FONCTIONNELLE...md §7`) et non implémenté.
- **Pas de verrouillage de session par inactivité, pas de PIN rapide de changement de caissier** — poste tactile partagé en libre accès = risque opérationnel réel en boutique.
- 9 `.unwrap()` dans `commands.rs` : sur un serveur web ce serait un simple 500, mais **dans une appli desktop Tauri, un panic Rust côté backend peut planter tout le processus** et donc l'application entière du caissier en plein encaissement. À faire converger vers un traitement d'erreur explicite (`Result` + message utilisateur), pas un `unwrap`.

### 3.3 Conformité fiscale marocaine (DGI) — écart avec la checklist du cahier des charges

| Exigence (`SPEC_FONCTIONNELLE...md §6/§14`) | État réel |
|---|---|
| Mentions légales ICE/IF/RC/Patente sur facture | **Non conforme.** Un seul champ texte libre `tax_number` ("Numéro fiscal (ICE/IF)") confond ICE et IF ; aucun champ RC, aucun champ Patente dans `settings` (`db.rs:143-152`, `Settings.tsx:50-62`). Un contrôle DGI exigera ces 4 identifiants distincts et lisibles. |
| Numérotation séquentielle sans trou par type/année | Logique Rust présente (`numerotation`), correcte dans son principe transactionnel — mais jamais matérialisée dans un document imprimable/archivable officiel (pas de PDF, cf. §2). |
| TVA multi-taux ventilée sur le document | Implémenté et correct dans le calcul (`receipt.ts:38-49`, ventilation par taux) — c'est un des rares points DGI réellement tenus. |
| Facture non supprimable, annulation via avoir uniquement | Le circuit `annuler_vente` change un statut plutôt que supprimer (bon réflexe), mais il n'existe pas de véritable document "Avoir" distinct côté produit/numérotation dédiée visible à l'écran — seulement un `dtype='avoir'` théorique sans écran. |
| Archivage légal | Pas de politique de rétention/export légal identifiée au-delà d'un export/backup DB générique (`backup_database`, `export_database`) sans garantie d'intégrité/horodatage opposable. |
| Facturation électronique DGI (veille réglementaire) | Non commencée — acceptable à ce stade vu le calendrier de généralisation encore en cours au Maroc, mais aucun point d'ancrage architectural (pas d'abstraction "document fiscal" isolée) pour l'accueillir facilement le moment venu ; il faudra probablement refondre le modèle `ventes` en un vrai modèle documentaire pour ça de toute façon (cf. §2). |

**Conclusion conformité** : en l'état, une facture émise par ce logiciel ne réunit pas les mentions obligatoires (ICE/IF/RC/Patente distincts) et n'existe pas sous forme de document PDF archivable — c'est un frein direct à la vente commerciale du produit à des professionnels assujettis à la TVA au Maroc, pas seulement un "nice to have".

### 3.4 Promesse offline/multi-caisse vs réalité
Le cahier des charges (`SPEC_FONCTIONNELLE...md §10`) exige : "Sync multi-device temps réel (POS ↔ backoffice ↔ mobile), résolution de conflits, file d'attente de synchronisation". Le code actuel est un **SQLite fichier local unique par installation Tauri**, sans aucune couche réseau/sync (pas de serveur, pas de client HTTP, pas de queue). C'est "offline" par construction (il n'y a jamais eu de "online" à perdre), mais ce n'est pas "offline-first avec sync" — c'est mono-poste isolé. Le multi-caisse au sein d'une même boutique physique n'a de sens que si toutes les caisses pointent vers le même fichier SQLite partagé (réseau local), ce qui n'est ni documenté ni testé ici, et SQLite avec des écritures concurrentes depuis plusieurs postes sur un partage réseau est une source connue de corruption de base — **point d'attention architecture avant toute promesse "multi-caisse".**

---

## 4. Couverture par vertical métier (l'objet demandé de cet audit)

Le sélecteur `business_type` (`Settings.tsx:62`) propose `standard | restaurant | mode | vrac`, mais **une seule valeur (`restaurant`) a un effet réel dans le code** — elle active la gestion de table (`POS.tsx:69, 377, 514, 578, 674`). `mode` et `vrac` sont des libellés d'UI sans aucune branche logique associée : sélectionner "Boutique mode" ou "Vrac/Boucherie" ne change strictement rien au comportement de l'application aujourd'hui. C'est une spécialisation métier **cosmétique**, pas fonctionnelle.

### 4.1 Supermarché / épicerie (superette)
**Le mieux couvert des 4**, cohérent avec le fait que c'est le cas d'usage d'origine du produit (cf. `FRONTEND_CONTEXT_RITAJPOS_SUPERETTE.md`). Fonctionne pour : scan code-barres, grille produits, remise ligne/document, multi-paiement, session caisse Z, alertes stock bas.
Manques encore réels pour ce vertical précis :
- Pas de gestion DLC/DLUO (aucune colonne date d'expiration dans `articles`) — pourtant explicitement listé comme "spécificité sectorielle" du retail/superette dans le cahier des charges (`SPEC_FONCTIONNELLE...md §1.3`).
- Pas d'inventaire physique avec écart théorique/réel (seulement un ajustement manuel ligne par ligne) — nécessaire pour un supermarché qui fait des inventaires périodiques.
- "Vrac/poids" listé dans le sélecteur `business_type="vrac"` mais sans aucun câblage : pas de champ "unité variable (kg)" sur la fiche article, pas d'intégration balance connectée.

### 4.2 Pharmacie & Parapharmacie — **vertical non pris en charge, absent même du sélecteur**
C'est le vertical le plus en retard par rapport aux exigences réglementaires marocaines réelles :
- Pas de traçabilité **lot / numéro de série** ni **date de péremption** par lot sur `articles` — c'est une obligation réglementaire de fait pour toute pharmacie (traçabilité des médicaments, rappels de lots du Ministère de la Santé), pas une option confort.
- Pas de notion d'ordonnance (rattachement vente ↔ prescription), ni de distinction produit "sur ordonnance / libre accès / parapharmacie".
- Pas de gestion des taux de remboursement AMO/mutuelle, ni de tiers-payant — flux de paiement standard du secteur pharmaceutique marocain absent du modèle `mode_paiement`.
- TVA à taux spécifique (médicaments à taux réduit/zéro selon la classe) : le moteur TVA multi-taux générique existe et pourrait couvrir ce besoin techniquement, mais rien ne signale ce cas d'usage dans l'UI (pas de préréglage "taux médicament").
- **Recommandation** : ne pas commercialiser ce produit pour la pharmacie tant que lot+péremption+ordonnance ne sont pas dans le modèle de données — le risque n'est pas seulement fonctionnel mais réglementaire pour le client final (Conseil de l'Ordre des Pharmaciens, inspections).

### 4.3 Boutique prêt-à-porter (mode) — **schéma orphelin**
La table `article_variantes` (taille, couleur, `stock_dedie`) existe bien en base (`db.rs`, table `article_variantes`) — c'est un bon signal d'intention — mais :
- **Aucune commande Tauri** n'expose de CRUD sur `article_variantes` (absente de la liste complète des `#[tauri::command]` du projet).
- **Aucune page/UI** ne permet de créer ou vendre par déclinaison taille/couleur.
- Le POS ne sait vendre qu'un `article_id` unique avec un seul `stock` global — impossible aujourd'hui de vendre "T-shirt bleu taille M" distinctement de "T-shirt bleu taille L" avec un stock propre à chaque déclinaison.
- Pas de gestion des retours/échanges par taille typiques du secteur habillement (essayage, échange de taille sans remboursement).
- **Conclusion** : la table existe mais la fonctionnalité est à 0% côté produit utilisable — c'est de la dette de schéma, pas une fonctionnalité livrée, contrairement à ce que sa seule présence en base pourrait laisser croire à un lecteur pressé du schéma.

### 4.4 Boutique matériel & outils pâtisserie — **non couvert, aucune spécificité identifiée**
Aucune ligne de code ni de documentation ne traite ce cas explicitement. Ce vertical a des besoins assez proches d'un commerce d'équipement générique B2C/B2B :
- Pas de **prix grossiste / multi-prix** (prix public vs prix pro) — pourtant listé au cahier des charges (`SPEC_FONCTIONNELLE...md §3.1`) et particulièrement pertinent pour un commerce qui vend à la fois à des particuliers (loisir pâtisserie) et des professionnels (boulangers, pâtissiers, traiteurs).
- Pas de **produits composés/kits** (ex. "kit moules + emporte-pièces" vendu comme un ensemble décrémentant plusieurs références) — cité comme besoin générique dans le cahier des charges (§3.1) et directement applicable à ce vertical (coffrets, kits de démarrage).
- Pas d'unités de vente multiples (pièce / lot de X / carton) sur une même fiche article — utile pour du matériel vendu à l'unité et en gros conditionnement.
- Ce vertical n'a par ailleurs aucune contrainte réglementaire marocaine spécifique forte (contrairement à la pharmacie) : c'est avant tout un manque fonctionnel de gestion catalogue B2B/B2C, pas un risque de conformité.

### 4.5 Synthèse comparative

| Vertical | Couverture réelle | Bloquant principal |
|---|---|---|
| Supermarché / épicerie | ~70% (socle solide, DLC et inventaire manquants) | Absence DLC/péremption |
| Pharmacie / parapharmacie | ~10% (aucune spécificité, absente du sélecteur) | Traçabilité lot/péremption + ordonnance = obligation réglementaire non couverte |
| Prêt-à-porter | ~5% (schéma en base, 0 UI/API) | Variantes taille/couleur non exploitables en vente |
| Matériel/outils pâtisserie | ~0% (aucune spécificité) | Pas de multi-prix B2B ni de kits — fonctionnel, pas réglementaire |

---

## 5. Dette technique — risques classés par impact

| # | Risque | Impact | Effort correction |
|---|---|---|---|
| 1 | Deux modèles de stock déconnectés (`articles.stock` vs `article_stocks`) | Survente garantie dès 2 points de vente | Moyen — unifier sur `article_stocks`, migrer `articles.stock` en vue calculée |
| 2 | Pas de PDF ni de document fiscal réel (juste HTML/TXT imprimé) | Bloque toute vente commerciale à un professionnel assujetti | Moyen-élevé — introduire une vraie couche "document fiscal" |
| 3 | `MEGA_PLAN_REFONTE.md` non fiable comme source de vérité | Risque de décision commerciale sur base fausse | Faible — reconstruire le doc depuis le code, brancher un script de vérif en CI |
| 4 | Impression Windows-only (`powershell`/`notepad.exe`) | Casse silencieusement sur tout poste macOS/Linux | Moyen — passer par un plugin Tauri ESC/POS multiplateforme (ex. `tauri-plugin-*` ou crate série/USB) |
| 5 | Pas de CI/CD | Toute régression passe inaperçue jusqu'au terrain | Faible — GitHub Actions : `tsc --noEmit`, `oxlint`, `vitest run`, `cargo check`/`cargo test` |
| 6 | Pas de journal d'audit ni de session lock/PIN | Contrôle interne caisse absent | Moyen |
| 7 | `CSP: null` | Surface XSS sur webview qui gère de l'argent | Faible |
| 8 | 9×`.unwrap()` côté Rust | Panic = crash total de l'app en plein encaissement | Faible |
| 9 | Pas de migrations SQL versionnées | Dérive de schéma imprévisible entre installations client | Moyen |
| 10 | Aucun test Rust | Logique métier critique (stock, caisse, TVA) non testée côté backend | Moyen |

---

## 6. Recommandations priorisées

**P0 — avant toute vente commerciale (juridique/financier)**
1. Séparer et rendre obligatoires les champs ICE / IF / RC / Patente de l'entreprise dans Settings (actuellement un seul champ texte libre).
2. Générer un vrai PDF facture/ticket archivable (actuellement HTML imprimé au vol, non réutilisable comme pièce comptable).
3. Corriger le double modèle de stock avant toute promesse commerciale "multi-magasin".
4. Mettre en place une CI minimale (tsc, oxlint, vitest, cargo check) pour que les affirmations "0 erreur / X/X tests" redeviennent vérifiables en continu.

**P1 — fiabiliser le socle**
5. Réécrire `MEGA_PLAN_REFONTE.md` depuis une lecture de code, avec un statut par item recoupé avec le nom de la commande Tauri et de la route frontend qui le sert réellement (sinon `⬜`, pas `✅`).
6. Ajouter journal d'audit, verrouillage session, PIN caissier rapide.
7. Remplacer les `.unwrap()` Rust par une gestion d'erreur explicite.
8. Rendre l'impression ESC/POS multiplateforme (ou documenter/assumer explicitement "Windows uniquement" comme contrainte produit si c'est le choix commercial).

**P2 — spécialisation verticale (objet de cette demande)**
9. Faire du `business_type` un vrai levier fonctionnel : au minimum, brancher `mode` sur un écran de gestion des variantes taille/couleur déjà modélisées en base, et ajouter une valeur `pharmacie` qui active lot + date de péremption + alerte rappel de lot.
10. Ajouter lot/péremption au niveau article (colonnes génériques réutilisables aussi par le supermarché pour le DLC alimentaire — un seul chantier sert deux verticaux).
11. Ajouter multi-prix (public/grossiste) et produits composés/kits — sert à la fois le vertical pâtisserie/matériel et le besoin générique déjà écrit dans `SPEC_FONCTIONNELLE_RITAJ_RETAIL_MAROC.md §3.1`.

---

## 7. Verdict "prêt marché" — checklist gate (mise à jour factuelle de celle du cahier des charges)

| Exigence | Statut réel vérifié au code |
|---|---|
| Facture conforme mentions légales (ICE, IF, RC, TVA détaillée) | ❌ — champ fiscal unique non conforme, pas de RC/Patente |
| Numérotation séquentielle sans trou | 🔶 — logique correcte, jamais matérialisée en document officiel |
| Fonctionnement sans connexion internet | ✅ — SQLite local, aucune dépendance réseau identifiée |
| Clôture de caisse (Z) avec écart signé | ✅ — `sessions_caisse` + `close_session` implémentés et transactionnels |
| Sauvegarde/export des données accessible au client | ✅ — `backup_database`/`export_database`/`import_database` présents |
| Interface bilingue FR/AR | ❌ — aucune trace d'i18n ni de RTL dans le code |
| Compatible matériel POS existant (imprimante, douchette, tiroir) | 🔶 — douchette OK (focus+Enter), imprimante Windows-only, tiroir dépend de l'imprimante |
| Multi-magasin fonctionnel | ❌ — modèle de stock à deux vitesses, non fiable en production |
| Spécialisation par vertical (objet de cette demande) | ❌ — un seul vertical sur quatre (restaurant) réellement câblé, les 3 autres cosmétiques ou absents |

**4 exigences sur 9 sont réellement tenues.** Le produit est un bon socle POS généraliste mono-boutique, pas encore un produit "prêt marché multi-secteurs" tel que documenté.

---

*Audit réalisé par lecture exhaustive du code source, sans exécution de la suite de tests (environnement sans `node_modules` au moment de l'audit — à revalider en CI). Toute contestation d'un constat ci-dessus doit être vérifiable par la référence fichier:ligne fournie.*
