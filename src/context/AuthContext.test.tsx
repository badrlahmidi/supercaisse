import { describe, it, expect, vi } from "vitest"
import { render, screen } from "@testing-library/react"
import { MemoryRouter } from "react-router-dom"
import { AuthProvider, useAuth } from "../context/AuthContext"

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }))

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

  it("restores user from localStorage", async () => {
    localStorage.setItem("supercaisse_user", JSON.stringify({ id: 1, login: "admin", nom: "Admin", role: "admin" }))

    function TestUser() {
      const { user } = useAuth()
      return <div data-testid="username">{user?.nom}</div>
    }

    render(
      <MemoryRouter>
        <AuthProvider>
          <TestUser />
        </AuthProvider>
      </MemoryRouter>
    )
    expect(await screen.findByTestId("username")).toHaveTextContent("Admin")
  })
})
