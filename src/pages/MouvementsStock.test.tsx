import { describe, it, expect, vi, beforeEach } from "vitest"
import { render, screen, fireEvent, waitFor } from "@testing-library/react"
import { MemoryRouter } from "react-router-dom"
import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import MouvementsStock from "./MouvementsStock"

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
  mockInvoke.mockImplementation((cmd: string, args?: { recherche?: string | null; page?: number }) => {
    if (cmd === "get_mouvements_stock") {
      const lignes = args?.recherche
        ? [{ id: 9, date: "2026-09-01 00:00:00", article_id: 2, designation: "Yaourt nature", quantite: 3, mtype: "sortie", reference_id: null, reference_type: null }]
        : [{ id: 1, date: "2026-09-02 00:00:00", article_id: 1, designation: "Riz", quantite: 1, mtype: "entree", reference_id: null, reference_type: null }]
      return Promise.resolve({ lignes, total: args?.recherche ? 1 : 151, page: args?.page ?? 0, par_page: 100 })
    }
    if (cmd === "get_articles") return Promise.resolve([])
    return Promise.resolve(null)
  })
})

describe("MouvementsStock", () => {
  it("cherche un article sur toutes les pages via le serveur", async () => {
    render(<MouvementsStock />, { wrapper: Wrapper })
    fireEvent.click(await screen.findByRole("button", { name: "Page suivante" }))
    await waitFor(() => expect(mockInvoke).toHaveBeenCalledWith("get_mouvements_stock", expect.objectContaining({ page: 1 })))
    fireEvent.change(screen.getByPlaceholderText("Rechercher par article..."), { target: { value: "yaourt" } })
    expect(await screen.findByText("Yaourt nature")).toBeInTheDocument()
    expect(mockInvoke).toHaveBeenCalledWith("get_mouvements_stock", expect.objectContaining({ recherche: "yaourt", page: 0 }))
  })
})
