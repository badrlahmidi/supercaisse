import { describe, it, expect, vi, beforeEach } from "vitest"
import { render, screen, fireEvent, waitFor } from "@testing-library/react"
import { MemoryRouter } from "react-router-dom"
import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import Caisses from "./Caisses"
import type { SessionSupervision } from "@/types/generated/SessionSupervision"

const mockInvoke = vi.hoisted(() => vi.fn())
vi.mock("@/lib/tauri", () => ({ invoke: mockInvoke }))

function Wrapper({ children }: { children: React.ReactNode }) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  return (
    <QueryClientProvider client={qc}>
      <MemoryRouter>{children}</MemoryRouter>
    </QueryClientProvider>
  )
}

const ouverte: SessionSupervision = {
  id: 7,
  caissier_id: 2,
  caissier_nom: "Samira",
  magasin_nom: "Principal",
  statut: "ouverte",
  date_ouverture: "2026-09-25 08:00:00",
  date_cloture: null,
  fond_initial: 200,
  recettes_especes: 70,
  recettes_cb: 50,
  recettes_cheque: 0,
  recettes_virement: 0,
  sorties: 15,
  entrees: 10,
  especes_attendu: null,
  especes_declare: null,
  ecart: null,
}

const cloturee: SessionSupervision = {
  ...ouverte,
  id: 6,
  caissier_nom: "Youssef",
  statut: "cloturee",
  date_cloture: "2026-09-24 20:00:00",
  especes_attendu: 265,
  especes_declare: 250,
  ecart: -15,
}

beforeEach(() => {
  vi.clearAllMocks()
  mockInvoke.mockImplementation((cmd: string) => {
    if (cmd === "get_caisses") return Promise.resolve([ouverte, cloturee])
    if (cmd === "get_tresorerie") return Promise.resolve(null)
    return Promise.resolve(null)
  })
})

describe("Caisses", () => {
  it("liste les sessions de caisse du point de vente", async () => {
    render(<Caisses />, { wrapper: Wrapper })
    expect(await screen.findByText("Samira")).toBeInTheDocument()
    expect(screen.getByText("Youssef")).toBeInTheDocument()
    expect(screen.getByText("Clôturée")).toBeInTheDocument()
    expect(screen.queryByText("Ouvrir une caisse")).not.toBeInTheDocument()
  })

  it("clôture une session avec les espèces comptées", async () => {
    render(<Caisses />, { wrapper: Wrapper })
    fireEvent.click(await screen.findByRole("button", { name: /Clôturer/ }))
    const champ = screen.getByLabelText("Espèces comptées (DH)") as HTMLInputElement
    expect(champ.value).toBe("265")
    fireEvent.change(champ, { target: { value: "260" } })
    fireEvent.click(screen.getByRole("button", { name: /Confirmer la clôture/ }))
    await waitFor(() =>
      expect(mockInvoke).toHaveBeenCalledWith("close_session", { sessionId: 7, totalEspecesDeclare: 260 })
    )
  })
})
