import { describe, it, expect, vi, beforeEach } from "vitest"
import { render, screen, waitFor } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { MemoryRouter } from "react-router-dom"
import Login from "./Login"

const mockInvoke = vi.hoisted(() => vi.fn())
vi.mock("@/lib/tauri", () => ({ invoke: mockInvoke }))

vi.mock("sonner", () => ({
  toast: { success: vi.fn(), error: vi.fn() },
}))

beforeEach(() => {
  vi.clearAllMocks()
  localStorage.clear()
})

describe("Login", () => {
  it("renders login form", () => {
    render(<Login />, { wrapper: MemoryRouter })
    expect(screen.getAllByText("SuperCaisse")[0]).toBeInTheDocument()
    expect(screen.getByText("Connexion")).toBeInTheDocument()
    expect(screen.getByLabelText("Login")).toBeInTheDocument()
    expect(screen.getByLabelText("Mot de passe")).toBeInTheDocument()
    expect(screen.getByRole("button", { name: /se connecter/i })).toBeInTheDocument()
  })

  it("shows default credentials hint", () => {
    render(<Login />, { wrapper: MemoryRouter })
    expect(screen.getByText("admin / admin")).toBeInTheDocument()
  })

  it("shows validation errors on empty submit", async () => {
    const user = userEvent.setup()
    render(<Login />, { wrapper: MemoryRouter })
    await user.click(screen.getByRole("button", { name: /se connecter/i }))
    await waitFor(() => {
      expect(screen.getByText("Le login est requis")).toBeInTheDocument()
      expect(screen.getByText("Le mot de passe est requis")).toBeInTheDocument()
    })
  })

  it("stores user in localStorage on successful login", async () => {
    const user = userEvent.setup()
    mockInvoke.mockResolvedValue({ id: 1, login: "admin", nom: "Administrateur", role: "admin" })
    render(<Login />, { wrapper: MemoryRouter })
    await user.type(screen.getByLabelText("Login"), "admin")
    await user.type(screen.getByLabelText("Mot de passe"), "admin")
    await user.click(screen.getByRole("button", { name: /se connecter/i }))
    await waitFor(() => {
      const stored = JSON.parse(localStorage.getItem("supercaisse_user")!)
      expect(stored).toMatchObject({ login: "admin", nom: "Administrateur", role: "admin" })
    })
  })
})
