import { describe, it, expect, vi, beforeEach } from "vitest"
import { render, screen, fireEvent, waitFor } from "@testing-library/react"
import { MemoryRouter, Route, Routes, useNavigate } from "react-router-dom"
import { AuthProvider, ProtectedRoute, useAuth } from "../context/AuthContext"
import { getSessionToken, invoke, setSessionToken } from "@/lib/tauri"

const tauriInvoke = vi.hoisted(() => vi.fn())
vi.mock("@tauri-apps/api/core", () => ({ invoke: tauriInvoke }))
vi.mock("sonner", () => ({ toast: { success: vi.fn(), error: vi.fn() } }))

beforeEach(() => {
  tauriInvoke.mockReset()
  localStorage.clear()
  setSessionToken(null)
})

describe("AuthProvider", () => {
  it("renders children", async () => {
    render(
      <MemoryRouter>
        <AuthProvider>
          <div data-testid="child">Hello</div>
        </AuthProvider>
      </MemoryRouter>
    )
    expect(await screen.findByTestId("child")).toHaveTextContent("Hello")
  })

  it("does not trust a user stored in localStorage", async () => {
    localStorage.setItem("supercaisse_user", JSON.stringify({ id: 1, login: "admin", nom: "Admin", role: "admin" }))

    function TestUser() {
      const { user } = useAuth()
      return <div data-testid="username">{user?.nom ?? "anonyme"}</div>
    }

    render(
      <MemoryRouter>
        <AuthProvider>
          <TestUser />
        </AuthProvider>
      </MemoryRouter>
    )
    expect(await screen.findByTestId("username")).toHaveTextContent("anonyme")
    expect(localStorage.getItem("supercaisse_user")).toBeNull()
  })

  it("keeps the session token in memory only and revokes it on logout", async () => {
    tauriInvoke.mockResolvedValue([])

    function TestSession() {
      const { user, loginAs, logout } = useAuth()
      return (
        <div>
          <div data-testid="username">{user?.nom ?? "anonyme"}</div>
          <button onClick={() => loginAs({ id: 1, login: "admin", nom: "Admin", role: "admin", token: "tok-1" })}>login</button>
          <button onClick={logout}>logout</button>
        </div>
      )
    }

    render(
      <MemoryRouter>
        <AuthProvider>
          <TestSession />
        </AuthProvider>
      </MemoryRouter>
    )
    fireEvent.click(await screen.findByText("login"))
    await waitFor(() => expect(screen.getByTestId("username")).toHaveTextContent("Admin"))
    expect(getSessionToken()).toBe("tok-1")
    expect(JSON.stringify(localStorage)).not.toContain("tok-1")
    expect(tauriInvoke).toHaveBeenCalledWith("get_permissions", { role: "admin", token: "tok-1" })

    fireEvent.click(screen.getByText("logout"))
    await waitFor(() => expect(screen.getByTestId("username")).toHaveTextContent("anonyme"))
    expect(tauriInvoke).toHaveBeenCalledWith("logout", { token: "tok-1" })
    expect(getSessionToken()).toBeNull()
  })

  it("logs out when the backend reports an expired session", async () => {
    function TestSession() {
      const { user, loginAs } = useAuth()
      return (
        <div>
          <div data-testid="username">{user?.nom ?? "anonyme"}</div>
          <button onClick={() => loginAs({ id: 1, login: "admin", nom: "Admin", role: "admin", token: "tok-2" })}>login</button>
        </div>
      )
    }

    tauriInvoke.mockResolvedValue([])
    render(
      <MemoryRouter>
        <AuthProvider>
          <TestSession />
        </AuthProvider>
      </MemoryRouter>
    )
    fireEvent.click(await screen.findByText("login"))
    await waitFor(() => expect(screen.getByTestId("username")).toHaveTextContent("Admin"))
    tauriInvoke.mockRejectedValue("Session invalide ou expirée : veuillez vous reconnecter")
    await expect(invoke("get_categories")).rejects.toBeTruthy()
    await waitFor(() => expect(screen.getByTestId("username")).toHaveTextContent("anonyme"))
  })

  it("routes according to the permissions table", async () => {
    tauriInvoke.mockImplementation((cmd: string) =>
      Promise.resolve(cmd === "get_permissions"
        ? [
            { module: "ventes", action: "voir", allowed: true },
            { module: "ventes", action: "creer", allowed: false },
            { module: "articles", action: "voir", allowed: false },
          ]
        : null),
    )

    function Connexion() {
      const { user, loginAs } = useAuth()
      const navigate = useNavigate()
      return user
        ? <button onClick={() => navigate("/articles")}>articles</button>
        : <button onClick={() => loginAs({ id: 5, login: "gerant", nom: "Gérant", role: "manager", token: "tok-3" })}>login</button>
    }

    render(
      <MemoryRouter initialEntries={["/login"]}>
        <AuthProvider>
          <Connexion />
          <Routes>
            <Route path="/articles" element={<ProtectedRoute chemin="/articles"><p>Page articles</p></ProtectedRoute>} />
            <Route path="/ventes" element={<ProtectedRoute chemin="/ventes"><p>Page ventes</p></ProtectedRoute>} />
            <Route path="/login" element={<p>Page login</p>} />
          </Routes>
        </AuthProvider>
      </MemoryRouter>
    )
    fireEvent.click(await screen.findByText("login"))
    fireEvent.click(await screen.findByText("articles"))
    expect(await screen.findByText("Page ventes")).toBeInTheDocument()
    expect(screen.queryByText("Page articles")).not.toBeInTheDocument()
  })
})
