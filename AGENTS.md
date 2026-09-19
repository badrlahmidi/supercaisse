# SuperCaisse — POS frontend (Tauri + React)

## Stack
- React 19 + TypeScript + Vite 8
- Tauri v2 (Rust backend, IPC via `@tauri-apps/api/core` invoke)
- react-router-dom v7 (lazy routes)
- @tanstack/react-query v5 (server state)
- Zustand (UI store + cart store, persisted)
- react-hook-form + zod (forms)
- shadcn/ui-style components in `src/ui/`
- Tailwind CSS v4
- Vitest + @testing-library/react (testing)
- sonner (toasts)
- date-fns (dates)

## Project structure
- `src/pages/` — lazy-loaded route pages (12 routes)
- `src/components/` — Layout, SuspensePage, ErrorBoundary
- `src/context/` — AuthContext (login/logout/roles)
- `src/store/` — Zustand stores (cart.ts, ui.ts)
- `src/ui/` — Reusable UI primitives (Button, Card, Dialog, Input, Table, Badge, Label, Textarea, Select)
- `src/lib/` — utils (formatCurrency, formatDate, cn)
- `src/routes/` — router.tsx (all routes + code-split config)
- `src/test/` — test setup (jest-dom matchers)

## Important conventions
- All page components are `export default function PageName()` — for React.lazy compatibility
- No barrel exports; import directly from the file (e.g. `@/ui/Button`)
- No inline comments in code
- Use `navigate()` instead of `<a href>` (SPA routing)
- Error boundaries wrap every lazy route via SuspensePage
- Loading skeletons on all list pages (Clients, Fournisseurs, Categories)
- Delete confirmations use Dialog with AlertTriangle + destructive button
- Dark mode via class strategy (`.dark` on `<html>`), controlled by `useUIStore`

## Tests
- `npm run test` — runs Vitest
- Tests live alongside their modules (`.test.ts` or `.test.tsx`)
- @tauri-apps/api/core must be mocked in tests

## Build commands
- `npm run dev` — Vite dev server
- `npm run build` — Vite production build
- `npm run lint` — ESLint
- `npx tsc --noEmit` — TypeScript check
- `npx vite build` — verify production build
