import { describe, it, expect, vi, beforeEach } from "vitest"
import { render, screen, fireEvent, waitFor } from "@testing-library/react"
import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import MisesAJour from "./MisesAJour"

const mockInvoke = vi.hoisted(() => vi.fn())
vi.mock("@/lib/tauri", () => ({ invoke: mockInvoke }))

function Wrapper({ children }: { children: React.ReactNode }) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  return <QueryClientProvider client={qc}>{children}</QueryClientProvider>
}

beforeEach(() => {
  vi.clearAllMocks()
})

describe("MisesAJour", () => {
  it("shows the build version", () => {
    render(<MisesAJour />, { wrapper: Wrapper })
    expect(screen.getByText(`SuperCaisse ${__APP_VERSION__}`)).toBeInTheDocument()
  })

  it("explains when updates are not configured", async () => {
    mockInvoke.mockResolvedValue({ version_actuelle: "0.9.0", configuree: false, disponible: null })
    render(<MisesAJour />, { wrapper: Wrapper })
    fireEvent.click(screen.getByRole("button", { name: /Rechercher/ }))
    expect(await screen.findByText(/non configurées/)).toBeInTheDocument()
  })

  it("installs an available update", async () => {
    mockInvoke.mockImplementation((cmd: string) =>
      Promise.resolve(cmd === "verifier_mise_a_jour"
        ? { version_actuelle: "0.9.0", configuree: true, disponible: { version: "0.9.1", date: null, notes: "Corrections" } }
        : undefined),
    )
    render(<MisesAJour />, { wrapper: Wrapper })
    fireEvent.click(screen.getByRole("button", { name: /Rechercher/ }))
    expect(await screen.findByText("Version 0.9.1 disponible")).toBeInTheDocument()
    expect(screen.getByText("Corrections")).toBeInTheDocument()
    fireEvent.click(screen.getByRole("button", { name: /Installer et redémarrer/ }))
    await waitFor(() => expect(mockInvoke).toHaveBeenCalledWith("installer_mise_a_jour"))
  })

  it("reports an up-to-date installation", async () => {
    mockInvoke.mockResolvedValue({ version_actuelle: "0.9.0", configuree: true, disponible: null })
    render(<MisesAJour />, { wrapper: Wrapper })
    fireEvent.click(screen.getByRole("button", { name: /Rechercher/ }))
    expect(await screen.findByText("SuperCaisse est à jour.")).toBeInTheDocument()
  })
})
