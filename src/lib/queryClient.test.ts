import { describe, it, expect, vi, beforeEach } from "vitest"
import { toast } from "sonner"
import { creerQueryClient } from "./queryClient"

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }))
vi.mock("sonner", () => ({ toast: { error: vi.fn() } }))

beforeEach(() => {
  vi.clearAllMocks()
})

async function muter(options: { erreur: unknown; onError?: () => void }) {
  const client = creerQueryClient()
  const mutation = client.getMutationCache().build(client, {
    mutationFn: () => Promise.reject(options.erreur),
    onError: options.onError,
  })
  await mutation.execute(undefined).catch(() => undefined)
}

describe("creerQueryClient", () => {
  it("signale l'échec d'une mutation qui ne gère pas son erreur", async () => {
    await muter({ erreur: "Stock insuffisant" })
    expect(toast.error).toHaveBeenCalledWith("Opération impossible", { description: "Stock insuffisant" })
  })

  it("laisse une mutation qui gère son erreur l'afficher elle-même", async () => {
    const onError = vi.fn()
    await muter({ erreur: "Refusé", onError })
    expect(onError).toHaveBeenCalled()
    expect(toast.error).not.toHaveBeenCalled()
  })

  it("ne double pas le message de session expirée", async () => {
    await muter({ erreur: "Session invalide ou expirée" })
    expect(toast.error).not.toHaveBeenCalled()
  })

  it("signale une lecture en échec une seule fois par requête, sauf si la page la gère", async () => {
    const client = creerQueryClient()
    await client.fetchQuery({ queryKey: ["clients"], queryFn: () => Promise.reject(new Error("Accès refusé")) }).catch(() => undefined)
    expect(toast.error).toHaveBeenCalledWith("Chargement impossible", { id: '["clients"]', description: "Accès refusé" })
    vi.mocked(toast.error).mockClear()
    await client
      .fetchQuery({ queryKey: ["rapport"], queryFn: () => Promise.reject(new Error("x")), meta: { erreurGeree: true } })
      .catch(() => undefined)
    expect(toast.error).not.toHaveBeenCalled()
    expect(client.getDefaultOptions().queries?.retry).toBe(false)
  })
})
