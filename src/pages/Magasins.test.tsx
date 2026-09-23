import { describe, it, expect, vi, beforeEach } from "vitest"
import { render, screen } from "@testing-library/react"
import { MemoryRouter } from "react-router-dom"
import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import Magasins from "./Magasins"

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

describe("Magasins", () => {
  it("renders page header", async () => {
    mockInvoke.mockResolvedValueOnce([{ id: 1, nom: "Principal", adresse: "Casablanca" }])
    render(<Magasins />, { wrapper: Wrapper })
    expect(await screen.findByText("Boutiques")).toBeInTheDocument()
  })

  it("renders magasin list", async () => {
    mockInvoke.mockResolvedValueOnce([
      { id: 1, nom: "Principal", adresse: "Casablanca" },
      { id: 2, nom: "Succursale", adresse: "Rabat" },
    ])
    render(<Magasins />, { wrapper: Wrapper })
    expect((await screen.findAllByText("Principal")).length).toBeGreaterThan(0)
    expect(screen.getAllByText("Succursale").length).toBeGreaterThan(0)
  })

  it("shows new boutique button", async () => {
    mockInvoke.mockResolvedValueOnce([])
    render(<Magasins />, { wrapper: Wrapper })
    expect(await screen.findByText("Nouvelle boutique")).toBeInTheDocument()
  })
})
