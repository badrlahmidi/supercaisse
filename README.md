# SuperCaisse (RitajPOS)

Logiciel de caisse / gestion retail (Tauri v2 + React 19 + TypeScript + SQLite),
destiné au marché marocain.

## Documentation

| Document | Rôle |
|---|---|
| `AGENTS.md` | Stack technique, conventions de code, structure du projet — à lire avant toute contribution |
| `ROADMAP_STATUS.md` | **État d'avancement réel**, vérifié module par module dans le code (source de vérité pour "qu'est-ce qui marche aujourd'hui ?") |
| `FONCTIONS_METIER.md` | Inventaire des fonctions métier — inclus / à améliorer / manquant, vue business pour prioriser le backlog |
| `AUDIT_ARCHITECTURE_SENIOR_2026-09.md` | Audit architecture, conformité fiscale marocaine (DGI) et couverture par vertical retail (supermarché, pharmacie/para, prêt-à-porter, matériel pâtisserie) |
| `SPEC_FONCTIONNELLE_RITAJ_RETAIL_MAROC.md` | Cahier des charges cible (vision produit, pas état d'avancement) |
| `QA_POS_Supermarche.md` | Scénarios de non-régression du module POS |
| `docs/archive/` | Anciens documents de planification, conservés pour historique — non fiables comme état actuel, cf. note en tête de chaque fichier |

## Démarrage

```bash
npm install
npm run dev          # serveur Vite
npm run tauri dev    # application desktop complète (frontend + backend Rust)
```

## Commandes

- `npm run build` — build production Vite
- `npm run lint` — Oxlint
- `npx tsc --noEmit` — vérification TypeScript
- `npm run test` — Vitest
- `cargo check` (dans `src-tauri/`) — vérification du backend Rust

## Stack

React 19 + TypeScript + Vite 8 · Tauri v2 (Rust, SQLite) · react-router-dom v7 ·
@tanstack/react-query v5 · Zustand · react-hook-form + zod · Tailwind CSS v4 ·
Vitest + Testing Library.

Détails complets des conventions dans `AGENTS.md`.
