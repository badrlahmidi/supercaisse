export interface DocumentVente {
  dtype?: string | null
  statut?: string | null
}

export function compteDansCA(v: DocumentVente): boolean {
  const dtype = v.dtype || "facture"
  if (v.statut === "annulee") return false
  if (dtype === "facture" || dtype === "avoir") return true
  return dtype === "bl" && v.statut !== "convertie"
}
