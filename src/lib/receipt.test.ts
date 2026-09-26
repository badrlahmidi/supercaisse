import { describe, it, expect } from "vitest"
import { esc, generateReceiptEscPos, generateReceiptHTML, intituleDocument, logoValide, totauxFiscaux, type ReceiptData } from "./receipt"

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

describe("mentions fiscales", () => {
  const legal: ReceiptData = {
    ...base,
    docType: "facture",
    docNumero: "FA-2026-00042",
    shopIce: "001234567000089",
    shopIf: "12345678",
    shopRc: "RC 4567",
    shopPatente: "34567890",
    clientIce: "009876543000021",
    items: [
      { designation: "Huile", quantite: 1, prix_unitaire: 100, tva: 20, total_ligne: 120, montant_tva: 20 },
      { designation: "Pain", quantite: 2, prix_unitaire: 5, tva: 0, total_ligne: 10, montant_tva: 0 },
      { designation: "Thé", quantite: 1, prix_unitaire: 50, tva: 10, total_ligne: 55, montant_tva: 5 },
    ],
    montantTotal: 185,
    netPaye: 185,
  }

  it("uses the fiscal number instead of the internal id", () => {
    expect(intituleDocument(legal)).toBe("FACTURE N° FA-2026-00042")
    expect(intituleDocument({ docType: "avoir", docNumero: null, venteId: 7 })).toBe("AVOIR N° #7")
    const html = generateReceiptHTML(legal)
    expect(html).toContain("FACTURE N° FA-2026-00042")
    expect(html).not.toContain("Facture #1")
  })

  it("shows HT, TVA per rate and TTC", () => {
    expect(totauxFiscaux(legal)).toEqual({ ht: 160, tva: 25, ttc: 185, tvaParTaux: { 20: 20, 10: 5 } })
    const html = generateReceiptHTML(legal)
    for (const texte of ["Total HT", "TVA 20%", "TVA 10%", "Total TTC", "ICE: 001234567000089", "IF: 12345678", "RC: RC 4567", "Patente: 34567890", "ICE Client: 009876543000021"]) {
      expect(html).toContain(texte)
    }
    const escpos = atob(generateReceiptEscPos(legal))
    for (const texte of ["Total HT", "TVA 20%", "Total TTC", "IF: 12345678", "FA-2026-00042"]) {
      expect(escpos).toContain(texte)
    }
  })

  it("references the source invoice on a credit note", () => {
    const html = generateReceiptHTML({ ...legal, docType: "avoir", docNumero: "AV-2026-00001", docSourceNumero: "FA-2026-00042" })
    expect(html).toContain("AVOIR N° AV-2026-00001")
    expect(html).toContain("Sur facture N° FA-2026-00042")
  })

  it("prefers the stored HT amount", () => {
    expect(totauxFiscaux({ ...legal, montantHT: 160.01 }).ht).toBe(160.01)
  })
})
