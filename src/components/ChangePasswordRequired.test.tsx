import { describe, it, expect, vi, beforeEach } from "vitest"
import { render, screen, waitFor } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { MemoryRouter } from "react-router-dom"
import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import { useEffect, useState, type ReactNode } from "react"
import { AuthProvider, ProtectedRoute, useAuth, type SessionUser } from "@/context/AuthContext"

const mockInvoke = vi.hoisted(() => vi.fn())
vi.mock("@/lib/tauri", () => ({
  invoke: mockInvoke,
  setSessionToken: vi.fn(),
  getSessionToken: vi.fn(() => null),
  onSessionExpired: vi.fn(() => () => undefined),
}))

vi.mock("sonner", () => ({
  toast: { success: vi.fn(), error: vi.fn() },
}))

function Connexion({ user, children }: { user: SessionUser; children: ReactNode }) {
  const { loginAs } = useAuth()
  const [pret, setPret] = useState(false)
  useEffect(() => {
    loginAs(user).then(() => setPret(true))
  }, [])
  return pret ? <>{children}</> : null
}

function renderProtected(user: SessionUser) {
  const queryClient = new QueryClient({ defaultOptions: { mutations: { retry: false } } })
  return render(
    <QueryClientProvider client={queryClient}>
      <MemoryRouter>
        <AuthProvider>
          <Connexion user={user}>
            <ProtectedRoute>
              <div data-testid="page">Page protégée</div>
            </ProtectedRoute>
          </Connexion>
        </AuthProvider>
      </MemoryRouter>
    </QueryClientProvider>,
  )
}

const aChanger: SessionUser = { id: 1, login: "admin", nom: "Admin", role: "admin", must_change_password: true, token: "tok" }

beforeEach(() => {
  vi.clearAllMocks()
  localStorage.clear()
  mockInvoke.mockImplementation((cmd: string) => Promise.resolve(cmd === "get_permissions" ? [] : undefined))
})

describe("ChangePasswordRequired", () => {
  it("blocks protected pages until the password is changed", async () => {
    renderProtected(aChanger)
    expect(await screen.findByText("Changement de mot de passe obligatoire")).toBeInTheDocument()
    expect(screen.queryByTestId("page")).not.toBeInTheDocument()
  })

  it("shows the page directly when no change is required", async () => {
    renderProtected({ ...aChanger, must_change_password: false })
    expect(await screen.findByTestId("page")).toBeInTheDocument()
  })

  it("validates the form before calling the backend", async () => {
    const user = userEvent.setup()
    renderProtected(aChanger)
    await user.type(await screen.findByLabelText("Mot de passe actuel"), "admin")
    await user.type(screen.getByLabelText("Nouveau mot de passe"), "court")
    await user.type(screen.getByLabelText("Confirmer le nouveau mot de passe"), "autre")
    await user.click(screen.getByRole("button", { name: /enregistrer/i }))
    expect(await screen.findByText("Minimum 8 caractères")).toBeInTheDocument()
    expect(mockInvoke).not.toHaveBeenCalledWith("change_password", expect.anything())
  })

  it("unlocks the app after a successful change", async () => {
    const user = userEvent.setup()
    renderProtected(aChanger)
    await user.type(await screen.findByLabelText("Mot de passe actuel"), "admin")
    await user.type(screen.getByLabelText("Nouveau mot de passe"), "Caisse-2026!")
    await user.type(screen.getByLabelText("Confirmer le nouveau mot de passe"), "Caisse-2026!")
    await user.click(screen.getByRole("button", { name: /enregistrer/i }))
    expect(await screen.findByTestId("page")).toBeInTheDocument()
    expect(mockInvoke).toHaveBeenCalledWith("change_password", {
      ancienMotDePasse: "admin",
      nouveauMotDePasse: "Caisse-2026!",
    })
  })

  it("stays locked when the backend refuses the change", async () => {
    const user = userEvent.setup()
    mockInvoke.mockImplementation((cmd: string) =>
      cmd === "change_password" ? Promise.reject("Ancien mot de passe incorrect") : Promise.resolve([]),
    )
    renderProtected(aChanger)
    await user.type(await screen.findByLabelText("Mot de passe actuel"), "mauvais")
    await user.type(screen.getByLabelText("Nouveau mot de passe"), "Caisse-2026!")
    await user.type(screen.getByLabelText("Confirmer le nouveau mot de passe"), "Caisse-2026!")
    await user.click(screen.getByRole("button", { name: /enregistrer/i }))
    await waitFor(() => expect(mockInvoke).toHaveBeenCalledWith("change_password", expect.anything()))
    expect(screen.queryByTestId("page")).not.toBeInTheDocument()
    expect(screen.getByText("Changement de mot de passe obligatoire")).toBeInTheDocument()
  })
})
