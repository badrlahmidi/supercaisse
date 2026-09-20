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
| Contrainte UNIQUE code-barres | ⬜ | aucune contrainte `UNIQUE` sur `articles.code_barre` dans `db.rs` — doublon silencieux possible |
| Inventaire physique (comptage vs théorique) | ⬜ | aucune commande/table dédiée |
| Péremption / DLC-DLUO | ⬜ | aucune colonne date d'expiration sur `articles` |
| Traçabilité lot / numéro de série | ⬜ | aucune table/colonne lot |

### Facturation / conformité fiscale
| Item | Statut | Preuve |
|---|---|---|
| Numérotation séquentielle par type/année (FA-YYYY-NNNNN) | ✅ | table `numerotation`, logique transactionnelle dans `create_vente` |
| Ventilation TVA multi-taux sur le document imprimé | ✅ | `receipt.ts:38-49` |
| ICE client sur vente B2B | ✅ | champ `clients.ice`, affiché sur reçu |
| ICE/IF/RC/Patente de l'entreprise (en-tête légal) | 🔶 | un seul champ texte libre `settings.tax_number` ("ICE/IF") ; **aucun champ RC ni Patente** |
| Document PDF archivable (facture/avoir) | ⬜ | aucune génération PDF dans le projet (`grep -ri pdf` négatif) |
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
| Multi-magasin avec stock isolé par magasin | ⬜ **cassé** | deux modèles de stock coexistent et sont déconnectés : `create_vente` décrémente la colonne globale `articles.stock` (`commands.rs:401-402`) sans jamais toucher `article_stocks` (par magasin), qui n'est lu/écrit que par `create_transfert`/`validate_transfert`. Une vente au magasin B ne reflète jamais sur le stock que voit le magasin A → survente garantie dès 2 points de vente |
| Transfert de stock inter-magasins | 🔶 | commandes `create_transfert`/`validate_transfert` existent, mais ne servent à rien tant que le point ci-dessus n'est pas corrigé |
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
| Supermarché / épicerie | 🔶 ~70% | DLC/péremption, inventaire physique, vrac/poids réellement câblé |
| Restaurant / café | 🔶 | tables + statuts existants (`tables_resto`), mais pas de KDS, pas de split bill, pas de menus composés |
| Pharmacie / parapharmacie | ⬜ ~10% | absente du sélecteur ; aucune traçabilité lot/péremption (obligation réglementaire, pas confort) ; pas de notion d'ordonnance ; pas de tiers-payant AMO/mutuelle |
| Prêt-à-porter (mode) | ⬜ ~5% | table `article_variantes` (taille/couleur) existe en base mais **aucune commande Tauri ni page** ne l'exploite — schéma orphelin |
| Matériel & outils pâtisserie | ⬜ ~0% | pas de multi-prix (public/grossiste), pas de produits composés/kits, pas d'unités de vente multiples |

---

## Synthèse chiffrée (recomptée depuis le code, pas depuis l'ancien document)

| Axe | Réellement tenu |
|---|---|
| Checklist "prêt marché" (9 exigences, cf. `AUDIT_ARCHITECTURE_SENIOR_2026-09.md §7`) | 4/9 |
| Verticaux demandés couverts fonctionnellement (sur 4) | 1 (supermarché, partiel) |
| Sprints "TERMINÉ" annoncés par l'ancien plan et confirmés par le code | 2 sur 6 (bugs sécurité Sprint 1, POS de base Sprint 2) |

---

## Prochaines actions (ordre d'exécution recommandé)

1. Corriger le double modèle de stock (`articles.stock` vs `article_stocks`) — bloquant pour toute promesse multi-magasin.
2. Séparer ICE / IF / RC / Patente en champs distincts + générer un vrai PDF facture.
3. Mettre en place une CI (`tsc --noEmit`, `oxlint`, `vitest run`, `cargo check`) pour que ce document reste vérifiable automatiquement et ne redevienne pas un `MEGA_PLAN_REFONTE.md` bis.
4. Ajouter lot + date de péremption au niveau article (sert supermarché ET pharmacie).
5. Câbler `article_variantes` (commandes Tauri + UI) pour rendre le prêt-à-porter vendable.
6. Ajouter multi-prix (public/grossiste) + produits composés/kits (sert pâtisserie/matériel ET le générique déjà écrit dans `SPEC_FONCTIONNELLE_RITAJ_RETAIL_MAROC.md §3.1`).

Détail complet des constats et recommandations : voir `AUDIT_ARCHITECTURE_SENIOR_2026-09.md`.
