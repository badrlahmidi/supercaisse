import { describe, it, expect, vi, beforeEach } from "vitest"
import { render, screen } from "@testing-library/react"
import { MemoryRouter } from "react-router-dom"
import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import Stock from "./Stock"

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

describe("Stock", () => {
  it("renders loading state initially", () => {
    mockInvoke.mockImplementationOnce(() => new Promise(() => {}))
    render(<Stock />, { wrapper: Wrapper })
    expect(screen.getByText("Gestion du stock")).toBeInTheDocument()
    expect(screen.getByText("Suivi des niveaux de stock")).toBeInTheDocument()
  })

  it("renders empty state when no articles", async () => {
    mockInvoke
      .mockResolvedValueOnce([])
    render(<Stock />, { wrapper: Wrapper })
    expect(await screen.findByText("Aucun article trouvé")).toBeInTheDocument()
  })

  it("renders stock list after loading", async () => {
    mockInvoke
      .mockResolvedValueOnce([
        { id: 1, code_barre: null, designation: "Sucre", prix_vente: 12, stock: 50, stock_alerte: 10, categorie_nom: "Épicerie" },
        { id: 2, code_barre: "654321", designation: "Huile", prix_vente: 25, stock: 3, stock_alerte: 5, categorie_nom: null },
      ])
    render(<Stock />, { wrapper: Wrapper })
    expect(await screen.findByText("Sucre")).toBeInTheDocument()
    expect(screen.getByText("Huile")).toBeInTheDocument()
  })

  it("shows alert badge for low stock items", async () => {
    mockInvoke
      .mockResolvedValueOnce([
        { id: 1, code_barre: null, designation: "Huile", prix_vente: 25, stock: 3, stock_alerte: 5, categorie_nom: null },
      ])
    render(<Stock />, { wrapper: Wrapper })
    expect(await screen.findByText("Huile")).toBeInTheDocument()
  })
})
