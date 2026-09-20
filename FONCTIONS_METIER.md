# Inventaire des fonctions métier — SuperCaisse / RitajPOS

> Vue business (pas de références code ici — pour la preuve technique fichier:ligne de
> chaque ligne, voir `ROADMAP_STATUS.md`). Objectif : donner une liste actionnable pour
> prioriser le prochain cycle de travail, fonction par fonction.
>
> **Légende** : ✅ Inclus et utilisable tel quel · 🔶 Inclus mais incomplet / à risque /
> comportement à corriger · ⬜ Manquant.
> Dernière mise à jour : 2026-09-20 (recomptée depuis `ROADMAP_STATUS.md` du même jour).
>
> **Mise à jour du soir** : stock multi-magasin, en-tête légal DGI (ICE/IF/RC/Patente), PDF
> facture archivable et détection doublon code-barres sont passés en ✅ — détail dans
> `ROADMAP_STATUS.md`.

---

## 1. Vente / Point de vente (caisse)

| Fonction | Statut | Note |
|---|---|---|
| Recherche produit (texte + scan code-barres) | ✅ | Fonctionne, focus conservé après scan |
| Panier : ajout, quantité, suppression, vidage | ✅ | |
| Remise ligne + remise document | ✅ | |
| Note libre par ligne panier | ✅ | |
| Paiement espèces avec rendu monnaie | ✅ | |
| Paiement carte / chèque / virement | ✅ | Saisie manuelle, pas d'intégration TPE |
| Paiement mixte (plusieurs modes sur un même ticket) | ✅ | |
| Vente à crédit avec plafond client bloquant | ✅ | |
| Mise en attente / reprise d'un ticket | ✅ | |
| Raccourcis clavier caisse (F1–F8, Echap) | ✅ | |
| Clôture de caisse (Z) avec écart théorique/réel | ✅ | |
| Rapport X (intermédiaire, en cours de session) | ⬜ | Seul le Z de clôture existe |
| Impression ticket | 🔶 | Fonctionne mais **uniquement sur poste Windows** (dépend de PowerShell/Notepad en coulisses) — à valider ou corriger avant tout déploiement Mac/Linux |
| Ouverture tiroir-caisse automatique | 🔶 | Techniquement présent, mais hérite de la même limite Windows-only |
| Ticket sans papier (email/SMS) | ⬜ | |
| Écran client secondaire (double afficheur) | ⬜ | |
| Balance connectée (produits au poids) | ⬜ | Le "mode vrac" existe dans les réglages mais ne déclenche aucun comportement |
| Vente par variante (taille/couleur) | ⬜ | La caisse ne sait vendre qu'un article unique, pas une déclinaison |
| Gestion de table / salle (mode restaurant) | ✅ | Actif seulement si secteur = "Restaurant" dans les réglages |
| Écran cuisine (KDS), split bill, pourboire | ⬜ | |

## 2. Catalogue produits

| Fonction | Statut | Note |
|---|---|---|
| Fiche produit (prix, TVA, stock, catégorie, fournisseur, image) | ✅ | |
| Import/export en masse (CSV) | ✅ | |
| Désactivation d'article sans le supprimer | ✅ | |
| Détection doublon code-barres | ✅ | Contrainte unique en base (corrige une erreur de l'audit initial, qui l'avait déclarée absente) |
| Génération automatique de code-barres interne | ⬜ | |
| Étiquettes code-barres imprimables (planches PDF) | ⬜ | |
| Prix multiple (public / grossiste / promo) | ⬜ | Un seul prix de vente par article |
| Produits composés / kits | ⬜ | |
| Variantes taille/couleur avec stock dédié | ⬜ | La structure existe en base mais rien ne permet de la créer ou de la vendre |
| Traçabilité lot / date de péremption | ⬜ | Aucun champ sur la fiche produit |

## 3. Stock

| Fonction | Statut | Note |
|---|---|---|
| Ajustement de stock avec motif | ✅ | |
| Historique des mouvements de stock | ✅ | |
| Alertes seuil bas / rupture | ✅ | |
| Valorisation du stock (affichage) | ✅ | |
| Inventaire physique (comptage vs théorique) | ⬜ | |
| Suggestion de réapprovisionnement automatique | ⬜ | |
| Cohérence du stock (pas de survente inter-boutique) | ✅ | Corrigé : le stock global affiché est maintenant un agrégat toujours recalculé depuis le détail par boutique, plus deux compteurs déconnectés |
| Utilisation réelle du multi-boutique (choisir sa boutique, en créer une 2ᵉ) | ⬜ | La donnée est fiable mais il manque encore l'écran pour créer une boutique et choisir sur quel poste on travaille |
| Transfert de stock entre boutiques | 🔶 | La fonction est fiable côté données, mais reste inutilisable sans écran pour créer une 2ᵉ boutique |

## 4. Achats & Fournisseurs

| Fonction | Statut | Note |
|---|---|---|
| Fiche fournisseur (coordonnées, ICE) | ✅ | |
| Commande fournisseur avec lignes | ✅ | |
| Statut livraison / paiement de l'achat | ✅ | |
| Suggestion de réappro basée sur seuils min/max | ⬜ | |
| Comparaison prix entre fournisseurs | ⬜ | |
| Rapprochement facture fournisseur ↔ bon de réception | ⬜ | |

## 5. Clients & Partenaires

| Fonction | Statut | Note |
|---|---|---|
| Fiche client (coordonnées, ICE, plafond crédit) | ✅ | |
| Historique et enregistrement des paiements client | ✅ | |
| Relevé de compte client exportable | ⬜ | |
| Programme de fidélité (points) | 🔶 | Les points se calculent en arrière-plan à chaque vente, mais rien ne permet de les consulter, configurer des paliers/récompenses, ou les afficher au client — fonctionnalité invisible pour le gérant |
| Segmentation client / campagnes marketing | ⬜ | |
| Relance / envoi facture par WhatsApp | 🔶 | Ouvre un lien WhatsApp pré-rempli manuellement — pas d'envoi automatique, pas de pièce jointe |

## 6. Paiements & Trésorerie

| Fonction | Statut | Note |
|---|---|---|
| Journal de caisse quotidien | ✅ | |
| Suivi des chèques (statuts, échéances) | ✅ | |
| Export comptable CSV | ✅ | Générique, pas de format Sage dédié |
| Multi-caisse avec vue trésorerie consolidée | ⬜ | |

## 7. Facturation & conformité fiscale marocaine

| Fonction | Statut | Note |
|---|---|---|
| Numérotation séquentielle par type de document/année | ✅ | |
| TVA multi-taux avec ventilation sur le ticket | ✅ | |
| ICE du client affiché sur vente B2B | ✅ | |
| ICE / IF / RC / Patente de l'entreprise sur le document | ✅ | 4 champs distincts, saisis dans Réglages et affichés sur le ticket |
| Document PDF archivable (facture, avoir) | ✅ | Génération PDF A4 avec mentions légales, disponible depuis la liste des ventes et juste après l'encaissement |
| Non-suppression d'une facture validée | ✅ | Annulation par changement de statut uniquement |
| Chaîne Devis → Commande → BL → Facture → Avoir | ⬜ | N'existe pas comme parcours utilisable ; seul un indicateur technique sans écran dédié |
| Veille facturation électronique DGI | ⬜ | Non commencé (acceptable à ce stade du calendrier réglementaire marocain) |

## 8. Utilisateurs, rôles & sécurité

| Fonction | Statut | Note |
|---|---|---|
| Connexion / déconnexion, 3 rôles (admin/manager/caissier) | ✅ | |
| Restriction des écrans par rôle | ✅ | |
| Mot de passe sécurisé (hash + sel) | ✅ | |
| Permissions fines par module (voir/créer/modifier/exporter) | ⬜ | |
| Journal d'audit (qui a fait quoi, quand) | ⬜ | Pas de traçabilité des annulations, remises exceptionnelles, changements de prix |
| Verrouillage automatique après inactivité | ⬜ | |
| PIN rapide de changement de caissier | ⬜ | |

## 9. Rapports & pilotage

| Fonction | Statut | Note |
|---|---|---|
| Tableau de bord (CA, articles, alertes stock, crédit clients) | ✅ | 5 indicateurs + graphique 7 jours |
| Rapports détaillés (marge, TVA collectée, rotation stock) | ⬜ | |
| Export PDF/Excel des rapports | ⬜ | Export CSV seulement, et pas sur tous les écrans |
| Comparateur multi-boutiques | ⬜ | |

## 10. Matériel & intégrations physiques

| Fonction | Statut | Note |
|---|---|---|
| Douchette code-barres (USB) | ✅ | Fonctionne nativement (focus + Entrée) |
| Imprimante ticket ESC/POS | 🔶 | Windows uniquement |
| Tiroir-caisse | 🔶 | Dépend de l'imprimante, donc même limite |
| Terminal de paiement (TPE) intégré | ⬜ | Saisie manuelle du montant seulement |
| Balance connectée | ⬜ | |

## 11. Paramétrage / Administration

| Fonction | Statut | Note |
|---|---|---|
| Réglages boutique (nom, adresse, téléphone, TVA défaut) | ✅ | |
| Sélecteur de secteur d'activité | 🔶 | Existe dans l'écran mais **un seul secteur sur quatre proposés a un effet réel** (Restaurant) — "Mode" et "Vrac/Boucherie" sont des libellés sans comportement associé, "Pharmacie" n'est même pas proposé |
| Sauvegarde / export / import de la base | ✅ | |
| Personnalisation modèles de documents imprimés | 🔶 | Aperçu ticket configurable, mais pas de vraie mise en page facture A4/étiquette |
| Interface bilingue FR/AR | ⬜ | |

---

## Synthèse par volume

| | Nombre de fonctions |
|---|---|
| ✅ Inclus et utilisable | 34 |
| 🔶 Inclus mais à corriger/compléter | 13 |
| ⬜ Manquant | 31 |

*(Mis à jour après les correctifs du 2026-09-20 : doublon code-barres, en-tête légal DGI et cohérence stock passés en ✅ ; ce dernier point a fait apparaître une nouvelle ligne ⬜ distincte — "utilisation réelle du multi-boutique" — puisque la donnée est maintenant fiable mais l'écran pour l'exploiter n'existe toujours pas.)*

## Les "🔶 à améliorer" les plus prioritaires (risque business le plus élevé)

1. ~~Stock isolé par boutique~~ ✅ corrigé le 2026-09-20 — reste une action UI (item 3 ci-dessous), plus un risque de survente.
2. ~~ICE/IF/RC/Patente de l'entreprise~~ ✅ corrigé le 2026-09-20.
3. **Impression Windows-only** — décide si on assume "Windows uniquement" comme contrainte produit ou si on corrige pour élargir le marché adressable.
4. **Sélecteur de secteur d'activité cosmétique** — soit on retire "Mode"/"Vrac" de la liste tant qu'ils ne font rien (évite de tromper le client au moment de la configuration), soit on les câble.
5. **Fidélité invisible côté gérant** — la donnée existe et s'accumule déjà en base ; l'écran de consultation/config est un développement relativement court par rapport à sa valeur perçue commerciale.
6. **Utilisation réelle du multi-boutique** — la donnée est maintenant fiable (point 1), mais il manque l'écran pour créer une 2ᵉ boutique et choisir son poste de caisse.

## Les fonctions manquantes qui structurent le plus la suite

- Traçabilité lot/péremption (sert à la fois supermarché et pharmacie)
- Variantes taille/couleur exploitables en vente (condition d'entrée pour le prêt-à-porter)
- Multi-prix + produits composés (condition d'entrée pour le matériel/pâtisserie)
- Document PDF archivable (condition d'entrée pour toute facturation professionnelle sérieuse)

---

## Comment traiter cette liste maintenant

Deux familles de décisions différentes, à ne pas mélanger :

- **Les ⬜ manquants** : rien à détecter, c'est un pur exercice de priorisation/backlog — pas besoin de QA, juste une décision "on le fait maintenant ou plus tard".
- **Les 🔶 à améliorer** : c'est là qu'un plan de Q/A cible vraiment quelque chose d'utile, parce que le mot "à améliorer" cache des réalités très différentes selon la ligne (limite d'environnement type impression Windows, écran manquant type fidélité). Un scénario par ligne 🔶 restante (13 scénarios) transforme chaque "à améliorer" en spécification précise de correction avant de chiffrer le développement — plutôt qu'un audit généraliste qui re-testerait aussi les ✅ déjà confirmés.

Le stock multi-boutique et l'en-tête légal DGI (les deux points qui bloquaient le plus la suite) sont traités. Prochaines cibles naturelles : génération PDF facture, écran multi-boutique (créer une 2ᵉ boutique, choisir son poste), traçabilité lot/péremption (sert supermarché ET pharmacie), ou le plan de Q/A sur les 13 lignes 🔶 restantes.
