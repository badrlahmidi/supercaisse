import { describe, it, expect } from "vitest"
import { esc, generateReceiptHTML, logoValide, type ReceiptData } from "./receipt"

const base: ReceiptData = {
  shopName: "Épicerie",
  shopAddress: "Rue 1",
  shopPhone: "0600",
  receiptFooter: "Merci",
  venteId: 1,
  date: "2026-09-25T10:00:00",
  caissier: "Karim",
  client: "Client de passage",
  items: [{ designation: "Huile", quantite: 1, prix_unitaire: 100, tva: 20, total_ligne: 120 }],
  montantTotal: 120,
  montantRemise: 0,
  netPaye: 120,
  modePaiement: "especes",
  monnaie: 0,
}

describe("generateReceiptHTML", () => {
  it("escapes HTML special characters", () => {
    expect(esc(`<img src=x onerror="alert(1)">&'`)).toBe("&lt;img src=x onerror=&quot;alert(1)&quot;&gt;&amp;&#39;")
    expect(esc(null)).toBe("")
  })

  it("never renders user-controlled text as HTML", () => {
    const payload = `<img src=x onerror="window.__TAURI_INTERNALS__.invoke('import_database')">`
    const html = generateReceiptHTML({
      ...base,
      shopName: payload,
      shopAddress: payload,
      receiptHeader: payload,
      receiptFooter: payload,
      caissier: payload,
      client: payload,
      clientIce: payload,
      modePaiement: payload,
      items: [{ ...base.items[0], designation: payload }],
    })
    expect(html).not.toContain("<img src=x")
    expect(html).not.toContain('onerror="')
    expect(html).toContain("&lt;img src=x onerror=")
  })

  it("keeps a valid logo and drops a malformed one", () => {
    expect(logoValide("iVBORw0KGgo=")).toBe(true)
    expect(logoValide(`x" onerror="alert(1)`)).toBe(false)
    expect(generateReceiptHTML({ ...base, logoBase64: "iVBORw0KGgo=" })).toContain("data:image/png;base64,iVBORw0KGgo=")
    expect(generateReceiptHTML({ ...base, logoBase64: `x" onerror="alert(1)` })).not.toContain("<img")
  })
})
