import { describe, it, expect, vi, beforeEach } from "vitest"
import { render, screen, fireEvent, waitFor } from "@testing-library/react"
import { MemoryRouter } from "react-router-dom"
import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import Clients from "./Clients"

const mockInvoke = vi.fn()
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...args: unknown[]) => mockInvoke(...args) }))

vi.mock("@/context/AuthContext", () => ({
  useAuth: () => ({ user: { nom: "Admin" } }),
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

describe("Clients", () => {
  it("renders loading skeleton initially", () => {
    mockInvoke.mockImplementationOnce(() => new Promise(() => {}))
    render(<Clients />, { wrapper: Wrapper })
    expect(screen.getByText("Clients")).toBeInTheDocument()
    expect(screen.getByText("Gestion de la clientèle")).toBeInTheDocument()
  })

  it("renders empty state when no clients", async () => {
    mockInvoke.mockResolvedValueOnce([])
    render(<Clients />, { wrapper: Wrapper })
    expect(await screen.findByText("Aucun client trouvé")).toBeInTheDocument()
  })

  it("renders client list after loading", async () => {
    mockInvoke.mockResolvedValueOnce([
      { id: 1, code: "C001", nom: "Jean Dupont", telephone: "0612345678", email: "", adresse: "", credit_plafond: 0, credit_actuel: 0 },
      { id: 2, code: null, nom: "Marie Curie", telephone: null, email: "marie@test.com", adresse: "15 Rue Test", credit_plafond: 5000, credit_actuel: 1200 },
    ])
    render(<Clients />, { wrapper: Wrapper })
    expect(await screen.findByText("Jean Dupont")).toBeInTheDocument()
    expect(screen.getByText("Marie Curie")).toBeInTheDocument()
    expect(screen.getByText("C001")).toBeInTheDocument()
  })

  it("filters clients by search", async () => {
    mockInvoke.mockResolvedValueOnce([
      { id: 1, code: null, nom: "Jean Dupont", telephone: "0612345678", email: "", adresse: "", credit_plafond: 0, credit_actuel: 0 },
      { id: 2, code: null, nom: "Marie Curie", telephone: null, email: "", adresse: "", credit_plafond: 0, credit_actuel: 0 },
    ])
    render(<Clients />, { wrapper: Wrapper })
    expect(await screen.findByText("Jean Dupont")).toBeInTheDocument()
    expect(screen.getByText("Marie Curie")).toBeInTheDocument()
  })

  it("creates a client without an email", async () => {
    mockInvoke.mockImplementation((cmd: string) => Promise.resolve(cmd === "add_client" ? 3 : []))
    render(<Clients />, { wrapper: Wrapper })
    await screen.findByText("Aucun client trouvé")
    fireEvent.click(screen.getByRole("button", { name: /Nouveau client/ }))
    fireEvent.change(await screen.findByPlaceholderText("Nom complet"), { target: { value: "Sans Email" } })
    fireEvent.click(screen.getByRole("button", { name: "Enregistrer" }))
    await waitFor(() =>
      expect(mockInvoke).toHaveBeenCalledWith("add_client", expect.objectContaining({ nom: "Sans Email" })),
    )
    expect(screen.queryByText("Email invalide")).not.toBeInTheDocument()
  })
})
