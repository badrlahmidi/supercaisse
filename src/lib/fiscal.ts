export const DELAI_ANNULATION_MINUTES = 15

const TYPES_FISCAUX = ["facture", "avoir"]

export function estFiscal(dtype: string | null | undefined): boolean {
  return TYPES_FISCAUX.includes(dtype || "facture")
}

export function iceValide(ice: string | null | undefined): boolean {
  return /^\d{15}$/.test((ice ?? "").replace(/\s/g, ""))
}

export function iceSaisieValide(ice: string | null | undefined): boolean {
  return !(ice ?? "").trim() || iceValide(ice)
}

export function ifSaisieValide(identifiant: string | null | undefined): boolean {
  return /^\d{0,15}$/.test((identifiant ?? "").trim())
}

export function mentionsVendeurManquantes(settings: { ice?: string | null; if_number?: string | null; rc_number?: string | null } | null | undefined): string[] {
  const manquantes: string[] = []
  if (!iceValide(settings?.ice)) manquantes.push("ICE")
  if (!settings?.if_number?.trim()) manquantes.push("IF")
  if (!settings?.rc_number?.trim()) manquantes.push("RC")
  return manquantes
}

export type AnnulationDirecte = "libre" | "motif" | "avoir"

export function annulationDirecte(vente: { dtype?: string | null; date: string }, maintenant: Date = new Date()): AnnulationDirecte {
  if (!estFiscal(vente.dtype)) return "libre"
  const emission = new Date(vente.date.replace(" ", "T"))
  const minutes = (maintenant.getTime() - emission.getTime()) / 60000
  return Number.isNaN(minutes) || minutes > DELAI_ANNULATION_MINUTES ? "avoir" : "motif"
}
