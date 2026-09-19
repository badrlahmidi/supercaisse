import { describe, it, expect, vi, beforeEach } from "vitest"
import { render, screen } from "@testing-library/react"
import { MemoryRouter } from "react-router-dom"
import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import Achats from "./Achats"

const mockInvoke = vi.hoisted(() => vi.fn())
vi.mock("@/lib/tauri", () => ({ invoke: mockInvoke }))

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

describe("Achats", () => {
  it("renders title initially", () => {
    mockInvoke.mockImplementation(() => new Promise(() => {}))
    render(<Achats />, { wrapper: Wrapper })
    expect(screen.getByText("Achats")).toBeInTheDocument()
    expect(screen.getByText("Gestion des achats fournisseurs")).toBeInTheDocument()
  })

  it("renders empty state when no achats", async () => {
    mockInvoke
      .mockResolvedValueOnce([])
      .mockResolvedValueOnce([])
      .mockResolvedValueOnce([])
    render(<Achats />, { wrapper: Wrapper })
    expect(await screen.findByText("Aucun achat trouvé")).toBeInTheDocument()
  })

  it("renders achats list after loading", async () => {
    mockInvoke
      .mockResolvedValueOnce([
        { id: 1, date: "2026-07-28T10:00:00", fournisseur_id: 1, reference: "REF001", montant_total: 500, statut: "recu", fournisseur_nom: "Fournisseur A" },
        { id: 2, date: "2026-07-27T14:00:00", fournisseur_id: null, reference: null, montant_total: 1200, statut: "recu", fournisseur_nom: null },
      ])
      .mockResolvedValueOnce([])
      .mockResolvedValueOnce([])
    render(<Achats />, { wrapper: Wrapper })
    expect(await screen.findByText("REF001")).toBeInTheDocument()
    expect(screen.getByText("Fournisseur A")).toBeInTheDocument()
  })
})
