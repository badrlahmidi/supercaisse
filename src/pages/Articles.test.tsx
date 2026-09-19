import { describe, it, expect, vi, beforeEach } from "vitest"
import { render, screen } from "@testing-library/react"
import { MemoryRouter } from "react-router-dom"
import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import Articles from "./Articles"

const mockInvoke = vi.fn()
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...args: unknown[]) => mockInvoke(...args) }))

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

describe("Articles", () => {
  it("renders loading skeleton initially", () => {
    mockInvoke.mockImplementation(() => new Promise(() => {}))
    render(<Articles />, { wrapper: Wrapper })
    expect(screen.getByText("Articles")).toBeInTheDocument()
    expect(screen.getByText("Gestion du catalogue produits")).toBeInTheDocument()
  })

  it("renders empty state when no articles", async () => {
    mockInvoke.mockResolvedValue([])
    render(<Articles />, { wrapper: Wrapper })
    expect(await screen.findByText("Aucun article trouvé")).toBeInTheDocument()
  })

  it("renders article list after loading", async () => {
    mockInvoke
      .mockResolvedValueOnce([
        { id: 1, code_barre: "123456", designation: "Coca-Cola", prix_achat: 5, prix_vente: 8, tva: 10, stock: 100, stock_alerte: 10, categorie_id: 1, categorie_nom: "Boissons", fournisseur_nom: "Distrib", actif: true },
      ])
      .mockResolvedValueOnce([])
      .mockResolvedValueOnce([])
    render(<Articles />, { wrapper: Wrapper })
    expect(await screen.findByText("Coca-Cola")).toBeInTheDocument()
    expect(screen.getByText("123456")).toBeInTheDocument()
  })
})
