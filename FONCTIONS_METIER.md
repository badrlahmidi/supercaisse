# Inventaire des fonctions métier — SuperCaisse / RitajPOS

> Vue business (pas de références code ici — pour la preuve technique fichier:ligne de
> chaque ligne, voir `ROADMAP_STATUS.md`). Objectif : donner une liste actionnable pour
> prioriser le prochain cycle de travail, fonction par fonction.
>
> **Légende** : ✅ Inclus et utilisable tel quel · 🔶 Inclus mais incomplet / à risque /
> comportement à corriger · ⬜ Manquant.
> Dernière mise à jour : 2026-09-20 (recomptée depuis `ROADMAP_STATUS.md` du même jour).
>
> **Mise à jour du 2026-09-21** : stock multi-magasin, en-tête légal DGI (ICE/IF/RC/Patente), PDF
> facture archivable, traçabilité lot/péremption, détection doublon code-barres, déclinaisons
> taille/couleur, multi-prix et produits composés/kits sont passés en ✅. UI multi-boutique
> (page Boutiques CRUD, sélection magasin à l'ouverture, transfert inter-boutiques, vue stock
> par magasin) également passée en ✅. Impression/tiroir-caisse cross-platform (→ ✅).
> Programme de fidélité (→ ✅ : colonne + historique dans Clients, config dans Paramètres).
> Sélecteur secteur d'activité (→ ✅ : options "Mode"/"Vrac" retirées). Bug persistance
> settings corrigé. Score final Phase 1 : **56 ✅ / 2 🔶 / 27 ⬜**.
>
> **Mise à jour du 2026-09-21 (Phase 3)** : Étiquettes code-barres imprimables (→ ✅ : sélection
> d'articles + planches PDF jsPDF 3×10 par page). Rapports détaillés marge/TVA/rotation
> (→ ✅ : page Rapports avec 5 KPIs, graphique CA/jour, top articles, rotation stock, ventilation
> paiements). Export PDF des rapports (→ ✅ : jsPDF A4 avec KPIs + top articles + ventilation).
> PIN rapide de changement de caissier (→ ✅ : numpad dans l'écran de verrouillage, set_user_pin
> dans Settings, login_pin en backend). Inventaire physique comptage vs théorique
> (→ ✅ : page Inventaire avec création par magasin, comptage article par article, validation
> avec application des écarts au stock). Score Phase 3 : **66 ✅ / 2 🔶 / 17 ⬜**.

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
| Rapport X (intermédiaire, en cours de session) | ✅ | Dialog dans POS avec CA, nb ventes, articles vendus, ventilation par mode de paiement, remises, annulations |
| Impression ticket | ✅ | Cross-platform : ESC/POS natif sur Windows (spooler) / Linux-Mac (lp/device), fallback navigateur (window.print) sur toute plateforme |
| Ouverture tiroir-caisse automatique | ✅ | Cross-platform via ESC/POS (même canal que l'impression ticket) |
| Ticket sans papier (email/SMS) | ⬜ | |
| Écran client secondaire (double afficheur) | ⬜ | |
| Balance connectée (produits au poids) | ⬜ | Aucune intégration série/USB de balance implémentée |
| Vente par variante (taille/couleur) | ✅ | Sélecteur à l'écran, scan direct, stock décompté sur la bonne déclinaison |
| Gestion de table / salle (mode restaurant) | ✅ | Actif seulement si secteur = "Restaurant" dans les réglages |
| Écran cuisine (KDS), split bill, pourboire | ⬜ | |

## 2. Catalogue produits

| Fonction | Statut | Note |
|---|---|---|
| Fiche produit (prix, TVA, stock, catégorie, fournisseur, image) | ✅ | |
| Import/export en masse (CSV) | ✅ | |
| Désactivation d'article sans le supprimer | ✅ | |
| Détection doublon code-barres | ✅ | Contrainte unique en base (corrige une erreur de l'audit initial, qui l'avait déclarée absente) |
| Génération automatique de code-barres interne | ✅ | Format INT-{id:06} auto-généré si vide à la création |
| Étiquettes code-barres imprimables (planches PDF) | ✅ | Sélection dans la liste Articles, génération planches PDF jsPDF (3×10 étiquettes/page, désignation + code + prix) |
| Prix multiple (public / grossiste) | ✅ | Prix grossiste optionnel par article, bascule d'un clic sur la ligne du panier caisse |
| Produits composés / kits | ✅ | Un kit vend normalement au scan ; le stock de chaque composant est décrémenté automatiquement au prorata |
| Variantes taille/couleur avec stock dédié | ✅ | Création/gestion et vente à la caisse toutes les deux opérationnelles |
| Traçabilité lot / date de péremption | ✅ | Case à cocher par article, réception de lot avec numéro + date, écran d'alerte dédié (page Péremptions) |

## 3. Stock

| Fonction | Statut | Note |
|---|---|---|
| Ajustement de stock avec motif | ✅ | |
| Historique des mouvements de stock | ✅ | |
| Alertes seuil bas / rupture | ✅ | |
| Valorisation du stock (affichage) | ✅ | |
| Alertes péremption avec horizon paramétrable (7/15/30/90 jours) | ✅ | Écran dédié, retrait du stock en un clic (péremption/casse) |
| Inventaire physique (comptage vs théorique) | ✅ | Page Inventaire : création par magasin, comptage article par article, progression, validation avec application écarts au stock réel + mouvements_stock |
| Suggestion de réapprovisionnement automatique | ⬜ | |
| Cohérence du stock (pas de survente inter-boutique) | ✅ | Corrigé : le stock global affiché est maintenant un agrégat toujours recalculé depuis le détail par boutique, plus deux compteurs déconnectés |
| Utilisation réelle du multi-boutique (choisir sa boutique, en créer une 2ᵉ) | ✅ | Page Boutiques (CRUD), sélection de boutique à l'ouverture de session caisse, vue du stock par boutique |
| Transfert de stock entre boutiques | ✅ | Interface de transfert inter-boutiques avec validation, depuis la page Boutiques |

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
| Relevé de compte client exportable | ✅ | Dialog avec historique ventes + paiements, export CSV |
| Programme de fidélité (points) | ✅ | Points calculés à la vente, affichés dans la page Clients (colonne + historique détaillé), configurables dans Paramètres (activer/désactiver, ratio DH/point, valeur point) ; reste ⬜ : paliers/récompenses automatiques |
| Segmentation client / campagnes marketing | ⬜ | |
| Relance / envoi facture par WhatsApp | 🔶 | Ouvre un lien WhatsApp pré-rempli avec détails facture — pas d'envoi automatique, pas de pièce jointe PDF (limitation WhatsApp Web) |

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
| Journal d'audit (qui a fait quoi, quand) | ✅ | Table audit_log + logging annulations/modifications/suppressions/paramètres + page admin filtrable + export CSV |
| Verrouillage automatique après inactivité | ✅ | Configurable en secondes dans Paramètres > Système, overlay de déverrouillage par mot de passe |
| PIN rapide de changement de caissier | ✅ | Numpad dans l'écran de verrouillage (PIN 4 chiffres), configuration dans Paramètres > Utilisateurs, login_pin backend |

## 9. Rapports & pilotage

| Fonction | Statut | Note |
|---|---|---|
| Tableau de bord (CA, articles, alertes stock, crédit clients) | ✅ | 5 indicateurs + graphique 7 jours |
| Rapports détaillés (marge, TVA collectée, rotation stock) | ✅ | Page Rapports avec 5 KPIs (CA, marge, TVA, nb ventes, remises), graphique CA/jour, top 10 articles, rotation stock top 20, ventilation par mode de paiement |
| Export PDF/Excel des rapports | ✅ | Export PDF (jsPDF A4, KPIs + top articles + ventilation) et CSV depuis la page Rapports |
| Comparateur multi-boutiques | ⬜ | |

## 10. Matériel & intégrations physiques

| Fonction | Statut | Note |
|---|---|---|
| Douchette code-barres (USB) | ✅ | Fonctionne nativement (focus + Entrée) |
| Imprimante ticket ESC/POS | ✅ | Windows (spooler COPY /B), Linux/Mac (lp -o raw ou /dev/usb/lpN) |
| Tiroir-caisse | ✅ | Cross-platform via ESC/POS (même canal) |
| Terminal de paiement (TPE) intégré | ⬜ | Saisie manuelle du montant seulement |
| Balance connectée | ⬜ | |

## 11. Paramétrage / Administration

| Fonction | Statut | Note |
|---|---|---|
| Réglages boutique (nom, adresse, téléphone, TVA défaut) | ✅ | |
| Sélecteur de secteur d'activité | ✅ | Deux secteurs fonctionnels (Standard, Restaurant) ; les options "Mode" et "Vrac" retirées car sans comportement réel — les variantes taille/couleur fonctionnent dans tous les modes |
| Sauvegarde / export / import de la base | ✅ | |
| Personnalisation modèles de documents imprimés | 🔶 | Pied de page ticket configurable ; pas de logo, pas de couleurs/polices personnalisables, pas d'éditeur de mise en page A4/étiquette |
| Interface bilingue FR/AR | ⬜ | |

---

## Synthèse par volume

| | Nombre de fonctions |
|---|---|
| ✅ Inclus et utilisable | 66 |
| 🔶 Inclus mais à corriger/compléter | 2 |
| ⬜ Manquant | 17 |

*(Mis à jour 2026-09-21 : impression, tiroir-caisse et ESC/POS passent en ✅ (cross-platform Windows/Linux/Mac) ; fidélité ✅ (colonne + historique dans Clients, config dans Paramètres) ; secteur ✅ (options "Mode"/"Vrac" sans effet retirées) ; bug persistance paramètres corrigé ; doublon footer ESC/POS corrigé. Recompté : 56 ✅, 2 🔶, 27 ⬜.)*

## Les "🔶 à améliorer" les plus prioritaires (risque business le plus élevé)

1. ~~Stock isolé par boutique~~ ✅ corrigé le 2026-09-20 — reste une action UI (item 3 ci-dessous), plus un risque de survente.
2. ~~ICE/IF/RC/Patente de l'entreprise~~ ✅ corrigé le 2026-09-20.
3. ~~Impression Windows-only~~ ✅ corrigé le 2026-09-21 — cross-platform : Windows (spooler), Linux/Mac (lp / device direct), fallback navigateur.
4. ~~Sélecteur de secteur d'activité cosmétique~~ ✅ corrigé le 2026-09-21 — options "Mode"/"Vrac" sans comportement retirées.
5. ~~Fidélité invisible côté gérant~~ ✅ corrigé le 2026-09-21 — colonne Points + historique par client + configuration complète dans Paramètres.
6. ~~Utilisation réelle du multi-boutique~~ ✅ corrigé le 2026-09-21 — page Boutiques (CRUD + transferts + vue stock par magasin), sélection de boutique à l'ouverture de session caisse.

## Les fonctions manquantes qui structurent le plus la suite

- ~~Traçabilité lot/péremption~~ ✅ fait 2026-09-20 (sert à la fois supermarché et pharmacie)
- ~~Document PDF archivable~~ ✅ fait 2026-09-20 (condition d'entrée pour toute facturation professionnelle sérieuse)
- ~~Variantes taille/couleur vendables depuis la caisse~~ ✅ fait 2026-09-21
- ~~Multi-prix + produits composés~~ ✅ fait 2026-09-21 (condition d'entrée pour le matériel/pâtisserie)
- Notion d'ordonnance + tiers-payant AMO/mutuelle (dernier verrou spécifique à la pharmacie, au-delà du socle technique lot/péremption)

---

## Comment traiter cette liste maintenant

Deux familles de décisions différentes, à ne pas mélanger :

- **Les ⬜ manquants** : rien à détecter, c'est un pur exercice de priorisation/backlog — pas besoin de QA, juste une décision "on le fait maintenant ou plus tard".
- **Les 🔶 à améliorer** : il reste 2 lignes 🔶 — WhatsApp (limitation wa.me sans API Business, pas de pièce jointe) et personnalisation documents (pied de page configurable, mais pas de logo/couleurs/éditeur de template). Ce sont des limitations acceptables à ce stade.

🔶 corrigées dans ce cycle : impression cross-platform (→ ✅ Windows + Linux/Mac), tiroir-caisse cross-platform (→ ✅), fidélité (→ ✅, historique + config visible), secteur d'activité (→ ✅, labels cosmétiques retirés), bug persistance paramètres corrigé, footer ESC/POS dédoublonné.
