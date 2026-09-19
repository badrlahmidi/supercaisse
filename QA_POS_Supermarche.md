# QA — Scénarios de test POS Supermarché (50 cas)

## Contexte
Application de caisse (POS) pour supermarché — Tauri + React.  
Backend Rust (IPC via invoke). Frontend avec gestion d'état, stock, clients, fournisseurs, ventes, impressions.

---

## 1. Authentification & Contrôle d'accès (6 scénarios)

### TC-01 — Connexion nominative
**Précondition** : Utilisateur Admin existe en base  
**Étapes** :
1. Ouvrir l'app → page `/login`
2. Saisir login `admin` / mot de passe correct
3. Cliquer "Se connecter"
**Résultat attendu** : Redirigé vers `/pos`. Sidebar affiche "Admin", badge `admin` visible. Bouton déconnexion accessible.

### TC-02 — Connexion refusée (mauvais mot de passe)
**Étapes** :
1. Saisir login valide + mot de passe invalide
2. Cliquer "Se connecter"
**Résultat attendu** : Message d'erreur "Login ou mot de passe incorrect". Pas de redirection.

### TC-03 — Session persistée (localStorage)
**Étapes** :
1. Se connecter avec succès
2. Rafraîchir la page (F5)
**Résultat attendu** : Reste connecté (même utilisateur, pas de retour au login).

### TC-04 — Déconnexion
**Étapes** :
1. Cliquer "Déconnexion" dans la sidebar
**Résultat attendu** : Redirigé vers `/login`. localStorage nettoyé. Accès à `/pos` impossible sans re-connexion.

### TC-05 — Protection des routes (rôle caissier)
**Précondition** : Connecté en tant que `caissier`  
**Étapes** :
1. Naviguer vers `/settings`
2. Naviguer vers `/articles`
**Résultat attendu** : Redirigé vers `/pos` sur les deux routes (non autorisé).

### TC-06 — Accès aux routes admin
**Précondition** : Connecté en tant que `admin`  
**Étapes** : Tenter d'accéder à `/settings`  
**Résultat attendu** : Page Settings affichée (admin seulement).

---

## 2. POS — Caisse (12 scénarios)

### TC-07 — Ajout article par clic
**Précondition** : Articles chargés (grille visible)  
**Étapes** :
1. Cliquer sur un article dans la grille produit
**Résultat attendu** : Article ajouté au panier. Quantité = 1. Total panier mis à jour. Compteur d'articles incrémenté.

### TC-08 — Ajout article par scan code-barres
**Précondition** : Article avec code-barres en base  
**Étapes** :
1. Focus input recherche
2. Saisir code-barres avec un scanner (ou taper + Enter)
**Résultat attendu** : Article ajouté au panier. Champ recherche vidé. Focus reste sur la recherche.

### TC-09 — Incrémenter/décrémenter quantité
**Étapes** :
1. Ajouter un article au panier
2. Cliquer "+" sur l'article dans le panier
3. Cliquer "-"
**Résultat attendu** : Quantité augmente/diminue. Total ligne mis à jour.

### TC-10 — Saisie manuelle de quantité
**Étapes** :
1. Ajouter un article
2. Modifier la valeur dans le champ quantité (ex: `5`)
**Résultat attendu** : Quantité = 5. Total ligne = 5 × PU. Stock non dépassé.

### TC-11 — Suppression article du panier
**Étapes** :
1. Ajouter plusieurs articles
2. Cliquer l'icône corbeille sur un article
**Résultat attendu** : Article retiré du panier. Total recalculé.

### TC-12 — Vider le panier
**Étapes** :
1. Ajouter des articles
2. Cliquer "Vider"
**Résultat attendu** : Panier vide. Message "Panier vide" affiché. Total = 0.

### TC-13 — Remise pourcentage
**Étapes** :
1. Ajouter article(s) au panier
2. Saisir `10` dans "Remise %"
**Résultat attendu** : Remise de 10% appliquée. Ligne "Remise (10%)" visible. Net à payer = Total - 10%.

### TC-14 — Paiement espèces — monnaie rendue
**Étapes** :
1. Ajouter article (total 150,00 DH)
2. Mode paiement = Espèces
3. Saisir montant donné = `200`
**Résultat attendu** : "Monnaie à rendre: 50,00 DH" affiché en vert. Bouton "Encaisser" actif.

### TC-15 — Paiement espèces — montant insuffisant
**Étapes** :
1. Ajouter article (total 150,00 DH)
2. Saisir montant donné = `100`
**Résultat attendu** : Message "Montant insuffisant — Il manque 50,00 DH". Bouton "Encaisser" désactivé.

### TC-16 — Paiement carte bancaire
**Étapes** :
1. Ajouter article
2. Mode paiement = Carte bancaire
3. Cliquer "Encaisser"
**Résultat attendu** : Vente validée. Toast "Vente #X - 150,00 DH". Panier vidé.

### TC-17 — Paiement crédit client
**Précondition** : Client avec crédit_plafond > 0 existe  
**Étapes** :
1. Ajouter article
2. Mode paiement = Crédit
3. Sélectionner un client
4. Cliquer "Encaisser"
**Résultat attendu** : Vente validée. Montant ajouté au crédit du client.

### TC-18 — Validation sans articles
**Étapes** : Cliquer "Encaisser" alors que panier vide  
**Résultat attendu** : Rien ne se passe (bouton désactivé).

---

## 3. POS — Raccourcis clavier (5 scénarios)

### TC-19 — F1 : Focus recherche
**Étapes** : Appuyer sur F1  
**Résultat attendu** : Champ recherche focusé. Texte sélectionné si présent.

### TC-20 — F2 : Nouvelle vente
**Précondition** : Panier non vide  
**Étapes** : Appuyer sur F2  
**Résultat attendu** : Panier vidé, client réinitialisé, remise = 0. Aucun toast. Focus recherche.

### TC-21 — F4 : Supprimer dernier article
**Précondition** : Au moins 2 articles dans le panier  
**Étapes** : Appuyer sur F4  
**Résultat attendu** : Dernier article retiré du panier. Focus recherche.

### TC-22 — F5 : Valider vente avec configuration correcte
**Précondition** : Articles dans panier, mode paiement valide  
**Étapes** : Appuyer sur F5  
**Résultat attendu** : Vente créée. Toast de succès.

### TC-23 — F6 : Changer mode paiement
**Étapes** : Appuyer plusieurs fois sur F6  
**Résultat attendu** : Le mode de paiement cycle dans l'ordre : espèces → carte → chèque → crédit → virement → espèces...

---

## 4. Stock & Inventaire (5 scénarios)

### TC-24 — Affichage stock dans grille POS
**Précondition** : Article avec stock=5, seuil=10  
**Étapes** :
1. Observer la carte article dans la grille
**Résultat attendu** : Badge "Stock: 5" en jaune (warning). Carte avec bordure warning.

### TC-25 — Article en rupture de stock
**Précondition** : Article avec stock=0  
**Étapes** :
1. Observer la carte article
**Résultat attendu** : Carte grisée (opacity-50). Badge "Rupture" en rouge. Clic désactivé.

### TC-26 — Empêcher dépassement stock
**Précondition** : Article stock=3  
**Étapes** :
1. Ajouter l'article au panier (quantité=1)
2. Cliquer "+" 5 fois (tenter d'aller au-delà de 3)
**Résultat attendu** : Toast "Stock maximum atteint". Quantité bloquée à 3.

### TC-27 — Ajustement stock (page Stock)
**Étapes** :
1. Aller sur `/stock`
2. Cliquer "Ajuster" sur un article
3. Saisir quantité +10 (entrée) ou -2 (sortie)
4. Valider
**Résultat attendu** : Stock mis à jour. Toast "Stock mis à jour". La ligne article reflète le nouveau stock.

### TC-28 — Filtres stock
**Étapes** :
1. Aller sur `/stock`
2. Cliquer successivement sur les filtres : Tous / Rupture / Stock bas / OK
**Résultat attendu** : Le tableau se filtre. Seuls les articles correspondant au statut sont visibles.

---

## 5. Clients (4 scénarios)

### TC-29 — Création client
**Étapes** :
1. Aller sur `/clients`
2. Cliquer "Nouveau client"
3. Remplir nom, téléphone, email
4. Cliquer "Enregistrer"
**Résultat attendu** : Client créé. Toast "Client créé". Client visible dans le tableau et dans la Select client du POS.

### TC-30 — Modification client
**Étapes** :
1. Cliquer l'icône crayon sur un client
2. Modifier le nom
3. Cliquer "Enregistrer"
**Résultat attendu** : Nom mis à jour. Toast "Client mis à jour".

### TC-31 — Suppression client avec confirmation
**Étapes** :
1. Cliquer l'icône corbeille > dialog confirmation
2. Cliquer "Annuler"
3. Re-cliquer corbeille > "Supprimer"
**Résultat attendu** : Cas 1 : dialog fermé, client existe toujours. Cas 2 : client supprimé, toast "Client supprimé".

### TC-32 — Filtrage clients par recherche
**Précondition** : Au moins 3 clients  
**Étapes** :
1. Saisir un nom partiel dans la barre recherche
**Résultat attendu** : Tableau filtré en temps réel. Seuls les clients correspondant affichés.

---

## 6. Articles / Catalogue (4 scénarios)

### TC-33 — Création article complet
**Étapes** :
1. Aller sur `/articles`
2. "Nouvel article"
3. Remplir : désignation, prix achat, prix vente, TVA=20, stock=50, catégorie, fournisseur
4. "Enregistrer"
**Résultat attendu** : Article créé. Visible dans tableau, grille POS, et page Stock.

### TC-34 — TVA calculée sur le prix de vente
**Étapes** :
1. Créer article avec prix_vente=100, TVA=20
2. Ajouter au panier
**Résultat attendu** : Sous-total HT = 100. TVA = 20. Net à payer = 120.

### TC-35 — Désactivation d'article
**Précondition** : Article avec `actif: false`  
**Étapes** : Observer la grille POS  
**Résultat attendu** : Article non visible dans la grille POS (filtré par `actif`).

### TC-36 — Recherche article par désignation
**Étapes** :
1. Saisir une partie de désignation dans la barre recherche
**Résultat attendu** : Articles filtrés avec debounce 300ms. Résultats affichés.

---

## 7. Fournisseurs (2 scénarios)

### TC-37 — CRUD fournisseur complet
**Étapes** :
1. Créer un fournisseur (nom, ICE, téléphone)
2. Modifier le téléphone
3. Supprimer (avec confirmation)
**Résultat attendu** : Création → toast "Fournisseur créé". Modification → toast "Fournisseur mis à jour". Suppression → toast "Fournisseur supprimé".

### TC-38 — Lier fournisseur à un article
**Étapes** :
1. Créer un fournisseur
2. Créer un article en sélectionnant ce fournisseur
3. Observer la ligne article dans Articles
**Résultat attendu** : Colonne "Fournisseur" affiche le nom du fournisseur.

---

## 8. Catégories (2 scénarios)

### TC-39 — CRUD catégorie
**Étapes** : Créer / Modifier / Supprimer une catégorie  
**Résultat attendu** : Même pattern que clients/fournisseurs. Suppression confirmée par dialog.

### TC-40 — Filtrage POS par catégorie
**Précondition** : Articles répartis dans 2+ catégories  
**Étapes** :
1. Cliquer "Boissons" dans les filtres catégories du POS
**Résultat attendu** : Grille filtrée : seuls les articles de la catégorie "Boissons" visibles. Compteur mis à jour.

---

## 9. Ventes & Historique (4 scénarios)

### TC-41 — Consultation historique avec dates
**Étapes** :
1. Aller sur `/ventes`
2. Modifier la date de début (J-7)
3. Modifier la date de fin (aujourd'hui)
**Résultat attendu** : Liste filtrée aux ventes de la période. Stats "Total ventes" et "Nombre de ventes" recalculés.

### TC-42 — Détail d'une vente
**Étapes** :
1. Cliquer l'icône œil sur une vente
**Résultat attendu** : Dialog détail avec : date, client, caissier, mode paiement, lignes (désignation, qté, PU, TVA, total), sous-total, remise, net payé.

### TC-43 — Recherche dans ventes
**Étapes** :
1. Saisir un ID de vente (`#123` ou `123`)
2. Saisir un nom de client
3. Saisir un mode de paiement (`carte`)
**Résultat attendu** : Filtrage en temps réel sur client, caissier, ID, mode paiement.

### TC-44 — Export CSV Ventes
**Étapes** :
1. Cliquer "Exporter" sur `/ventes`
**Résultat attendu** : Fichier `ventes_AAAA-MM-JJ_AAAA-MM-JJ.csv` téléchargé. Contient : ID, Date, Client, Caissier, Total, Remise, Mode, Statut.

---

## 10. Export CSV (3 scénarios)

### TC-45 — Export Clients
**Étapes** :
1. Aller sur `/clients`
2. Cliquer "Exporter"
**Résultat attendu** : Fichier `clients.csv`. Colonnes : Code, Nom, Téléphone, Email, Adresse, Plafond crédit, Crédit actuel.

### TC-46 — Export Articles
**Étapes** :
1. Aller sur `/articles`
2. Cliquer "Exporter"
**Résultat attendu** : Fichier `articles.csv`. Colonnes : Code-barres, Désignation, Prix achat, Prix vente, TVA, Stock, Stock alerte, Catégorie, Fournisseur.

### TC-47 — Encodage CSV (Excel)
**Étapes** :
1. Exporter un CSV depuis Ventes
2. Ouvrir avec Excel
**Résultat attendu** : Caractères accentués corrects (BOM UTF-8). Nombres formatés.

---

## 11. Impression ticket (2 scénarios)

### TC-48 — Impression après vente (toast)
**Étapes** :
1. Valider une vente (panier non vide)
2. Dans le toast "Vente #X validée", cliquer "Imprimer"
**Résultat attendu** : Nouvelle fenêtre d'impression (ou popup). Ticket formaté : nom magasin, date, articles (désignation, qté, PU, total), sous-total, remise, net, monnaie, pied de page.

### TC-49 — Réimpression depuis le panier
**Précondition** : Vente validée précédemment  
**Étapes** :
1. Cliquer le bouton "Ticket" dans l'en-tête du panier
**Résultat attendu** : Même ticket que lors de la vente. Impression déclenchée.

---

## 12. Interface & Navigation (3 scénarios)

### TC-50 — Dashboard — indicateurs clés
**Précondition** : Ventes et articles en base  
**Étapes** :
1. Aller sur `/dashboard`
**Résultat attendu** : 5 cartes stats visibles (Ventes 30j, Articles, Stock alerte, Crédit clients, Nb clients). Graphique ventes 7 derniers jours. Dernières ventes. Raccourcis rapides.

### TC-51 — Dark mode
**Étapes** :
1. Cliquer l'icône Lune dans la sidebar
**Résultat attendu** : Classe `.dark` ajoutée au `<html>`. Interface en mode sombre. Persisté au rechargement.

### TC-52 — Sidebar responsive
**Étapes** :
1. Réduire la fenêtre en mobile (< 1024px)
**Résultat attendu** : Sidebar masquée. Menu hamburger visible. Clic sur hamburger → overlay + sidebar.

### TC-53 — Barre de navigation filtrée par rôle
**Précondition** : Connecté en `caissier`  
**Étapes** : Observer les entrées de navigation  
**Résultat attendu** : Seuls POS, Dashboard, Ventes visibles. Paramètres, Articles, Stock absents.

### TC-54 — Lazy loading des routes
**Étapes** : Naviguer entre toutes les routes (dashboard → articles → clients → etc.)  
**Résultat attendu** : Chaque page se charge avec un spinner de chargement (fallback Suspense). Pas de page blanche.

### TC-55 — ErrorBoundary sur échec de route
**Étapes** : (Simuler un chunk manquant ou une erreur de rendu)  
**Résultat attendu** : Page d'erreur avec icône AlertTriangle, message, bouton "Réessayer". Pas de crash de l'app entière.

---

## Matrice de couverture

| Domaine | Nb scénarios |
|---------|:-----------:|
| Authentification & Accès | 6 |
| POS — Caisse | 12 |
| POS — Raccourcis clavier | 5 |
| Stock & Inventaire | 5 |
| Clients | 4 |
| Articles | 4 |
| Fournisseurs | 2 |
| Catégories | 2 |
| Ventes & Historique | 4 |
| Export CSV | 3 |
| Impression ticket | 2 |
| Interface & Navigation | 6 |
| **Total** | **55** |
