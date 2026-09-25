import { describe, it, expect } from "vitest"
import { ACCES_ROUTES, accesAutorise, routeAccueil, type Habilitations } from "./acces"
import { router } from "./router"

function habilitations(role: string, permissions: Record<string, string[]>): Habilitations {
  return { role, aLaPermission: (module, action) => role === "admin" || (permissions[module] ?? []).includes(action) }
}

const caissier = habilitations("caissier", { ventes: ["voir", "creer"], clients: ["voir"] })
const manager = habilitations("manager", { ventes: ["voir", "creer"], rapports: ["voir"], articles: ["voir"] })

describe("accès aux routes", () => {
  it("follows the permissions table instead of the role alone", () => {
    expect(accesAutorise("/pos", caissier)).toBe(true)
    expect(accesAutorise("/ventes", caissier)).toBe(true)
    expect(accesAutorise("/dashboard", caissier)).toBe(false)
    expect(accesAutorise("/articles", caissier)).toBe(false)
    expect(accesAutorise("/articles", manager)).toBe(true)
    expect(accesAutorise("/stock", manager)).toBe(false)
    expect(accesAutorise("/settings", manager)).toBe(false)
    expect(accesAutorise("/settings", habilitations("admin", {}))).toBe(true)
  })

  it("denies unknown routes to non-admins", () => {
    expect(accesAutorise("/inconnue", manager)).toBe(false)
  })

  it("picks a landing page the user can open", () => {
    expect(routeAccueil(caissier)).toBe("/pos")
    expect(routeAccueil(habilitations("manager", { rapports: ["voir"] }))).toBe("/dashboard")
    expect(routeAccueil(habilitations("caissier", {}))).toBe("/cuisine")
  })

  it("declares a rule for every routed page", () => {
    const racine = router.routes[0].children?.find((r) => !r.path)?.children ?? []
    const chemins = racine.filter((r) => r.path).map((r) => `/${r.path}`)
    expect(chemins.length).toBeGreaterThan(20)
    for (const chemin of chemins) expect(ACCES_ROUTES[chemin], chemin).toBeDefined()
  })
})
