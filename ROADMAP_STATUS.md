# ROADMAP_STATUS — SuperCaisse / RitajPOS
## Suivi d'avancement vérifié dans le code (remplace `docs/archive/MEGA_PLAN_REFONTE.md`)

> **Règle de tenue de ce document** : un statut ✅ n'est écrit que s'il existe une commande
> Tauri exposée **et** une route/page frontend qui l'appelle **et**, pour les écritures, une
> table SQL qui la porte réellement (pas seulement déclarée). Toute mise à jour de ce fichier
> doit citer `fichier:ligne`. Aucune exception — c'est précisément l'absence de cette discipline
> qui a rendu `docs/archive/MEGA_PLAN_REFONTE.md` inutilisable (voir
> `AUDIT_ARCHITECTURE_SENIOR_2026-09.md §2`).
>
> Dernière vérification : 2026-09-20, par lecture intégrale de `src-tauri/src/{db,commands,lib}.rs`
> et `src/{pages,routes,hooks,lib}/**`.
>
> **Mise à jour 2026-09-20 (soir)** : 5 correctifs/ajouts de cet audit ont été implémentés et
> validés (`cargo check`, `tsc --noEmit`, `oxlint`, `vitest run` 68/68, `vite build` — tous verts ;
> une CI GitHub Actions existe désormais dans `.github/workflows/ci.yml` pour que ces statuts
> restent vérifiés automatiquement) : stock multi-magasin unifié, en-tête légal ICE/IF/RC/Patente
> séparé, durcissement des `.unwrap()` Rust sur les chemins critiques, génération PDF facture/avoir
> (`jsPDF` + commande `save_document_pdf`), traçabilité lot/péremption (DLC-DLUO) avec écran
> d'alerte dédié. Détail dans les sections correspondantes ci-dessous. Une erreur de l'audit initial a aussi été corrigée : la contrainte
> `UNIQUE` sur `code_barre` existait déjà (`db.rs:430`, une simple recherche `CREATE TABLE` sans
> chercher `CREATE INDEX` l'avait fait manquer) — jamais confirmé sans le grep exact, y compris
> nos propres constats précédents.

## Légende
| Symbole | Signification |
|---|---|
| ✅ | Implémenté et câblé de bout en bout (DB → commande Tauri → UI) |
| 🔶 | Partiel — au moins une couche manque (ex: table DB sans commande, ou commande sans UI) |
| ⬜ | Non implémenté |

---

## P0 — Socle POS (mono-caisse, mono-boutique effective)

### Vente / Caisse
| Item | Statut | Preuve |
|---|---|---|
| Scan/recherche produit, panier, quantités, remise ligne + document | ✅ | `src/pages/POS.tsx`, `commands.rs` `create_vente` (remise_ligne l.393) |
| 5 modes de paiement + paiement mixte (splits) | ✅ | `commands.rs:281` (`splits`), journal_caisse par split |
| Mise en attente / reprise ticket (hold/resume) | ✅ | `src/store/cart.ts`, raccourcis F7/F8 dans `POS.tsx` |
| Session de caisse (ouverture fond, clôture Z, écart) | ✅ | `commands.rs` `open_session`/`close_session`, table `sessions_caisse` |
| Impression ticket | 🔶 | HTML imprimé via popup navigateur (`receipt.ts`) **ou** ESC/POS natif — mais `print_ticket`/`print_receipt`/`print_escpos` sont **Windows-only** (PowerShell/`notepad.exe`, `commands.rs:1028-1088`) — aucun chemin macOS/Linux |
| Ouverture tiroir-caisse | ✅ (Windows) | `commands.rs:1082` `open_cash_drawer`, dépend du même chemin Windows-only que ci-dessus |
| Crédit client avec plafond bloquant | ✅ | contrôle plafond dans `create_vente`, `credit_plafond` sur `clients` |

### Catalogue & Stock
| Item | Statut | Preuve |
|---|---|---|
| CRUD article (prix, TVA, stock, catégorie, fournisseur) | ✅ | `add_article`/`update_article`/`delete_article` |
| Import/Export CSV articles | ✅ | `import_articles_csv`, export frontend `Articles.tsx` |
| Alertes stock bas / rupture | ✅ | `get_articles_stock_alerte` |
| Mouvements de stock tracés | ✅ | table `mouvements_stock`, page `MouvementsStock.tsx` |
| Contrainte UNIQUE code-barres | ✅ | `db.rs:430` — index `UNIQUE` conditionnel sur `code_barre` (corrige une erreur de l'audit initial, qui l'avait déclarée absente) |
| Inventaire physique (comptage vs théorique) | ⬜ | aucune commande/table dédiée |
| Péremption / DLC-DLUO | ✅ | table `article_lots` (numéro de lot + date de péremption + quantité par magasin), toggle `articles.suivi_lot`, écran `Articles.tsx` (case à cocher), `Stock.tsx` (dialog lots par article), page `PeremptionsStock.tsx` (alerte globale par horizon, retrait du stock) |
| Traçabilité lot / numéro de série | ✅ | `article_lots.numero_lot`, réception via `add_article_lot`, consultable via `get_article_lots` |
| FEFO automatique à la vente (consommer le lot qui périme en premier) | ⬜ | `create_vente` décrémente l'agrégat, pas un lot précis — limite connue, documentée dans le code (`commands.rs`, section "Lots / péremption") |

### Facturation / conformité fiscale
| Item | Statut | Preuve |
|---|---|---|
| Numérotation séquentielle par type/année (FA-YYYY-NNNNN) | ✅ | table `numerotation`, logique transactionnelle dans `create_vente` |
| Ventilation TVA multi-taux sur le document imprimé | ✅ | `receipt.ts:38-49` |
| ICE client sur vente B2B | ✅ | champ `clients.ice`, affiché sur reçu |
| ICE/IF/RC/Patente de l'entreprise (en-tête légal) | ✅ | 4 champs distincts (`settings.ice/if_number/rc_number/patente`), migration automatique de l'ancien `tax_number` vers `ice`, affichés sur le ticket HTML et ESC/POS (`Settings.tsx`, `receipt.ts`) |
| Document PDF archivable (facture/avoir) | ✅ | Génération A4 côté frontend (`jsPDF`, `receipt.ts` → `generateFacturePdfBase64`), écriture disque via la commande `save_document_pdf` (`commands.rs`) ; en-tête légal ICE/IF/RC/Patente, client + ICE B2B, lignes, ventilation TVA. Bouton "Générer PDF" sur chaque document dans `Ventes.tsx` (liste + détail) et juste après une vente dans le panier POS (`CartPanel.tsx`) |
| Non-suppression facture (annulation via statut) | ✅ | `annuler_vente` change le statut, pas de `delete_vente` exposé |
| Chaîne Devis → Commande → BL → Facture → Avoir avec conversion | ⬜ | seul un champ `ventes.dtype` existe (facture/devis/bl/avoir) sur l'unique table `ventes`, sans statuts de workflow, sans `parent_document_id`, sans écran de conversion, sans route dédiée dans `router.tsx` |

### Utilisateurs & sécurité
| Item | Statut | Preuve |
|---|---|---|
| Login/logout, 3 rôles, guards de route | ✅ | `AuthContext.tsx`, `router.tsx` (`allowedRoles`) |
| Hash mot de passe Argon2 + sel | ✅ | `db.rs:1-31, 513-526` |
| Permissions granulaires par module (voir/créer/modifier/exporter) | ⬜ | seul un contrôle par rôle au niveau route existe |
| Journal d'audit (qui a fait quoi) | ⬜ | aucune table/commande d'audit trouvée |
| Verrouillage session par inactivité / PIN caissier rapide | ⬜ | absent de `AuthContext.tsx` |
| CSP configurée (webview) | ⬜ | `tauri.conf.json:26` → `"csp": null` |

---

## P1 — Cycle commercial étendu
| Item | Statut | Preuve |
|---|---|---|
| CRUD Clients / Fournisseurs | ✅ | `Clients.tsx`, `Fournisseurs.tsx` |
| Achats : commande fournisseur + statuts livraison/paiement | ✅ | `create_achat`, `update_achat_status` |
| Suggestion réappro auto (seuils min/max) | ⬜ | non trouvé |
| Rapprochement facture fournisseur vs bon de réception | ⬜ | non trouvé |
| Paiement client (règlement, historique) | ✅ | `add_paiement`/`get_paiements` + page `Paiements.tsx` |
| Suivi chèques (statuts, échéances) | ✅ | table `cheques`, page `Cheques.tsx` |
| Export comptable (Sage / CSV mappable) | 🔶 | export CSV générique par écran, pas de format Sage dédié |
| Programme de fidélité configurable (paliers, récompenses) | 🔶 | points calculés en écriture (`mouvements_fidelite`), **aucune commande de lecture/config exposée**, aucune page dédiée |
| WhatsApp (envoi facture, relance) | 🔶 | lien `wa.me` pré-rempli uniquement (`Ventes.tsx:67`, `Clients.tsx:59`) — pas de Cloud API, pas d'automatisation, pas de pièce jointe |

---

## P2 — Robustesse terrain
| Item | Statut | Preuve |
|---|---|---|
| SQLite local embarqué (offline par nature) | ✅ | pas de dépendance réseau identifiée |
| Stock cohérent (agrégat unifié) | ✅ | `article_stocks` (par magasin) est désormais la seule source d'écriture ; `articles.stock` est recalculé comme `SUM(article_stocks.quantite)` à chaque mouvement via le helper `adjust_article_stock` (`commands.rs`), appelé depuis `create_vente`, `annuler_vente`, `create_achat`, `update_achat_status`, `update_article_stock`, `add_article`, `import_articles_csv`, `validate_transfert`. Corrige la survente garantie constatée dans l'audit initial |
| Multi-magasin réellement opérable (sélection du magasin actif, création de boutiques) | ⬜ | reste à construire : `add_magasin` (commande ajoutée) n'a pas d'UI ; `sessions_caisse.magasin_id` est renseigné à l'ouverture de session mais toujours avec le magasin par défaut (`default_magasin_id`) faute de sélecteur ; aucun écran ne permet de créer une 2ᵉ boutique ni de choisir un poste de caisse par magasin |
| Transfert de stock inter-magasins | 🔶 | commandes `create_transfert`/`validate_transfert` fonctionnent et sont maintenant cohérentes avec l'agrégat, mais sans UI pour créer un 2ᵉ magasin, restent inutilisables en pratique |
| Sync multi-device temps réel / résolution de conflits | ⬜ | aucune couche réseau/sync dans le projet — SQLite fichier local unique |
| Sauvegarde/export/import DB | ✅ | `backup_database`/`export_database`/`import_database` |
| Migrations SQL versionnées | ⬜ | schéma évolue via `ALTER TABLE` exécutés au démarrage, erreurs avalées (`.ok()`) |

---

## P3 — Spécialisation métier / verticaux (objet de l'audit du 2026-09-20)

Le sélecteur `Settings.tsx` → `business_type` (`standard | restaurant | mode | vrac`) n'a
**qu'une seule branche réellement câblée** : `restaurant` (gestion de table, `POS.tsx:69,377,514,578,674`).
`mode` et `vrac` sont des libellés sans effet fonctionnel.

| Vertical | Statut | Ce qui manque pour un MVP vendable |
|---|---|---|
| Supermarché / épicerie | 🔶 ~80% | DLC/péremption ✅ (2026-09-20), reste : inventaire physique, vrac/poids réellement câblé |
| Restaurant / café | 🔶 | tables + statuts existants (`tables_resto`), mais pas de KDS, pas de split bill, pas de menus composés |
| Pharmacie / parapharmacie | ⬜ ~25% | lot/péremption ✅ (2026-09-20, socle technique correct) ; reste bloquant : toujours absente du sélecteur `business_type` ; pas de notion d'ordonnance ; pas de tiers-payant AMO/mutuelle |
| Prêt-à-porter (mode) | ⬜ ~5% | table `article_variantes` (taille/couleur) existe en base mais **aucune commande Tauri ni page** ne l'exploite — schéma orphelin |
| Matériel & outils pâtisserie | ⬜ ~0% | pas de multi-prix (public/grossiste), pas de produits composés/kits, pas d'unités de vente multiples |

---

## Synthèse chiffrée (recomptée depuis le code, pas depuis l'ancien document)

| Axe | Réellement tenu |
|---|---|
| Checklist "prêt marché" (9 exigences, cf. `AUDIT_ARCHITECTURE_SENIOR_2026-09.md §7`) | 6/9 (mentions légales ICE/IF/RC/Patente + PDF facture maintenant présents ; reste bloquant : pas de bilingue FR/AR, multi-magasin toujours sans UI, spécialisation vertical) |
| Verticaux demandés couverts fonctionnellement (sur 4) | 1 (supermarché, partiel) |
| Sprints "TERMINÉ" annoncés par l'ancien plan et confirmés par le code | 2 sur 6 (bugs sécurité Sprint 1, POS de base Sprint 2) |

---

## Prochaines actions (ordre d'exécution recommandé)

1. ~~Corriger le double modèle de stock~~ ✅ fait 2026-09-20 (`adjust_article_stock`, cf. section P2 ci-dessus).
2. ~~Séparer ICE / IF / RC / Patente en champs distincts~~ ✅ fait 2026-09-20.
3. ~~Mettre en place une CI~~ ✅ fait 2026-09-20 (`.github/workflows/ci.yml`).
4. ~~Génération PDF facture/avoir~~ ✅ fait 2026-09-20 (`jsPDF` + `save_document_pdf`, cf. section Facturation ci-dessus). Reste : le PDF est généré à la demande (bouton), pas encore archivé/horodaté de façon opposable (pas de scellement, de numérotation de fichier garantie unique au-delà du nom).
5. ~~Ajouter lot + date de péremption au niveau article~~ ✅ fait 2026-09-20 (`article_lots`, cf. section Catalogue & Stock ci-dessus). Reste : pas de FEFO automatique à la vente (limite documentée dans le code), pas de notion d'ordonnance/tiers-payant pour la pharmacie.
6. Câbler `article_variantes` (commandes Tauri + UI) pour rendre le prêt-à-porter vendable.
7. Ajouter multi-prix (public/grossiste) + produits composés/kits (sert pâtisserie/matériel ET le générique déjà écrit dans `SPEC_FONCTIONNELLE_RITAJ_RETAIL_MAROC.md §3.1`).
8. Construire l'UI multi-magasin (sélection du magasin actif à l'ouverture de session, écran de création de boutique sur `add_magasin`) pour que le travail du point 1 devienne utilisable, pas seulement sûr.

Détail complet des constats et recommandations : voir `AUDIT_ARCHITECTURE_SENIOR_2026-09.md`.
