import { formatCurrency, formatDateTime } from "@/lib/utils"
import { EscPosBuilder } from "./escpos"

export interface ReceiptData {
  shopName: string
  shopAddress: string
  shopPhone: string
  receiptFooter: string
  venteId: number
  date: string
  caissier: string
  client: string
  clientIce?: string | null
  shopIce?: string | null
  items: Array<{
    designation: string
    quantite: number
    prix_unitaire: number
    tva: number
    total_ligne: number
    remise_ligne?: number
  }>
  montantTotal: number
  montantRemise: number
  netPaye: number
  modePaiement: string
  monnaie: number
}

export function generateReceiptHTML(data: ReceiptData): string {
  const itemsRows = data.items.map((item) => `
    <tr>
      <td style="padding:2px 0">${item.designation}${item.remise_ligne ? ` (-${item.remise_ligne}%)` : ""}</td>
      <td style="text-align:center;padding:2px 0">${item.quantite}</td>
      <td style="text-align:right;padding:2px 0">${formatCurrency(item.prix_unitaire)}</td>
      <td style="text-align:right;padding:2px 0">${formatCurrency(item.total_ligne)}</td>
    </tr>
  `).join("")

  // Calcul ventilation TVA
  const tvaBreakdown = data.items.reduce((acc, item) => {
    if (item.tva > 0) {
      const baseLigne = item.total_ligne / (1 + item.tva / 100)
      const montantTva = item.total_ligne - baseLigne
      acc[item.tva] = (acc[item.tva] || 0) + montantTva
    }
    return acc
  }, {} as Record<number, number>)

  const tvaRows = Object.entries(tvaBreakdown).map(([taux, montant]) => 
    `<div class="total-line"><span>TVA ${taux}%</span><span>${formatCurrency(montant)}</span></div>`
  ).join("")

  return `<!DOCTYPE html>
<html lang="fr">
<head>
  <meta charset="UTF-8">
  <title>Ticket #${data.venteId}</title>
  <style>
    @page { margin: 0; size: 80mm auto; }
    * { margin: 0; padding: 0; box-sizing: border-box; }
    body {
      font-family: 'Courier New', monospace;
      font-size: 12px;
      width: 80mm;
      padding: 10px 8px;
      color: #000;
    }
    .center { text-align: center; }
    .header { font-size: 16px; font-weight: bold; margin-bottom: 4px; }
    .infos { font-size: 11px; margin-bottom: 6px; }
    .divider { border-top: 1px dashed #000; margin: 6px 0; }
    table { width: 100%; border-collapse: collapse; font-size: 11px; }
    th { border-bottom: 1px solid #000; padding: 4px 0; text-align: left; }
    th.right { text-align: right; }
    th.center { text-align: center; }
    .total-line { display: flex; justify-content: space-between; font-size: 12px; padding: 2px 0; }
    .net { font-size: 16px; font-weight: bold; margin: 4px 0; }
    .footer { font-size: 10px; margin-top: 8px; }
    .monnaie { font-size: 14px; font-weight: bold; margin: 4px 0; }
    @media print {
      .no-print { display: none; }
    }
  </style>
</head>
<body>
  <div class="center header">${data.shopName}</div>
  <div class="center infos">
    ${data.shopAddress}<br>
    ${data.shopPhone}<br>
    ${data.shopIce ? `ICE: ${data.shopIce}` : ""}
  </div>
  <div class="divider"></div>
  <div class="infos">
    Facture #${data.venteId}<br>
    ${formatDateTime(data.date)}<br>
    Caissier: ${data.caissier}<br>
    Client: ${data.client}
    ${data.clientIce ? `<br>ICE Client: ${data.clientIce}` : ""}
  </div>
  <div class="divider"></div>
  <table>
    <thead>
      <tr>
        <th>Article</th>
        <th class="center">Qté</th>
        <th class="right">P.U</th>
        <th class="right">Total</th>
      </tr>
    </thead>
    <tbody>
      ${itemsRows}
    </tbody>
  </table>
  <div class="divider"></div>
  <div class="total-line"><span>Sous-total TTC</span><span>${formatCurrency(data.montantTotal)}</span></div>
  ${data.montantRemise > 0 ? `<div class="total-line"><span>Remise</span><span>-${formatCurrency(data.montantRemise)}</span></div>` : ""}
  <div class="divider"></div>
  <div class="total-line net"><span>Net à payer</span><span>${formatCurrency(data.netPaye)}</span></div>
  <div class="total-line"><span>Paiement</span><span>${data.modePaiement}</span></div>
  ${data.monnaie > 0 ? `<div class="total-line monnaie"><span>Monnaie rendue</span><span>${formatCurrency(data.monnaie)}</span></div>` : ""}
  ${Object.keys(tvaBreakdown).length > 0 ? `
  <div class="divider"></div>
  <div class="infos" style="text-align:center;font-weight:bold;margin-bottom:2px">Ventilation TVA</div>
  ${tvaRows}
  ` : ""}
  <div class="center footer">${data.receiptFooter}</div>
</body>
</html>`
}

export function printReceipt(data: ReceiptData) {
  const html = generateReceiptHTML(data)
  const win = window.open("", "_blank", "width=400,height=600")
  if (!win) return
  win.document.write(html)
  win.document.close()
  win.focus()
  setTimeout(() => win.print(), 300)
}

export function generateReceiptEscPos(data: ReceiptData): string {
  const builder = new EscPosBuilder()
  
  builder.align("center")
    .bold(true).line(data.shopName).bold(false)
    .line(data.shopAddress)
    .line(data.shopPhone)
  
  if (data.shopIce) builder.line(`ICE: ${data.shopIce}`)
  
  builder.line("--------------------------------")
    .align("left")
    .line(`Document #${data.venteId}`)
    .line(formatDateTime(data.date))
    .line(`Caissier: ${data.caissier}`)
    .line(`Client: ${data.client}`)
    
  if (data.clientIce) builder.line(`ICE Client: ${data.clientIce}`)
  
  builder.line("--------------------------------")
  
  // En-tête tableau simplifié
  builder.line("Article        Qte  Prix   Total")
  builder.line("--------------------------------")
  
  for (const item of data.items) {
    const desig = item.designation.substring(0, 14).padEnd(14)
    const qte = item.quantite.toString().padStart(3)
    const pu = item.prix_unitaire.toFixed(2).padStart(6)
    const total = item.total_ligne.toFixed(2).padStart(7)
    builder.line(`${desig} ${qte} ${pu} ${total}`)
    if (item.remise_ligne) {
      builder.line(`  Remise: -${item.remise_ligne}%`)
    }
  }
  
  builder.line("--------------------------------")
    .align("right")
    .line(`Total Brut: ${formatCurrency(data.montantTotal)}`)
  
  if (data.montantRemise > 0) {
    builder.line(`Remise: -${formatCurrency(data.montantRemise)}`)
  }
  
  builder.bold(true).size(2, 2).line(`NET: ${formatCurrency(data.netPaye)}`).size(1, 1).bold(false)
    .line(`Mode: ${data.modePaiement.toUpperCase()}`)
  
  if (data.monnaie > 0) {
    builder.line(`Monnaie rendue: ${formatCurrency(data.monnaie)}`)
  }

  // TVA
  const tvaBreakdown = data.items.reduce((acc, item) => {
    if (item.tva > 0) {
      const baseLigne = item.total_ligne / (1 + item.tva / 100)
      const montantTva = item.total_ligne - baseLigne
      acc[item.tva] = (acc[item.tva] || 0) + montantTva
    }
    return acc
  }, {} as Record<number, number>)

  if (Object.keys(tvaBreakdown).length > 0) {
    builder.line("--------------------------------")
    for (const [taux, montant] of Object.entries(tvaBreakdown)) {
      builder.line(`TVA ${taux}% : ${formatCurrency(montant)}`)
    }
  }
  
  builder.line("--------------------------------")
    .align("center")
    .line(data.receiptFooter)
    .line("Merci de votre visite")
    .cut()
    .openDrawer()
    
  return builder.toBase64()
}

export async function printViaTauri(data: ReceiptData) {
  try {
    const { invoke } = await import("@/lib/tauri")
    // Tentative d'impression native ESC/POS
    const base64Data = generateReceiptEscPos(data)
    await invoke("print_escpos", { base64Data })
  } catch (e) {
    console.error("Erreur impression ESC/POS:", e)
    // Fallback HTML (navigateur) si erreur
    printReceipt(data)
  }
}
