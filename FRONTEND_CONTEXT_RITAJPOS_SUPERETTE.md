# FRONTEND_CONTEXT.md — RitajPOS Superette

> À coller en tête de tes prompts L'Orchestrateur pour ce projet. Structure/densité inspirées d'Alina POS, recolorées avec l'identité RitajPOS, spécifiées pour le stack ci-dessous.

## 0. Stack technique (contraintes pour l'agent)

```
React 19 + TypeScript + Vite 8
Tauri v2            → backend Rust, tout accès natif/DB passe par invoke() (@tauri-apps/api/core)
react-router-dom v7 → routes lazy (React.lazy + Suspense par route)
@tanstack/react-query v5 → tout le server state (jamais de useEffect+fetch manuel)
Zustand             → UI store (sidebar collapsed, modals, thème) + cart store (persisted via middleware persist)
react-hook-form + zod → tous les formulaires, schémas zod = source de vérité de validation
shadcn/ui-style     → composants dans src/ui/ (Button, Card, Dialog, Input, Table, Badge, Tabs, Select...)
Tailwind CSS v4     → tokens via @theme dans src/index.css, PAS de tailwind.config.js pour les couleurs
Vitest + @testing-library/react → test par composant/hook, colocated *.test.tsx
sonner              → tous les feedbacks (succès/erreur), jamais d'alert() natif
date-fns            → formatage dates (locale fr), jamais de manipulation Date manuelle
```

**Règles non négociables pour l'agent :**
- Server state (produits, ventes, stock, paiements) = **react-query** (`useQuery`/`useMutation`) qui appelle `invoke('cmd_name', {...})`. Jamais de state React local pour des données venant du backend Rust.
- État panier caisse = **Zustand cart store** (`useCartStore`), persisté en localStorage/Tauri store pour survivre à un refresh pendant une session de vente.
- Aucun style inline ni CSS module : uniquement classes Tailwind + tokens `@theme`.
- Composants UI de base (`Button`, `Badge`, `Card`...) ne se recréent jamais ad-hoc dans une page — toujours importés depuis `src/ui/`.
- Formulaires : `zodResolver` + `react-hook-form`, erreurs affichées inline sous le champ, jamais via `alert`/`toast` seul pour une erreur de validation de champ.
- Toasts `sonner` : succès = ton neutre confirmant l'action ("Vente enregistrée"), erreur = explique quoi + comment corriger, jamais "Une erreur est survenue" seul.

## 1. Design Tokens

### Tailwind v4 — `src/index.css` (`@theme`)
Tailwind v4 n'utilise plus `tailwind.config.js` pour la palette : tout passe par `@theme` en CSS. L'agent doit créer/mettre à jour ce bloc, pas un fichier config JS.

```css
@import "tailwindcss";

@theme {
  --color-primary: #2F6FED;
  --color-primary-dark: #1E4FBF;
  --color-primary-light: #E8F0FE;

  --color-accent: #F5A623;
  --color-accent-dark: #D98C0E;

  --color-ink: #3A3D42;
  --color-ink-soft: #6B7280;
  --color-sidebar: #1C1E26;

  --color-surface: #FFFFFF;
  --color-bg-page: #F4F6FA;
  --color-border: #E5E7EB;

  --color-success: #16A34A;
  --color-warning: #F5A623;
  --color-danger: #DC2626;

  --radius-card: 0.5rem;   /* 8px */
  --radius-pill: 999px;
}
```
Utilisation dans le JSX : `bg-primary`, `text-ink-soft`, `border-border`, `rounded-card`, etc. (Tailwind v4 génère automatiquement les utilitaires depuis les `--color-*` et `--radius-*` déclarés).

### Règle de remplacement vs Alina (référence visuelle)
- Tout ce qui était **rouge Alina** (`#E8362B`) → **`primary`** `#2F6FED` (bouton Payer, CTA, logo header, barre de progression top)
- Le **jaune `accent`** `#F5A623` remplace les touches "attention" ponctuelles (badges promo produit, badge notification)
- Sidebar : fond sombre bleu-anthracite `sidebar` (`#1C1E26`) au lieu du noir pur Alina

### Typographie
- Police : Inter (via `@font-face` ou Google Fonts self-hosted, pas de CDN externe pour l'app desktop Tauri)
- Titres section : `text-lg font-semibold text-ink` (18–20px)
- Labels colonnes/tables : `text-xs font-medium text-ink-soft uppercase tracking-wide`
- Valeurs chiffrées (prix, totaux) : `font-bold tabular-nums`
- Montants MAD toujours formatés `X XXX,XX MAD` (helper `formatMAD()` centralisé dans `src/lib/format.ts`, utilisant `Intl.NumberFormat('fr-MA')`)

### Spacing & radius
- Radius cards/inputs/boutons : `rounded-card` (8px)
- Radius pills (catégories) : `rounded-pill` (full)
- Padding card standard : `p-4` à `p-5` (16–20px)
- Gap grille produits : `gap-3` (12px)

## 2. Layout global

```
┌─────────────────────────────────────────────┐
│ barre fine "brand" (dégradé primary) 3-4px   │
├───────────┬───────────────────────────────────┤
│           │ Header: session/caisse | icônes | user │
│  Sidebar  ├───────────────────────────────────┤
│  (sombre) │                                   │
│  icônes+  │        Zone contenu (bg-page)     │
│  labels   │        cards blanches, ombre légère│
│           │                                   │
└───────────┴───────────────────────────────────┘
```
Sidebar collapsible (icône `«`), items avec sous-menus dépliables : Tableau de bord, Point de Vente, Partenaires, Ventes, Achats, Paiement, Catalogue, Stock, Utilisateurs, Rapports, Réglages.

## 2bis. Structure de fichiers attendue

```
src/
  ui/                    # composants shadcn/ui-style, agnostiques métier
    button.tsx, card.tsx, dialog.tsx, input.tsx, table.tsx, badge.tsx, tabs.tsx, select.tsx...
  components/            # composants métier composés à partir de src/ui
    pos/                 # ProductGrid, CartPanel, NumericPad, CategoryPills
    dashboard/           # KpiCard, SalesChart, SellerChart
    products/            # ProductForm, ProductStockTab
  routes/                # une route = un fichier, export default lazy-loadable
    dashboard.tsx
    pos.tsx
    sales/index.tsx, sales/[id].tsx
    stock/inventory.tsx
    settings.tsx
  stores/
    ui-store.ts          # useUiStore: sidebarCollapsed, activeModal
    cart-store.ts         # useCartStore: items, addItem, removeItem, total (persisted)
  hooks/                  # hooks react-query par domaine
    use-products.ts, use-sales.ts, use-stock.ts, use-payments.ts
  lib/
    format.ts             # formatMAD, formatDate (date-fns, locale fr)
    schemas/              # schémas zod par entité (product.schema.ts, sale.schema.ts...)
  App.tsx                 # <RouterProvider>, <QueryClientProvider>, <Toaster /> (sonner)
```

### Routing (react-router-dom v7)
- Routes déclarées avec `createBrowserRouter` + `lazy: () => import('./routes/pos')`
- Layout parent (`AppShell`) contient Sidebar + Header + `<Outlet />`, jamais dupliqué par route
- Chaque route lourde (Point de Vente, Stock) affiche un `<Suspense fallback={<RouteSkeleton />}>`

### State management — qui fait quoi
| Donnée | Outil | Exemple |
|---|---|---|
| Catalogue produits, stock, ventes, paiements | react-query | `useQuery(['products', categoryId], () => invoke('list_products', { categoryId }))` |
| Écriture (créer vente, ajuster stock) | react-query `useMutation` + `invalidateQueries` | `useMutation({ mutationFn: (sale) => invoke('create_sale', { sale }) })` |
| Panier en cours (écran POS) | Zustand `useCartStore` (persisted) | `addItem`, `removeItem`, `setQty`, `clear`, sélecteur `total` |
| UI éphémère (sidebar, modal ouvert, tab actif) | Zustand `useUiStore` (non persisted) | `toggleSidebar`, `openModal(id)` |
| Formulaires (produit, client, ajustement stock) | react-hook-form + zod | `useForm({ resolver: zodResolver(productSchema) })` |

### Exemple pattern formulaire (à respecter partout)
```tsx
const form = useForm<ProductInput>({ resolver: zodResolver(productSchema) });
const { mutate, isPending } = useMutation({
  mutationFn: (data: ProductInput) => invoke('save_product', { data }),
  onSuccess: () => { toast.success('Produit enregistré'); queryClient.invalidateQueries({ queryKey: ['products'] }); },
  onError: (e) => toast.error(`Échec de l'enregistrement : ${e.message}`),
});
```

## 3. Composants clés (adaptés au contexte Superette)

### 3.1 Écran Point de Vente — `routes/pos.tsx`
- **`<CartPanel />`** (colonne gauche) : lit `useCartStore`, liste articles (nom, qté × PU, sous-total), ligne active `bg-primary-light`
- **`<NumericPad />`** : boutons Qté/Rem/Prix/+/- qui écrivent dans `useCartStore` (pas de state local isolé)
- Bloc total : "Taxes" + "Total à payer" — dérivé du store via un sélecteur mémoïsé (`useCartStore(s => s.total)`)
- Boutons `Client` / `En attente` : `<Button variant="outline">` (src/ui)
- **`<Button size="xl" className="w-full">Payer</Button>`** : `primary`, hauteur ≥56px (cible tactile), déclenche `useMutation` → `invoke('create_sale', {...})`
- **`<CategoryPills />`** + **`<ProductGrid />`** (colonne droite) : `useQuery(['products', categoryId])`, cards produit avec badge stock (`success`/`warning`/`danger`)
- Champ recherche/scanner code-barres : `<Input autoFocus>` en haut de `ProductGrid`, doit rester focus après chaque scan (douchette = clavier + Enter)

### 3.2 Tableau de bord — `routes/dashboard.tsx`
- Rangée de `<KpiCard />` (Nouveaux clients, Ventes, CA, Marge brute, Panier moyen, Taux de retour), chaque valeur via `useQuery(['dashboard-kpis', dateRange])`
- `<SalesChart />` (barres) + `<SellerChart />` (lignes multi-séries) — lib de charts au choix de l'agent (recharts recommandé, cohérent avec l'écosystème React)

### 3.3 Tables de données (Ventes, Paiements, Commandes fournisseur, Inventaire)
- `<DataTable />` générique dans `src/ui/table.tsx` : header sticky, tri/filtre via colonnes, pagination
- Badges statut via `<Badge variant="success|neutral|primary|danger">` — Accepté/Livré=success, Brouillon=neutral, Validé=primary, Annulé=danger
- Actions en icônes de fin de ligne (lucide-react : Eye, Copy, Pencil, Trash2), couleurs cohérentes avec le variant

### 3.4 Modals — `src/ui/dialog.tsx`
- `<Dialog>` centré, header titre + `<DialogClose />`, footer `Annuler` (`variant="outline"`) + action principale (`variant="default"`, ex Sauvegarder)

### 3.5 Fiche produit / Formulaires — `components/products/ProductForm.tsx`
- Layout 2 colonnes : image+détails à gauche, `<Tabs>` (Ventes/Stock/Mouvements/Conditionnement) à droite
- Formulaires (produit, ajustement stock, commande fournisseur) : react-hook-form + zod, tableau lignes éditable inline pour les documents multi-lignes (Désignation, Prix U HT, Quantité, Taxe, Total HT) géré via `useFieldArray`

## 3bis. Tests & accessibilité (Vitest)
- Un test par composant métier significatif : rendu + interaction clé (`fireEvent.click` sur Payer → vérifie `mutate` appelé)
- Mock `invoke` via `vi.mock('@tauri-apps/api/core')` dans un setup file partagé
- Focus visible obligatoire sur tous les éléments interactifs (écran tactile + clavier/douchette) : ne jamais `outline-none` sans remplacement `focus-visible:ring-2 ring-primary`
- Cibles tactiles ≥ 44px de hauteur sur l'écran POS (boutons pavé numérique, cards produit cliquables)

## 4. Spécificités métier "Superette" à intégrer
- Priorité au **scan code-barres rapide** (le champ recherche catalogue doit accepter focus clavier + douchette)
- Gestion **poids/vrac** possible (fruits, légumes en kg) → prévoir unité variable sur les cards produit
- **Alertes stock bas** visibles directement sur la card produit (badge orange/rouge), pas seulement dans l'onglet Stock
- Multi-caisse / multi-session (comme Alina) essentiel pour superette avec plusieurs postes d'encaissement

## 5. Instruction pour l'agent (Claude Code / Cline)
> Utiliser exclusivement les tokens ci-dessus. Ne jamais réintroduire de rouge Alina. Le bleu `#2F6FED` est LA couleur d'action principale du produit. Respecter la densité d'information des tables (ligne compacte ~40-44px hauteur) car les utilisateurs superette travaillent sur des écrans tactiles 15-21".
