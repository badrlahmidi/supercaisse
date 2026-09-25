export interface PaiementVente {
  mode: string
  montant: number
}

const round2 = (n: number) => Math.round(n * 100) / 100

export function buildPaiements(
  splits: { mode: string; amount: number }[],
  paymentMode: string,
  netAmount: number,
): PaiementVente[] {
  if (splits.length === 0) {
    return [{ mode: paymentMode, montant: round2(Math.max(0, netAmount)) }]
  }
  let rendu = Math.max(0, splits.reduce((s, p) => s + p.amount, 0) - netAmount)
  return splits.map((s) => {
    let montant = s.amount
    if (s.mode === "especes" && rendu > 0) {
      const deduit = Math.min(rendu, montant)
      montant -= deduit
      rendu -= deduit
    }
    return { mode: s.mode, montant: round2(montant) }
  })
}
