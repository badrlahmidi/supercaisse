export const round2 = (n: number) => Math.round(n * 100) / 100

export interface LigneMontants {
  quantite: number
  prix_unitaire: number
  tva: number
  remise_ligne?: number
}

export interface LigneCalculee {
  total_ligne: number
  montant_ht: number
  montant_tva: number
}

export interface TotauxDocument {
  lignes: LigneCalculee[]
  montantTotal: number
  montantHT: number
  montantTVA: number
  netTTC: number
  montantRemise: number
  sousTotalHT: number
  totalTVABrut: number
  tvaParTaux: Record<number, number>
}

export function calculerLigne(ligne: LigneMontants, remiseGlobale = 0): LigneCalculee {
  const remiseLigne = ligne.remise_ligne ?? 0
  const brutHT = round2(ligne.quantite * ligne.prix_unitaire * (1 - remiseLigne / 100))
  const brutTVA = round2(brutHT * ligne.tva / 100)
  const montantHT = round2(ligne.quantite * ligne.prix_unitaire * (1 - remiseLigne / 100) * (1 - remiseGlobale / 100))
  const montantTVA = round2(montantHT * ligne.tva / 100)
  return { total_ligne: round2(brutHT + brutTVA), montant_ht: montantHT, montant_tva: montantTVA }
}

export function calculerTotaux(lignes: LigneMontants[], remiseGlobale = 0): TotauxDocument {
  const calculs = lignes.map((l) => calculerLigne(l, remiseGlobale))
  const bruts = lignes.map((l) => calculerLigne(l, 0))
  const montantTotal = round2(calculs.reduce((s, l) => s + l.total_ligne, 0))
  const montantHT = round2(calculs.reduce((s, l) => s + l.montant_ht, 0))
  const montantTVA = round2(calculs.reduce((s, l) => s + l.montant_tva, 0))
  const netTTC = round2(montantHT + montantTVA)
  const tvaParTaux: Record<number, number> = {}
  lignes.forEach((l, i) => {
    if (l.tva > 0) tvaParTaux[l.tva] = round2((tvaParTaux[l.tva] ?? 0) + calculs[i].montant_tva)
  })
  return {
    lignes: calculs,
    montantTotal,
    montantHT,
    montantTVA,
    netTTC,
    montantRemise: round2(montantTotal - netTTC),
    sousTotalHT: round2(bruts.reduce((s, l) => s + l.montant_ht, 0)),
    totalTVABrut: round2(bruts.reduce((s, l) => s + l.montant_tva, 0)),
    tvaParTaux,
  }
}
