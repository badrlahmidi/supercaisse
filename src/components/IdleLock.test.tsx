import { describe, it, expect, vi, beforeEach, afterEach } from "vitest"
import { render, screen, fireEvent, act } from "@testing-library/react"
import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import IdleLock from "./IdleLock"

const mockInvoke = vi.hoisted(() => vi.fn())
const loginAs = vi.hoisted(() => vi.fn())
vi.mock("@/lib/tauri", () => ({ invoke: mockInvoke }))
vi.mock("@/hooks/useSettings", () => ({ useAppSettings: () => ({ data: { idle_timeout: "1" } }) }))
vi.mock("@/context/AuthContext", () => ({
  useAuth: () => ({ user: { id: 2, login: "karim", nom: "Karim", role: "caissier" }, loginAs }),
}))

function Wrapper({ children }: { children: React.ReactNode }) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  return <QueryClientProvider client={qc}>{children}</QueryClientProvider>
}

async function verrouiller() {
  render(<IdleLock><p>Caisse</p></IdleLock>, { wrapper: Wrapper })
  await act(async () => { vi.advanceTimersByTime(1500) })
  await act(async () => { await vi.runOnlyPendingTimersAsync() })
  await act(async () => { await vi.runOnlyPendingTimersAsync() })
  vi.useRealTimers()
  expect(await screen.findByText("Session verrouillée")).toBeInTheDocument()
}

function taper(pin: string) {
  for (const chiffre of pin) fireEvent.click(screen.getByRole("button", { name: chiffre }))
}

beforeEach(() => {
  vi.clearAllMocks()
  vi.useFakeTimers()
})

afterEach(() => {
  vi.useRealTimers()
})

describe("IdleLock", () => {
  it("unlocks another account with its login and PIN", async () => {
    mockInvoke.mockImplementation((cmd: string) =>
      Promise.resolve(cmd === "get_comptes_pin"
        ? [{ login: "gerant", nom: "Gérant" }, { login: "karim", nom: "Karim" }]
        : { id: 3, login: "gerant", nom: "Gérant", role: "manager", token: "t" }),
    )
    await verrouiller()
    const select = await screen.findByLabelText("Compte")
    expect(select).toHaveValue("karim")
    fireEvent.change(select, { target: { value: "gerant" } })
    const valider = screen.getByRole("button", { name: "Valider le PIN" })
    taper("482")
    expect(valider).toBeDisabled()
    taper("61")
    fireEvent.click(valider)
    expect(mockInvoke).toHaveBeenCalledWith("login_pin", { login: "gerant", pin: "48261" })
    await vi.waitFor(() => expect(loginAs).toHaveBeenCalledWith(expect.objectContaining({ login: "gerant" })))
    expect(screen.queryByText("Session verrouillée")).not.toBeInTheDocument()
  })

  it("offers the PIN accounts even when the current user has none", async () => {
    mockInvoke.mockImplementation((cmd: string) =>
      Promise.resolve(cmd === "get_comptes_pin" ? [{ login: "gerant", nom: "Gérant" }] : null),
    )
    await verrouiller()
    const select = await screen.findByLabelText("Compte")
    expect(select).toHaveValue("karim")
    expect(screen.getAllByRole("option").map((o) => o.textContent)).toEqual(["Karim", "Gérant"])
  })

  it("shows the lockout message from the backend", async () => {
    mockInvoke.mockImplementation((cmd: string) =>
      cmd === "get_comptes_pin"
        ? Promise.resolve([])
        : Promise.reject("PIN bloqué après 5 essais incorrects : réessayez dans 5 minutes ou utilisez le mot de passe"),
    )
    await verrouiller()
    taper("4826")
    fireEvent.click(screen.getByRole("button", { name: "Valider le PIN" }))
    expect(mockInvoke).toHaveBeenCalledWith("login_pin", { login: "karim", pin: "4826" })
    expect(await screen.findByText(/PIN bloqué/)).toBeInTheDocument()
    expect(loginAs).not.toHaveBeenCalled()
  })
})
