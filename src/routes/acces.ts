export interface RegleAcces {
  module?: string
  action?: string
  roles?: string[]
}

export const ACCES_ROUTES: Record<string, RegleAcces> = {
  "/pos": { module: "ventes", action: "creer" },
  "/dashboard": { module: "rapports" },
  "/articles": { module: "articles" },
  "/categories": { module: "categories" },
  "/clients": { module: "clients" },
  "/paiements": { module: "clients" },
  "/fournisseurs": { module: "fournisseurs" },
  "/reappro": { module: "reappro" },
  "/ventes": { module: "ventes" },
  "/achats": { module: "achats" },
  "/comparaison-prix": { module: "achats" },
  "/rapprochement": { module: "achats" },
  "/stock": { module: "stock" },
  "/stock/mouvements": { module: "stock" },
  "/stock/peremptions": { module: "stock" },
  "/inventaire": { module: "inventaire" },
  "/journal": { module: "journal" },
  "/cheques": { module: "cheques" },
  "/rapports": { module: "rapports" },
  "/caisses": { module: "journal" },
  "/cuisine": {},
  "/magasins": { module: "magasins" },
  "/boutiques": { roles: ["admin"] },
  "/veille-dgi": { roles: ["admin"] },
  "/peripheriques": { roles: ["admin"] },
  "/audit": { module: "audit" },
  "/settings": { roles: ["admin"] },
}

const ORDRE_ACCUEIL = ["/pos", "/dashboard", "/ventes", "/clients", "/cuisine"]

export interface Habilitations {
  role: string
  aLaPermission: (module: string, action: string) => boolean
}

export function accesAutorise(chemin: string, habilitations: Habilitations): boolean {
  const regle = ACCES_ROUTES[chemin]
  if (!regle) return habilitations.role === "admin"
  if (regle.roles && !regle.roles.includes(habilitations.role)) return false
  if (regle.module) return habilitations.aLaPermission(regle.module, regle.action ?? "voir")
  return true
}

export function routeAccueil(habilitations: Habilitations): string | null {
  const candidates = [...ORDRE_ACCUEIL, ...Object.keys(ACCES_ROUTES)]
  return candidates.find((chemin) => accesAutorise(chemin, habilitations)) ?? null
}
