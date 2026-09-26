const CORRECTION_DEMI_CENTIME = 1e-6

export function versCentimes(n: number): number {
  const centimes = n * 100
  return Math.round(centimes + Math.sign(centimes) * CORRECTION_DEMI_CENTIME) + 0
}

export const round2 = (n: number) => versCentimes(n) / 100

export const sommeDH = (montants: number[]) => montants.reduce((s, m) => s + versCentimes(m), 0) / 100

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
  const montantTotal = sommeDH(calculs.map((l) => l.total_ligne))
  const montantHT = sommeDH(calculs.map((l) => l.montant_ht))
  const montantTVA = sommeDH(calculs.map((l) => l.montant_tva))
  const netTTC = sommeDH([montantHT, montantTVA])
  const tvaParTaux: Record<number, number> = {}
  lignes.forEach((l, i) => {
    if (l.tva > 0) tvaParTaux[l.tva] = sommeDH([tvaParTaux[l.tva] ?? 0, calculs[i].montant_tva])
  })
  return {
    lignes: calculs,
    montantTotal,
    montantHT,
    montantTVA,
    netTTC,
    montantRemise: sommeDH([montantTotal, -netTTC]),
    sousTotalHT: sommeDH(bruts.map((l) => l.montant_ht)),
    totalTVABrut: sommeDH(bruts.map((l) => l.montant_tva)),
    tvaParTaux,
  }
}
