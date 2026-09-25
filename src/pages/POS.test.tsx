import { describe, it, expect, vi, beforeEach } from "vitest"
import { render, screen } from "@testing-library/react"
import { MemoryRouter } from "react-router-dom"
import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import POS from "./POS"

const mockInvoke = vi.fn()
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...args: unknown[]) => mockInvoke(...args) }))

vi.mock("@/context/AuthContext", () => ({
  useAuth: () => ({ user: { id: 1, nom: "Caissier", role: "caissier" } }),
  ProtectedRoute: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}))

vi.mock("@/lib/receipt", () => ({
  printViaTauri: vi.fn(),
  saveFacturePdf: vi.fn(),
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
  mockInvoke.mockImplementation((cmd: string) => {
    switch (cmd) {
      case "get_current_session": return Promise.resolve(null)
      case "get_articles": return Promise.resolve([])
      case "get_categories": return Promise.resolve([])
      case "get_clients": return Promise.resolve([])
      case "get_settings": return Promise.resolve({ nom_boutique: "Test", devise: "MAD", tva_defaut: 20 })
      case "get_magasins": return Promise.resolve([{ id: 1, nom: "Principal" }])
      case "get_tables": return Promise.resolve([])
      default: return Promise.resolve(null)
    }
  })
})

describe("POS", () => {
  it("renders session blocker when no session", async () => {
    render(<POS />, { wrapper: Wrapper })
    expect(await screen.findByText("Ouvrir la caisse")).toBeInTheDocument()
  })

  it("shows fond de caisse input", async () => {
    render(<POS />, { wrapper: Wrapper })
    expect(await screen.findByText(/Fond de caisse initial/i)).toBeInTheDocument()
  })
})
