import { describe, it, expect, vi, beforeEach } from "vitest"
import { render, screen } from "@testing-library/react"
import { MemoryRouter } from "react-router-dom"
import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import Fournisseurs from "./Fournisseurs"

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

describe("Fournisseurs", () => {
  it("renders loading skeleton initially", () => {
    mockInvoke.mockImplementationOnce(() => new Promise(() => {}))
    render(<Fournisseurs />, { wrapper: Wrapper })
    expect(screen.getByText("Fournisseurs")).toBeInTheDocument()
  })

  it("renders empty state when no fournisseurs", async () => {
    mockInvoke.mockResolvedValueOnce([])
    render(<Fournisseurs />, { wrapper: Wrapper })
    expect(await screen.findByText("Aucun fournisseur trouvé")).toBeInTheDocument()
  })

  it("renders fournisseur list after loading", async () => {
    mockInvoke.mockResolvedValueOnce([
      { id: 1, nom: "Distrib SARL", ice: "ICE001", telephone: "0522000000", email: "", adresse: "" },
      { id: 2, nom: "Grossiste ABC", ice: null, telephone: null, email: "abc@test.com", adresse: "Zone ind." },
    ])
    render(<Fournisseurs />, { wrapper: Wrapper })
    expect(await screen.findByText("Distrib SARL")).toBeInTheDocument()
    expect(screen.getByText("Grossiste ABC")).toBeInTheDocument()
    expect(screen.getByText("ICE001")).toBeInTheDocument()
  })
})
