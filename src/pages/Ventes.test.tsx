import { describe, it, expect, vi, beforeEach } from "vitest"
import { render, screen, fireEvent, waitFor } from "@testing-library/react"
import { MemoryRouter } from "react-router-dom"
import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import Ventes from "./Ventes"

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

describe("Ventes", () => {
  it("renders title initially", () => {
    mockInvoke.mockImplementationOnce(() => new Promise(() => {}))
    render(<Ventes />, { wrapper: Wrapper })
    expect(screen.getByText("Documents de Vente")).toBeInTheDocument()
    expect(screen.getByText("Historique des factures, BL et devis")).toBeInTheDocument()
  })

  it("renders empty state when no ventes", async () => {
    mockInvoke.mockImplementation((cmd: string) =>
      Promise.resolve(cmd === "get_ventes" ? { lignes: [], total: 0, page: 0, par_page: 50, chiffre_affaires: 0 } : null)
    )
    render(<Ventes />, { wrapper: Wrapper })
    expect(await screen.findByText("Aucune vente trouvée")).toBeInTheDocument()
  })

  it("renders sales list after loading", async () => {
    mockInvoke.mockImplementation((cmd: string) =>
      Promise.resolve(cmd === "get_ventes" ? {
        lignes: [
          { id: 1, date: "2026-07-28T10:00:00", client_id: null, caissier_id: 1, montant_total: 100, montant_remise: 0, mode_paiement: "especes", statut: "validee", client_nom: null, caissier_nom: "Admin" },
          { id: 2, date: "2026-07-28T11:00:00", client_id: 1, caissier_id: 1, montant_total: 250, montant_remise: 25, mode_paiement: "carte", statut: "validee", client_nom: "Jean Dupont", caissier_nom: "Admin" },
        ],
        total: 1234,
        page: 0,
        par_page: 50,
        chiffre_affaires: 98765.4,
      } : null)
    )
    render(<Ventes />, { wrapper: Wrapper })
    expect(await screen.findByText("#1")).toBeInTheDocument()
    expect(screen.getByText("#2")).toBeInTheDocument()
    expect(screen.getByText("Client de passage")).toBeInTheDocument()
    expect(screen.getByText("Jean Dupont")).toBeInTheDocument()
    expect(screen.getByText("1234")).toBeInTheDocument()
    expect(screen.getByText("1–50 sur 1234")).toBeInTheDocument()
  })

  it("envoie la recherche au serveur et revient à la première page", async () => {
    mockInvoke.mockImplementation((cmd: string, args?: { page?: number }) =>
      Promise.resolve(cmd === "get_ventes" ? { lignes: [], total: 120, page: args?.page ?? 0, par_page: 50, chiffre_affaires: 0 } : null)
    )
    render(<Ventes />, { wrapper: Wrapper })
    fireEvent.click(await screen.findByRole("button", { name: "Page suivante" }))
    await waitFor(() => expect(mockInvoke).toHaveBeenCalledWith("get_ventes", expect.objectContaining({ page: 1 })))
    fireEvent.change(screen.getByPlaceholderText(/Rechercher/), { target: { value: "Dupont" } })
    await waitFor(() =>
      expect(mockInvoke).toHaveBeenCalledWith("get_ventes", expect.objectContaining({ recherche: "Dupont", page: 0 }))
    )
  })
})
