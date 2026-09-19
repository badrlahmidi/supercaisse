import { describe, it, expect, vi, beforeEach } from "vitest"
import { render, screen } from "@testing-library/react"
import { MemoryRouter } from "react-router-dom"
import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import Categories from "./Categories"

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

describe("Categories", () => {
  it("renders loading skeleton initially", () => {
    mockInvoke.mockImplementationOnce(() => new Promise(() => {}))
    render(<Categories />, { wrapper: Wrapper })
    expect(screen.getByText("Catégories")).toBeInTheDocument()
  })

  it("renders empty state when no categories", async () => {
    mockInvoke.mockResolvedValueOnce([])
    render(<Categories />, { wrapper: Wrapper })
    expect(await screen.findByText("Aucune catégorie trouvée")).toBeInTheDocument()
  })

  it("renders category list after loading", async () => {
    mockInvoke.mockResolvedValueOnce([
      { id: 1, nom: "Boissons", description: "Boissons et jus" },
      { id: 2, nom: "Snacks", description: null },
    ])
    render(<Categories />, { wrapper: Wrapper })
    expect(await screen.findByText("Boissons")).toBeInTheDocument()
    expect(screen.getByText("Snacks")).toBeInTheDocument()
    expect(screen.getByText("Boissons et jus")).toBeInTheDocument()
  })
})
