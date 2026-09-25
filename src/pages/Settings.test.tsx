import { describe, it, expect, vi, beforeEach } from "vitest"
import { render, screen, fireEvent, waitFor } from "@testing-library/react"
import { MemoryRouter } from "react-router-dom"
import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import Settings from "./Settings"

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

const mockSettings = {
  shop_name: "SuperCaisse",
  shop_address: "15 Rue Test",
  shop_phone: "+212612345678",
  shop_email: "contact@supercaisse.ma",
  ice: "ICE123456789",
  if_number: "",
  rc_number: "",
  patente: "",
  default_tva: 20,
  receipt_footer: "Merci de votre visite",
  currency: "MAD",
}

describe("Settings", () => {
  it("renders title and tabs", async () => {
    mockInvoke
      .mockResolvedValueOnce([])
      .mockResolvedValueOnce(mockSettings)
    render(<Settings />, { wrapper: Wrapper })
    expect(await screen.findByText("Paramètres")).toBeInTheDocument()
    expect(screen.getByText("Général")).toBeInTheDocument()
    expect(screen.getByText("Utilisateurs")).toBeInTheDocument()
    expect(screen.getByText("Ticket")).toBeInTheDocument()
    expect(screen.getByText("Système")).toBeInTheDocument()
    expect(screen.getByText("Sauvegarde")).toBeInTheDocument()
  })

  it("renders general tab with settings form", async () => {
    mockInvoke
      .mockResolvedValueOnce([])
      .mockResolvedValueOnce(mockSettings)
    render(<Settings />, { wrapper: Wrapper })
    expect(await screen.findByDisplayValue("SuperCaisse")).toBeInTheDocument()
    expect(await screen.findByDisplayValue("15 Rue Test")).toBeInTheDocument()
    expect(await screen.findByDisplayValue("+212612345678")).toBeInTheDocument()
    expect(await screen.findByDisplayValue("contact@supercaisse.ma")).toBeInTheDocument()
  })

  it("renders users tab content", async () => {
    mockInvoke
      .mockResolvedValueOnce([
        { id: 1, login: "admin", nom: "Administrateur", role: "admin" },
        { id: 2, login: "caissier", nom: "Caissier Un", role: "caissier" },
      ])
      .mockResolvedValueOnce(mockSettings)
    render(<Settings />, { wrapper: Wrapper })
    const usersTab = await screen.findByRole("tab", { name: "Utilisateurs" })
    usersTab.click()
    expect(await screen.findByText("Gestion des utilisateurs")).toBeInTheDocument()
    expect(screen.getByText("Administrateur")).toBeInTheDocument()
    expect(screen.getByText("Caissier Un")).toBeInTheDocument()
  })

  it("saves settings whose optional fields are null", async () => {
    mockInvoke.mockImplementation((cmd: string) =>
      Promise.resolve(cmd === "get_settings"
        ? { ...mockSettings, shop_address: null, shop_phone: null, shop_email: null, ice: "001234567000089", if_number: "1234", rc_number: "RC 1", patente: null, receipt_footer: null, business_type: null, logo_base64: null }
        : cmd === "update_settings" ? null : []),
    )
    render(<Settings />, { wrapper: Wrapper })
    await screen.findByDisplayValue("001234567000089")
    fireEvent.click(screen.getAllByRole("button", { name: "Enregistrer les paramètres" })[0])
    await waitFor(() => expect(mockInvoke).toHaveBeenCalledWith("update_settings", expect.objectContaining({ ice: "001234567000089", shop_address: "" })))
  })

  it("rejects an invalid ICE", async () => {
    mockInvoke.mockImplementation((cmd: string) => Promise.resolve(cmd === "get_settings" ? mockSettings : []))
    render(<Settings />, { wrapper: Wrapper })
    await screen.findByDisplayValue("ICE123456789")
    fireEvent.click(screen.getAllByRole("button", { name: "Enregistrer les paramètres" })[0])
    expect(await screen.findByText("ICE invalide : 15 chiffres attendus")).toBeInTheDocument()
    expect(mockInvoke).not.toHaveBeenCalledWith("update_settings", expect.anything())
  })
})
