import { describe, it, expect, vi, beforeEach } from "vitest"
import { render, screen } from "@testing-library/react"
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
      if (cmd === "get_journal_caisse") return Promise.resolve([])
      if (cmd === "get_current_session") return Promise.resolve(null)
      return Promise.resolve(null)
    })
    render(<JournalCaisse />, { wrapper: Wrapper })
    expect(await screen.findByText("Aucune opération trouvée")).toBeInTheDocument()
  })

  it("renders journal entries after loading", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_journal_caisse") {
        return Promise.resolve([
          { id: 1, date: "2026-07-28T10:00:00", utilisateur_id: 1, jtype: "entree", montant: 500, description: "Vente du jour", user_nom: "Admin" },
          { id: 2, date: "2026-07-28T09:00:00", utilisateur_id: null, jtype: "sortie", montant: 100, description: "Dépense", user_nom: null },
        ])
      }
      if (cmd === "get_current_session") return Promise.resolve(null)
      return Promise.resolve(null)
    })
    render(<JournalCaisse />, { wrapper: Wrapper })
    expect(await screen.findByText("Vente du jour")).toBeInTheDocument()
    expect(screen.getByText("Dépense")).toBeInTheDocument()
  })
})
