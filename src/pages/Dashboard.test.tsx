import { describe, it, expect, vi, beforeEach } from "vitest"
import { render, screen } from "@testing-library/react"
import { MemoryRouter } from "react-router-dom"
import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import Dashboard from "./Dashboard"

const mockInvoke = vi.fn()
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...args: unknown[]) => mockInvoke(...args) }))

vi.mock("@/context/AuthContext", () => ({
  useAuth: () => ({ user: { nom: "Admin" } }),
  ProtectedRoute: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}))

function Wrapper({ children }: { children: React.ReactNode }) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  return (
    <QueryClientProvider client={qc}>
      <MemoryRouter>{children}</MemoryRouter>
    </QueryClientProvider>
  )
}

beforeEach(() => {
  vi.clearAllMocks()
})

describe("Dashboard", () => {
  it("renders loading skeleton initially", () => {
    mockInvoke.mockImplementationOnce(() => new Promise(() => {}))
    render(<Dashboard />, { wrapper: Wrapper })
    expect(screen.getByText("Tableau de bord")).toBeInTheDocument()
    expect(screen.getByText("Bienvenue, Admin")).toBeInTheDocument()
  })

  it("renders stats cards after loading", async () => {
    mockInvoke
      .mockResolvedValueOnce({
        total_ventes_30j: 15000,
        nb_articles: 120,
        stock_alerte: 3,
        credit_total: 2500,
        nb_clients: 45,
      })
      .mockResolvedValueOnce([])

    render(<Dashboard />, { wrapper: Wrapper })

    expect(await screen.findByText("CA Aujourd'hui")).toBeInTheDocument()
    expect(await screen.findByText("CA ce Mois")).toBeInTheDocument()
    expect(await screen.findByText("Marge brute (Mois)")).toBeInTheDocument()
    expect(await screen.findByText("Crédit à recouvrer")).toBeInTheDocument()
    expect(await screen.findByText("Stock en alerte")).toBeInTheDocument()
  })

  it("renders quick actions", async () => {
    mockInvoke
      .mockResolvedValueOnce({
        total_ventes_30j: 0,
        nb_articles: 0,
        stock_alerte: 0,
        credit_total: 0,
        nb_clients: 0,
      })
      .mockResolvedValueOnce([])

    render(<Dashboard />, { wrapper: Wrapper })

    expect(await screen.findByText("Raccourcis rapides")).toBeInTheDocument()
    expect(screen.getByText("Caisse POS")).toBeInTheDocument()
    expect(screen.getByText("Ajouter article")).toBeInTheDocument()
    expect(screen.getByText("Voir le stock")).toBeInTheDocument()
    expect(screen.getByText("Gérer clients")).toBeInTheDocument()
  })

  it("shows empty state when no sales", async () => {
    mockInvoke
      .mockResolvedValueOnce({
        total_ventes_30j: 0,
        nb_articles: 0,
        stock_alerte: 0,
        credit_total: 0,
        nb_clients: 0,
      })
      .mockResolvedValueOnce([])

    render(<Dashboard />, { wrapper: Wrapper })

    expect(await screen.findByText("Aucune vente pour le moment")).toBeInTheDocument()
    expect(screen.getByText("Effectuer une vente")).toBeInTheDocument()
  })
})
