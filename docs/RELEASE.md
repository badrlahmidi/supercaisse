# Publier une version de SuperCaisse

Les caisses installées vérifient les mises à jour depuis
`https://github.com/badrlahmidi/supercaisse/releases/latest/download/latest.json`
(Paramètres > Système > À propos et mises à jour, réservé à l'administrateur).
Chaque installateur est signé : une caisse refuse toute mise à jour dont la
signature ne correspond pas à la clé publique embarquée dans l'application.

## Mise en place (une seule fois)

1. Générer la paire de clés de signature sur un poste de confiance :

   ```sh
   npx tauri signer generate -w ~/.tauri/supercaisse.key
   ```

   Conserver la clé privée et son mot de passe hors du dépôt (coffre de mots
   de passe). Leur perte empêche de livrer des mises à jour aux caisses déjà
   installées ; leur fuite permet de signer une fausse mise à jour.

2. Copier le contenu de `~/.tauri/supercaisse.key.pub` dans
   `plugins.updater.pubkey` de `src-tauri/tauri.conf.json` et le committer.
   La clé publique n'est pas secrète.

3. Dans GitHub > Settings > Secrets and variables > Actions, créer :
   - `TAURI_SIGNING_PRIVATE_KEY` : contenu de `~/.tauri/supercaisse.key`
   - `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` : son mot de passe

## Publier une version

1. Mettre à jour la version (package.json, tauri.conf.json via
   `"version": "../package.json"`, Cargo.toml et Cargo.lock via le script
   `version`) :

   ```sh
   npm version 0.9.1
   git push --follow-tags
   ```

2. Le tag `v0.9.1` déclenche `.github/workflows/release.yml`, qui vérifie que
   le tag correspond à `package.json`, que la clé publique est renseignée,
   relance les tests, construit l'installateur Windows signé et crée une
   release GitHub **en brouillon** avec `latest.json`.

3. Tester l'installateur du brouillon sur une caisse de test, puis publier la
   release : les caisses la verront à leur prochaine recherche.

Avant chaque installation, l'application sauvegarde la base
(`backups/avant_mise_a_jour_*.db`) et trace l'opération dans le journal
d'audit. Les versions sont signées avec leur numéro (`requireSignedVersion`),
ce qui empêche de faire réinstaller une ancienne version sous un numéro plus
récent.
