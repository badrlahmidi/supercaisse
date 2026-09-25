import { describe, it, expect, vi, beforeEach } from "vitest"
import { render, screen, waitFor } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { MemoryRouter } from "react-router-dom"
import type { ReactNode } from "react"
import { AuthProvider } from "@/context/AuthContext"
import Login from "./Login"

const mockInvoke = vi.hoisted(() => vi.fn())
vi.mock("@/lib/tauri", () => ({ invoke: mockInvoke }))

vi.mock("sonner", () => ({
  toast: { success: vi.fn(), error: vi.fn() },
}))

function Wrapper({ children }: { children: ReactNode }) {
  return (
    <MemoryRouter>
      <AuthProvider>{children}</AuthProvider>
    </MemoryRouter>
  )
}

beforeEach(() => {
  vi.clearAllMocks()
  localStorage.clear()
  mockInvoke.mockImplementation((cmd: string) => Promise.resolve(cmd === "get_permissions" ? [] : null))
})

describe("Login", () => {
  it("renders login form", async () => {
    render(<Login />, { wrapper: Wrapper })
    expect((await screen.findAllByText("SuperCaisse"))[0]).toBeInTheDocument()
    expect(screen.getByText("Connexion")).toBeInTheDocument()
    expect(screen.getByLabelText("Login")).toBeInTheDocument()
    expect(screen.getByLabelText("Mot de passe")).toBeInTheDocument()
    expect(screen.getByRole("button", { name: /se connecter/i })).toBeInTheDocument()
  })

  it("shows default credentials hint in development builds", async () => {
    render(<Login />, { wrapper: Wrapper })
    expect(await screen.findByText("admin / admin")).toBeInTheDocument()
  })

  it("shows validation errors on empty submit", async () => {
    const user = userEvent.setup()
    render(<Login />, { wrapper: Wrapper })
    await user.click(await screen.findByRole("button", { name: /se connecter/i }))
    await waitFor(() => {
      expect(screen.getByText("Le login est requis")).toBeInTheDocument()
      expect(screen.getByText("Le mot de passe est requis")).toBeInTheDocument()
    })
  })

  it("logs the user in through the auth context", async () => {
    const user = userEvent.setup()
    mockInvoke.mockImplementation((cmd: string) =>
      Promise.resolve(
        cmd === "login"
          ? { id: 1, login: "admin", nom: "Administrateur", role: "admin", must_change_password: true }
          : [],
      ),
    )
    render(<Login />, { wrapper: Wrapper })
    await user.type(await screen.findByLabelText("Login"), "admin")
    await user.type(screen.getByLabelText("Mot de passe"), "admin")
    await user.click(screen.getByRole("button", { name: /se connecter/i }))
    await waitFor(() => {
      const stored = JSON.parse(localStorage.getItem("supercaisse_user")!)
      expect(stored).toMatchObject({ login: "admin", nom: "Administrateur", role: "admin", must_change_password: true })
    })
    expect(mockInvoke).toHaveBeenCalledWith("get_permissions", { role: "admin" })
  })
})
