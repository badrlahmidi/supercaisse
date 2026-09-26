import { describe, it, expect } from "vitest"
import { compteDansCA } from "./ventes"

describe("compteDansCA", () => {
  it("counts invoices, credit notes and unconverted delivery notes", () => {
    expect(compteDansCA({ dtype: "facture", statut: "validee" })).toBe(true)
    expect(compteDansCA({ dtype: "facture", statut: "convertie" })).toBe(true)
    expect(compteDansCA({ dtype: "avoir", statut: "validee" })).toBe(true)
    expect(compteDansCA({ dtype: "bl", statut: "validee" })).toBe(true)
    expect(compteDansCA({ statut: "validee" })).toBe(true)
  })

  it("excludes quotes, orders, cancelled documents and converted delivery notes", () => {
    expect(compteDansCA({ dtype: "devis", statut: "validee" })).toBe(false)
    expect(compteDansCA({ dtype: "commande", statut: "validee" })).toBe(false)
    expect(compteDansCA({ dtype: "facture", statut: "annulee" })).toBe(false)
    expect(compteDansCA({ dtype: "bl", statut: "convertie" })).toBe(false)
  })
})
