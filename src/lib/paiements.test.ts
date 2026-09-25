import { describe, it, expect } from "vitest"
import { buildPaiements } from "./paiements"

describe("buildPaiements", () => {
  it("returns a single payment for the net amount when not split", () => {
    expect(buildPaiements([], "especes", 120.004)).toEqual([{ mode: "especes", montant: 120 }])
  })

  it("maps split amounts to montant", () => {
    expect(buildPaiements([{ mode: "especes", amount: 50 }, { mode: "carte", amount: 70 }], "especes", 120)).toEqual([
      { mode: "especes", montant: 50 },
      { mode: "carte", montant: 70 },
    ])
  })

  it("removes the change given back from the cash part", () => {
    expect(buildPaiements([{ mode: "carte", amount: 70 }, { mode: "especes", amount: 100 }], "especes", 120)).toEqual([
      { mode: "carte", montant: 70 },
      { mode: "especes", montant: 50 },
    ])
  })

  it("keeps exact cents when the split parts sum to the net amount", () => {
    const splits = [
      { mode: "especes", amount: 33.33 },
      { mode: "cb", amount: 33.33 },
      { mode: "cheque", amount: 33.34 },
    ]
    const p = buildPaiements(splits, "especes", 100)
    expect(p.map((x) => x.montant)).toEqual([33.33, 33.33, 33.34])
  })

  it("computes the change to the cent", () => {
    const p = buildPaiements([{ mode: "cb", amount: 0.1 }, { mode: "especes", amount: 0.3 }], "especes", 0.3)
    expect(p).toEqual([{ mode: "cb", montant: 0.1 }, { mode: "especes", montant: 0.2 }])
  })

  it("never records a negative amount", () => {
    expect(buildPaiements([], "credit", -5)).toEqual([{ mode: "credit", montant: 0 }])
  })

  it("adds the loyalty points as a fidelite payment", () => {
    expect(buildPaiements([], "especes", 100, 20)).toEqual([
      { mode: "especes", montant: 100 },
      { mode: "fidelite", montant: 20 },
    ])
    expect(buildPaiements([{ mode: "carte", amount: 60 }, { mode: "especes", amount: 50 }], "especes", 100, 20)).toEqual([
      { mode: "carte", montant: 60 },
      { mode: "especes", montant: 40 },
      { mode: "fidelite", montant: 20 },
    ])
  })
})
