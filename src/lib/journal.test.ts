import { describe, it, expect, vi, beforeEach } from "vitest"

const invokeMock = vi.fn()

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}))

describe("journal", () => {
  beforeEach(() => {
    vi.resetModules()
    invokeMock.mockReset()
    invokeMock.mockResolvedValue(undefined)
    Object.defineProperty(window, "__TAURI_INTERNALS__", { value: {}, configurable: true })
  })

  it("envoie le message au backend sans jeton de session", async () => {
    const { journaliser } = await import("./journal")
    journaliser("error", "boom")
    expect(invokeMock).toHaveBeenCalledWith("journaliser_frontend", { niveau: "error", message: "boom" })
  })

  it("limite le nombre de messages par minute", async () => {
    const { journaliser } = await import("./journal")
    for (let i = 0; i < 100; i++) journaliser("warn", `m${i}`)
    expect(invokeMock).toHaveBeenCalledTimes(30)
  })

  it("ne propage pas un échec de journalisation", async () => {
    invokeMock.mockRejectedValue(new Error("ipc"))
    const { journaliser } = await import("./journal")
    expect(() => journaliser("info", "x")).not.toThrow()
  })

  it("formate les erreurs avec leur pile", async () => {
    const { formaterErreur } = await import("./journal")
    const e = new Error("cassé")
    expect(formaterErreur(e)).toContain("cassé")
    expect(formaterErreur({ a: 1 })).toBe('{"a":1}')
    expect(formaterErreur("texte")).toBe("texte")
  })

  it("capture les promesses rejetées non gérées", async () => {
    const { installerJournalGlobal } = await import("./journal")
    installerJournalGlobal()
    const evenement = new Event("unhandledrejection") as Event & { reason: unknown }
    evenement.reason = new Error("rejet")
    window.dispatchEvent(evenement)
    expect(invokeMock).toHaveBeenCalledWith(
      "journaliser_frontend",
      expect.objectContaining({ niveau: "error", message: expect.stringContaining("rejet") }),
    )
  })
})
