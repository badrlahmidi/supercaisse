# SPEC FONCTIONNELLE — RitajPOS Retail Management (Maroc)
## Cahier des charges fonctionnel complet pour une solution all-in-one prête marché

> Document de référence à réutiliser dans L'Orchestrateur pour cadrer le scope produit, prioriser les sprints, et vérifier qu'aucune fonctionnalité "table stakes" du marché marocain n'est oubliée. Organisé par flux métier quotidien, pas par couche technique.
>
> **Nature du document : cible/vision, pas état d'avancement.** Ce cahier des charges décrit ce
> que le produit doit couvrir, pas ce qui est implémenté aujourd'hui. Pour l'état réel vérifié
> dans le code, voir `ROADMAP_STATUS.md` (suivi d'avancement module par module) et
> `AUDIT_ARCHITECTURE_SENIOR_2026-09.md` (audit architecture + conformité + couverture par
> vertical) à la racine du dépôt.

---

## 0. Principe directeur

Un logiciel de caisse/gestion marocain "prêt marché" doit couvrir **3 couches indissociables** :
1. **Le flux commercial complet** : Devis → Commande → Bon de Livraison → Facture → Paiement → Avoir/Retour (chaque document peut se convertir dans le suivant sans ressaisie)
2. **La conformité fiscale marocaine** (DGI, TVA, ICE) — non négociable pour vendre légalement
3. **La résilience terrain** (offline-first, multi-caisse, matériel physique) — le vrai facteur différenciant vs les solutions cloud pures qui échouent en cas de coupure internet/électricité

---

## 1. Flux Ventes (cœur du produit)

### 1.1 Chaîne documentaire
| Document | Rôle | Convertible vers |
|---|---|---|
| **Devis** | Proposition commerciale non engageante, validité limitée dans le temps | Commande ou Facture directe |
| **Commande client** | Engagement client avant expédition/prestation | Bon de Livraison |
| **Bon de Livraison (BL)** | Preuve de livraison physique, décrémente le stock | Facture |
| **Facture** | Document fiscal définitif, numérotation légale séquentielle | Avoir (si retour) |
| **Avoir / Bon de Retour** | Annulation partielle/totale, ré-incrémente le stock | — |
| **Ticket de caisse (POS)** | Vente comptant instantanée, équivalent facture simplifiée | Facture sur demande client |

**Règles transverses obligatoires :**
- Chaque conversion **copie les lignes sans ressaisie**, en gardant la traçabilité du document d'origine (chaînage `parent_document_id`)
- Statuts par document : Brouillon → Envoyé → Validé/Accepté → Converti / Annulé
- Numérotation séquentielle **sans trou** par type de document et par année fiscale (exigence DGI) : `FA-2026-00001`
- Chaque document doit pouvoir être **exporté en PDF** et **envoyé par email/WhatsApp** directement depuis l'écran
- Remises : au niveau ligne (%, montant fixe) ET au niveau document global
- Multi-devise en option (MAD par défaut, EUR/USD pour clients export) avec taux de change figé à la date du document

### 1.2 Point de Vente (POS) — flux quotidien caissier
- Ouverture de caisse : saisie fond de caisse initial, horodatage, caissier identifié
- Recherche produit : scan code-barres (douchette), recherche texte, navigation par catégories
- Panier : ajout/retrait, modification quantité, remise ligne, note ligne, gestion articles au poids (balance connectée)
- Client (option) : rattachement à un client existant ou vente anonyme ; compte client / crédit client
- Paiement : espèces (avec calcul rendu monnaie), carte bancaire (TPE intégré ou manuel), paiement mixte (split), compte client, chèque, bons d'achat/avoirs
- Mise en attente d'un ticket (pause vente en cours, reprise plus tard) — essentiel si interruption client
- Impression ticket ESC/POS (imprimante 58/80mm) + option ticket sans papier (email/SMS)
- Ouverture tiroir-caisse automatique à l'encaissement
- Retour/annulation ticket avec motif obligatoire, traçabilité caissier
- Clôture de caisse (Z de caisse) : comptage espèces, écart théorique/réel, rapport de session signé

### 1.3 Spécificités sectorielles (à activer selon vertical)
- **Retail/Superette** : gestion vrac/poids, code-barres internes pour produits non étiquetés, gestion péremption (DLC/DLUO)
- **Restaurant/Café/Chicha** : gestion de tables/salle, KDS (Kitchen Display System), split bill, pourboire, menu composé/modificateurs, prise de commande tablette serveur
- **Boutique mode/multi-variantes** : gestion taille/couleur (déclinaisons), code-barres par variante

---

## 2. Flux Achats (miroir des ventes)
- **Commande fournisseur** → **Bon de Réception** (incrémente stock) → **Facture fournisseur** → **Avoir fournisseur**
- Suggestion automatique de réapprovisionnement basée sur seuils de stock min/max
- Comparaison prix fournisseurs multiples pour un même produit
- Rapprochement facture fournisseur vs bon de réception (contrôle des écarts qté/prix)
- Gestion des délais de paiement fournisseur et échéancier

---

## 3. Catalogue & Stock

### 3.1 Catalogue produits
- Fiche produit : nom, référence interne, code-barres (EAN13/Code128), catégorie, image, prix HT/TTC, taux de TVA, unité de vente, prix d'achat
- Produits composés/kits (ex. menu restaurant = somme d'ingrédients avec décrément stock des composants)
- Variantes/déclinaisons (taille, couleur, poids)
- Gestion multi-prix : prix public, prix grossiste, prix promo, prix par point de vente
- Import/export catalogue en masse (Excel/CSV)
- Étiquettes code-barres à imprimer (génération PDF planches d'étiquettes)

### 3.2 Stock
- Multi-emplacement (plusieurs boutiques/entrepôts), transfert de stock inter-magasins
- Mouvements de stock tracés (vente, achat, transfert, ajustement, casse/perte, retour)
- Inventaire physique (comptage) avec écart calculé vs théorique
- Alertes seuil bas / rupture, alertes péremption proche
- Valorisation stock (FIFO/coût moyen pondéré) pour le bilan comptable
- Traçabilité par lot/numéro de série (optionnel, pharma/électronique)

---

## 4. Partenaires (Clients & Fournisseurs)
- Fiche client : coordonnées, ICE (si professionnel), plafond crédit, historique achats/paiements, solde compte courant
- Fiche fournisseur : coordonnées, ICE, RIB, conditions de paiement, historique commandes
- Programme de fidélité : points, paliers, cartes de fidélité (physique/dématérialisée), récompenses automatiques
- Segmentation client pour campagnes marketing (WhatsApp Business, SMS)
- Relevé de compte client exportable (PDF)

---

## 5. Paiements & Trésorerie
- Multi-modes de paiement (espèces, carte, chèque, virement, compte client, bons d'achat)
- Rapprochement paiement ↔ facture (paiement partiel, échelonné, avoir imputé)
- Journal de caisse quotidien consolidé (toutes sessions/caissiers/magasins)
- Suivi des chèques (encaissé/en attente/impayé) et relances
- Export comptable (format compatible logiciels comptables marocains : Sage, sinon CSV générique mappable)
- Multi-caisse simultanée avec vue consolidée trésorerie groupe

---

## 6. Conformité fiscale marocaine (bloc critique "prêt marché")
- **Facturation électronique DGI** : conformité au calendrier de généralisation de la facturation électronique au Maroc (intégration API DGI dès que le cahier des charges officiel est publié pour le segment concerné — veille réglementaire nécessaire)
- Gestion ICE (Identifiant Commun de l'Entreprise) obligatoire sur factures B2B
- Gestion IF (Identifiant Fiscal), RC (Registre de Commerce), Patente sur l'en-tête entreprise
- TVA multi-taux (0%, 7%, 10%, 14%, 20%) paramétrable par produit
- Régime auto-entrepreneur vs société (formats de facture différents)
- Archivage légal des documents (durée de conservation réglementaire)
- Numérotation légale inaltérable (pas de suppression physique de facture — annulation via avoir uniquement)
- Facture normalisée conforme modèle DGI (mentions obligatoires : ICE, IF, RC, TVA détaillée)

---

## 7. Utilisateurs, Rôles & Sécurité (RBAC)
- Rôles types : Administrateur, Gérant/Manager multi-boutique, Caissier, Comptable, Vendeur/Serveur, Magasinier
- Permissions granulaires par module (voir/créer/modifier/supprimer/exporter) et par magasin
- Journal d'audit : qui a fait quoi, quand (annulations, remises exceptionnelles, modifications de prix)
- Authentification PIN rapide pour changement de caissier sur poste partagé (écran tactile)
- Session verrouillée après inactivité (sécurité poste en libre accès)
- Gestion multi-établissement avec cloisonnement des données par franchise/succursale si nécessaire

---

## 8. Rapports & Pilotage
- Tableau de bord temps réel : CA, marge, panier moyen, top produits/vendeurs, taux de retour
- Rapports périodiques : ventes par période/produit/catégorie/vendeur/magasin, rapport de marge, rapport de TVA collectée (aide déclaration)
- Rapport de stock : valorisation, rotation, produits dormants
- Rapport de caisse (Z quotidien, X intermédiaire)
- Export tous rapports en PDF/Excel
- Comparateur multi-magasins pour réseaux de franchise

---

## 9. Matériel & Intégrations physiques (essentiel terrain marocain)
- Imprimantes tickets ESC/POS (USB, réseau, Bluetooth) — formats 58mm/80mm
- Tiroir-caisse (ouverture via imprimante ou impulsion directe)
- Douchette code-barres (USB HID, sans driver custom)
- Terminal de paiement (TPE) — intégration CMI ou saisie manuelle du montant validé
- Balance connectée pour produits au poids
- Écran client secondaire (afficheur double-face pour transparence prix)
- Tablette prise de commande (serveur en salle, synchronisée avec KDS cuisine)
- Impression cuisine par zone (bar/cuisine chaude/froide) pour restaurants

---

## 10. Connectivité, Offline & Synchronisation
- **Mode offline-first obligatoire** : vente possible sans connexion internet, file d'attente de synchronisation dès reconnexion
- Résolution de conflits de synchronisation (dernier écrit gagne / merge intelligent selon le champ)
- Sync multi-device temps réel (POS ↔ backoffice ↔ mobile manager)
- Sauvegarde automatique locale + cloud
- Indicateur visuel clair de l'état de connexion (comme le badge wifi vu sur Alina POS)

---

## 11. Multi-canal & Extensions (différenciation marché)
- Application mobile gérant (consultation CA/stock à distance, notifications alertes stock)
- Intégration WhatsApp Business : envoi automatique de factures/tickets, notifications de commande, campagnes marketing (cohérent avec la stratégie social media Ritaj)
- Click & collect / commande en ligne simple connectée au même stock (si extension e-commerce)
- API ouverte pour intégrations tierces (comptabilité, CRM, livraison type Glovo/Kaalix)
- Mode multi-langue FR/AR (interface + documents imprimés, RTL pour l'arabe)

---

## 12. Paramétrage & Administration
- Configuration multi-boutique : horaires, devises, taux TVA par défaut, numérotation par entité
- Personnalisation en-tête/pied de page des documents imprimés (logo, mentions légales, CGV)
- Gestion des modèles d'impression (ticket, facture A4, étiquette)
- Sauvegarde/restauration des données, export complet (portabilité, anti-lock-in perçu = argument commercial)
- Gestion des abonnements/licences si modèle SaaS (multi-tenant, quotas par plan)

---

## 13. Priorisation suggérée pour un MVP "prêt marché"

| Phase | Contenu |
|---|---|
| **P0 — Socle vendable** | POS complet (vente, paiement, ticket, clôture caisse), Catalogue, Stock simple, Facture conforme DGI de base, Multi-utilisateurs/RBAC basique |
| **P1 — Cycle commercial complet** | Devis, Commande, BL, Avoir, chaînage de conversion, Partenaires (clients/fournisseurs), Paiements multi-modes |
| **P2 — Robustesse terrain** | Offline-first, multi-caisse, multi-magasin, matériel (imprimante/douchette/TPE), inventaire |
| **P3 — Différenciation** | Fidélité, WhatsApp Business, rapports avancés/BI, appli mobile gérant, conformité DGI facturation électronique complète |
| **P4 — Vertical spécifique** | KDS/tables (HoReCa), vrac/DLC (superette), variantes (mode) |

---

## 14. Checklist de conformité "prêt marché" (à valider avant lancement commercial)
- [ ] Facture conforme aux mentions légales marocaines (ICE, IF, RC, TVA détaillée)
- [ ] Numérotation séquentielle sans trou par document/année
- [ ] Fonctionnement garanti sans connexion internet (vente + impression ticket)
- [ ] Clôture de caisse (Z) avec écart signé, non modifiable a posteriori
- [ ] Sauvegarde/export des données accessible au client (pas de verrouillage total des données)
- [ ] Interface bilingue FR/AR minimum sur les écrans de vente
- [ ] Compatible avec le matériel POS déjà déployé chez les 200+ clients Ritaj existants (éviter rupture de compatibilité)
