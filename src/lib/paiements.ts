export interface PaiementVente {
  mode: string
  montant: number
}

import { round2, sommeDH } from "@/lib/totaux"

export function buildPaiements(
  splits: { mode: string; amount: number }[],
  paymentMode: string,
  netAmount: number,
  fidelite = 0,
): PaiementVente[] {
  const avecFidelite = (paiements: PaiementVente[]) =>
    fidelite > 0 ? [...paiements, { mode: "fidelite", montant: round2(fidelite) }] : paiements
  if (splits.length === 0) {
    return avecFidelite([{ mode: paymentMode, montant: round2(Math.max(0, netAmount)) }])
  }
  let rendu = Math.max(0, sommeDH([...splits.map((p) => p.amount), -netAmount]))
  return avecFidelite(splits.map((s) => {
    let montant = s.amount
    if (s.mode === "especes" && rendu > 0) {
      const deduit = Math.min(rendu, montant)
      montant = sommeDH([montant, -deduit])
      rendu = sommeDH([rendu, -deduit])
    }
    return { mode: s.mode, montant: round2(montant) }
  }))
}
