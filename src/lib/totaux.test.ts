import { describe, it, expect } from "vitest"
import { calculerLigne, calculerTotaux } from "./totaux"

describe("calculerTotaux", () => {
  it("adds TVA on top of HT prices", () => {
    expect(calculerLigne({ quantite: 2, prix_unitaire: 100, tva: 20 })).toEqual({ total_ligne: 240, montant_ht: 200, montant_tva: 40 })
  })

  it("applies line discounts before TVA", () => {
    expect(calculerLigne({ quantite: 1, prix_unitaire: 100, tva: 20, remise_ligne: 10 })).toEqual({ total_ligne: 108, montant_ht: 90, montant_tva: 18 })
  })

  it("reduces the TVA base with the document discount", () => {
    const t = calculerTotaux([
      { quantite: 1, prix_unitaire: 100, tva: 20, remise_ligne: 10 },
      { quantite: 3, prix_unitaire: 10, tva: 0 },
    ], 10)
    expect(t).toMatchObject({ montantTotal: 138, montantHT: 108, montantTVA: 16.2, netTTC: 124.2, montantRemise: 13.8, sousTotalHT: 120, totalTVABrut: 18 })
    expect(t.tvaParTaux).toEqual({ 20: 16.2 })
  })

  it("matches the Rust calculation to the cent (commands/calcul.rs parity test)", () => {
    const t = calculerTotaux([
      { quantite: 3, prix_unitaire: 3.33, tva: 20 },
      { quantite: 1.5, prix_unitaire: 12.49, tva: 7, remise_ligne: 15 },
      { quantite: 1, prix_unitaire: 9.99, tva: 0 },
      { quantite: 7, prix_unitaire: 1.15, tva: 10, remise_ligne: 3 },
    ], 5)
    expect(t.lignes[0]).toEqual({ total_ligne: 11.99, montant_ht: 9.49, montant_tva: 1.9 })
    expect(t.lignes[1]).toEqual({ total_ligne: 17.03, montant_ht: 15.13, montant_tva: 1.06 })
    expect(t.lignes[3]).toEqual({ total_ligne: 8.59, montant_ht: 7.42, montant_tva: 0.74 })
    expect(t).toMatchObject({ montantTotal: 47.6, montantHT: 41.53, montantTVA: 3.7, netTTC: 45.23, montantRemise: 2.37 })
  })

  it("keeps net = total - remise", () => {
    const t = calculerTotaux([{ quantite: 0.335, prix_unitaire: 89.9, tva: 20, remise_ligne: 7 }], 12.5)
    expect(Math.round((t.montantTotal - t.montantRemise) * 100) / 100).toBe(t.netTTC)
  })
})
