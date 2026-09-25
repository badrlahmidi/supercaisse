import { describe, it, expect } from "vitest"
import { annulationDirecte, iceSaisieValide, iceValide, ifSaisieValide, mentionsVendeurManquantes } from "./fiscal"

describe("fiscal", () => {
  it("validates the ICE format", () => {
    expect(iceValide("001234567000089")).toBe(true)
    expect(iceValide("001 234567 000089")).toBe(true)
    expect(iceValide("12345")).toBe(false)
    expect(iceSaisieValide("")).toBe(true)
    expect(iceSaisieValide("ABC")).toBe(false)
    expect(ifSaisieValide("1234")).toBe(true)
    expect(ifSaisieValide("IF-12")).toBe(false)
  })

  it("lists the missing seller mentions", () => {
    expect(mentionsVendeurManquantes({})).toEqual(["ICE", "IF", "RC"])
    expect(mentionsVendeurManquantes({ ice: "001234567000089", if_number: "12", rc_number: "RC 1" })).toEqual([])
  })

  it("limits direct cancellation of fiscal documents", () => {
    const maintenant = new Date(2026, 8, 25, 12, 0, 0)
    expect(annulationDirecte({ dtype: "devis", date: "2026-01-01 10:00:00" }, maintenant)).toBe("libre")
    expect(annulationDirecte({ dtype: "bl", date: "2026-01-01 10:00:00" }, maintenant)).toBe("libre")
    expect(annulationDirecte({ dtype: "facture", date: "2026-09-25 11:50:00" }, maintenant)).toBe("motif")
    expect(annulationDirecte({ dtype: "facture", date: "2026-09-25 11:40:00" }, maintenant)).toBe("avoir")
    expect(annulationDirecte({ dtype: "avoir", date: "2026-09-25 11:59:00" }, maintenant)).toBe("motif")
  })
})
