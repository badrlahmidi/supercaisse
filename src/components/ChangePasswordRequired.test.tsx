import { describe, it, expect, vi, beforeEach } from "vitest"
import { render, screen, waitFor } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { MemoryRouter } from "react-router-dom"
import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import { AuthProvider, ProtectedRoute } from "@/context/AuthContext"

const mockInvoke = vi.hoisted(() => vi.fn())
vi.mock("@/lib/tauri", () => ({ invoke: mockInvoke }))

vi.mock("sonner", () => ({
  toast: { success: vi.fn(), error: vi.fn() },
}))

function renderProtected() {
  const queryClient = new QueryClient({ defaultOptions: { mutations: { retry: false } } })
  return render(
    <QueryClientProvider client={queryClient}>
      <MemoryRouter>
        <AuthProvider>
          <ProtectedRoute>
            <div data-testid="page">Page protégée</div>
          </ProtectedRoute>
        </AuthProvider>
      </MemoryRouter>
    </QueryClientProvider>,
  )
}

beforeEach(() => {
  vi.clearAllMocks()
  localStorage.clear()
  mockInvoke.mockImplementation((cmd: string) => Promise.resolve(cmd === "get_permissions" ? [] : undefined))
})

describe("ChangePasswordRequired", () => {
  it("blocks protected pages until the password is changed", async () => {
    localStorage.setItem("supercaisse_user", JSON.stringify({ id: 1, login: "admin", nom: "Admin", role: "admin", must_change_password: true }))
    renderProtected()
    expect(await screen.findByText("Changement de mot de passe obligatoire")).toBeInTheDocument()
    expect(screen.queryByTestId("page")).not.toBeInTheDocument()
  })

  it("shows the page directly when no change is required", async () => {
    localStorage.setItem("supercaisse_user", JSON.stringify({ id: 1, login: "admin", nom: "Admin", role: "admin" }))
    renderProtected()
    expect(await screen.findByTestId("page")).toBeInTheDocument()
  })

  it("validates the form before calling the backend", async () => {
    const user = userEvent.setup()
    localStorage.setItem("supercaisse_user", JSON.stringify({ id: 1, login: "admin", nom: "Admin", role: "admin", must_change_password: true }))
    renderProtected()
    await user.type(await screen.findByLabelText("Mot de passe actuel"), "admin")
    await user.type(screen.getByLabelText("Nouveau mot de passe"), "court")
    await user.type(screen.getByLabelText("Confirmer le nouveau mot de passe"), "autre")
    await user.click(screen.getByRole("button", { name: /enregistrer/i }))
    expect(await screen.findByText("Minimum 8 caractères")).toBeInTheDocument()
    expect(mockInvoke).not.toHaveBeenCalledWith("change_password", expect.anything())
  })

  it("unlocks the app after a successful change", async () => {
    const user = userEvent.setup()
    localStorage.setItem("supercaisse_user", JSON.stringify({ id: 1, login: "admin", nom: "Admin", role: "admin", must_change_password: true }))
    renderProtected()
    await user.type(await screen.findByLabelText("Mot de passe actuel"), "admin")
    await user.type(screen.getByLabelText("Nouveau mot de passe"), "Caisse-2026!")
    await user.type(screen.getByLabelText("Confirmer le nouveau mot de passe"), "Caisse-2026!")
    await user.click(screen.getByRole("button", { name: /enregistrer/i }))
    expect(await screen.findByTestId("page")).toBeInTheDocument()
    expect(mockInvoke).toHaveBeenCalledWith("change_password", {
      userId: 1,
      ancienMotDePasse: "admin",
      nouveauMotDePasse: "Caisse-2026!",
    })
    await waitFor(() => {
      expect(JSON.parse(localStorage.getItem("supercaisse_user")!).must_change_password).toBe(false)
    })
  })

  it("stays locked when the backend refuses the change", async () => {
    const user = userEvent.setup()
    mockInvoke.mockImplementation((cmd: string) =>
      cmd === "change_password" ? Promise.reject("Ancien mot de passe incorrect") : Promise.resolve([]),
    )
    localStorage.setItem("supercaisse_user", JSON.stringify({ id: 1, login: "admin", nom: "Admin", role: "admin", must_change_password: true }))
    renderProtected()
    await user.type(await screen.findByLabelText("Mot de passe actuel"), "mauvais")
    await user.type(screen.getByLabelText("Nouveau mot de passe"), "Caisse-2026!")
    await user.type(screen.getByLabelText("Confirmer le nouveau mot de passe"), "Caisse-2026!")
    await user.click(screen.getByRole("button", { name: /enregistrer/i }))
    await waitFor(() => expect(mockInvoke).toHaveBeenCalledWith("change_password", expect.anything()))
    expect(screen.queryByTestId("page")).not.toBeInTheDocument()
    expect(screen.getByText("Changement de mot de passe obligatoire")).toBeInTheDocument()
  })
})
