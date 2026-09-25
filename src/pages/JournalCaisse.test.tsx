import { describe, it, expect, vi, beforeEach } from "vitest"
import { render, screen, fireEvent } from "@testing-library/react"
import { MemoryRouter } from "react-router-dom"
import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import JournalCaisse from "./JournalCaisse"

const mockInvoke = vi.hoisted(() => vi.fn())
vi.mock("@/lib/tauri", () => ({ invoke: mockInvoke }))

vi.mock("@/context/AuthContext", () => ({
  useAuth: () => ({ user: { id: 1, nom: "Admin", role: "admin" } }),
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

describe("JournalCaisse", () => {
  it("renders title initially", () => {
    mockInvoke.mockImplementation(() => new Promise(() => {}))
    render(<JournalCaisse />, { wrapper: Wrapper })
    expect(screen.getByText("Journal de caisse")).toBeInTheDocument()
    expect(screen.getByText("Suivi des mouvements de caisse")).toBeInTheDocument()
  })

  it("renders empty state when no entries", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_journal_caisse") return Promise.resolve({ lignes: [], total: 0, page: 0, par_page: 100, total_entrees: 0, total_sorties: 0 })
      if (cmd === "get_current_session") return Promise.resolve(null)
      return Promise.resolve(null)
    })
    render(<JournalCaisse />, { wrapper: Wrapper })
    expect(await screen.findByText("Aucune opération trouvée")).toBeInTheDocument()
  })

  it("renders journal entries after loading", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_journal_caisse") {
        return Promise.resolve({
          lignes: [
            { id: 1, date: "2026-07-28T10:00:00", utilisateur_id: 1, jtype: "entree", montant: 500, description: "Vente du jour", user_nom: "Admin" },
            { id: 2, date: "2026-07-28T09:00:00", utilisateur_id: null, jtype: "sortie", montant: 100, description: "Dépense", user_nom: null },
          ],
          total: 250,
          page: 0,
          par_page: 100,
          total_entrees: 12500,
          total_sorties: 3400,
        })
      }
      if (cmd === "get_current_session") return Promise.resolve(null)
      return Promise.resolve(null)
    })
    render(<JournalCaisse />, { wrapper: Wrapper })
    expect(await screen.findByText("Vente du jour")).toBeInTheDocument()
    expect(screen.getByText("Dépense")).toBeInTheDocument()
    expect(screen.getByText("1–100 sur 250")).toBeInTheDocument()
    expect(screen.getByText("Page 1 / 3")).toBeInTheDocument()
  })

  it("demande la page suivante au serveur", async () => {
    mockInvoke.mockImplementation((cmd: string, args?: { page?: number }) => {
      if (cmd === "get_journal_caisse") {
        return Promise.resolve({
          lignes: [{ id: 10 + (args?.page ?? 0), date: "2026-07-28T10:00:00", utilisateur_id: 1, jtype: "entree", montant: 5, description: `Ligne page ${args?.page ?? 0}`, user_nom: "Admin" }],
          total: 150,
          page: args?.page ?? 0,
          par_page: 100,
          total_entrees: 750,
          total_sorties: 0,
        })
      }
      return Promise.resolve(null)
    })
    render(<JournalCaisse />, { wrapper: Wrapper })
    fireEvent.click(await screen.findByRole("button", { name: "Page suivante" }))
    expect(await screen.findByText("Ligne page 1")).toBeInTheDocument()
    expect(mockInvoke).toHaveBeenCalledWith("get_journal_caisse", expect.objectContaining({ page: 1, parPage: 100 }))
  })
})
