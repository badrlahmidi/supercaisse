import { describe, it, expect, vi, beforeEach } from "vitest"
import { render, screen } from "@testing-library/react"
import { MemoryRouter } from "react-router-dom"
import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import Rapports from "./Rapports"

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

describe("Rapports", () => {
  it("renders page header", async () => {
    mockInvoke.mockResolvedValueOnce({
      ca_total: 0, total_remises: 0, nb_ventes: 0, marge_brute: 0, tva_collectee: 0,
      top_articles: [], rotation_stock: [], ventes_par_jour: [], par_mode: [],
    })
    render(<Rapports />, { wrapper: Wrapper })
    expect(screen.getByText("Rapports détaillés")).toBeInTheDocument()
  })

  it("shows loading state", () => {
    mockInvoke.mockImplementationOnce(() => new Promise(() => {}))
    render(<Rapports />, { wrapper: Wrapper })
    expect(screen.getByText("Rapports détaillés")).toBeInTheDocument()
  })

  it("renders stats after loading", async () => {
    mockInvoke.mockResolvedValueOnce({
      ca_total: 50000, total_remises: 500, nb_ventes: 120, marge_brute: 15000, tva_collectee: 8000,
      top_articles: [], rotation_stock: [], ventes_par_jour: [], par_mode: [],
    })
    render(<Rapports />, { wrapper: Wrapper })
    expect(await screen.findByText("Chiffre d'affaires")).toBeInTheDocument()
  })

  it("renders date filter inputs", () => {
    mockInvoke.mockResolvedValueOnce({
      ca_total: 0, total_remises: 0, nb_ventes: 0, marge_brute: 0, tva_collectee: 0,
      top_articles: [], rotation_stock: [], ventes_par_jour: [], par_mode: [],
    })
    render(<Rapports />, { wrapper: Wrapper })
    expect(screen.getByText("Rapports détaillés")).toBeInTheDocument()
    expect(screen.getByText("PDF")).toBeInTheDocument()
  })
})
