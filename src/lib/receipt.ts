import { formatCurrency, formatDateTime } from "@/lib/utils"
import { EscPosBuilder } from "./escpos"
import { round2 } from "./totaux"
import { jsPDF } from "jspdf"

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
  shopIf?: string | null
  shopRc?: string | null
  shopPatente?: string | null
  logoBase64?: string | null
  docPrimaryColor?: string | null
  receiptHeader?: string | null
  docType?: string
  docNumero?: string | null
  items: Array<{
    designation: string
    quantite: number
    prix_unitaire: number
    tva: number
    total_ligne: number
    montant_tva?: number | null
    remise_ligne?: number
  }>
  montantTotal: number
  montantRemise: number
  netPaye: number
  modePaiement: string
  monnaie: number
}

const DOC_TITLES: Record<string, string> = {
  facture: "FACTURE",
  devis: "DEVIS",
  commande: "COMMANDE",
  bl: "BON DE LIVRAISON",
  avoir: "AVOIR",
}

const HTML_ESCAPES: Record<string, string> = { "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }

export function esc(value: unknown): string {
  return String(value ?? "").replace(/[&<>"']/g, (c) => HTML_ESCAPES[c])
}

export function logoValide(logo: string | null | undefined): logo is string {
  return !!logo && /^[A-Za-z0-9+/]+={0,2}$/.test(logo.replace(/\s/g, ""))
}

export function ventilationTva(data: Pick<ReceiptData, "items" | "montantTotal" | "netPaye">): Record<number, number> {
  const ratio = data.montantTotal !== 0 ? data.netPaye / data.montantTotal : 1
  const acc: Record<number, number> = {}
  for (const item of data.items) {
    if (item.tva <= 0) continue
    const montant = item.montant_tva ?? (item.total_ligne - item.total_ligne / (1 + item.tva / 100)) * ratio
    acc[item.tva] = round2((acc[item.tva] ?? 0) + montant)
  }
  return acc
}

export function generateReceiptHTML(data: ReceiptData): string {
  const itemsRows = data.items.map((item) => `
    <tr>
      <td style="padding:2px 0">${esc(item.designation)}${item.remise_ligne ? ` (-${esc(item.remise_ligne)}%)` : ""}</td>
      <td style="text-align:center;padding:2px 0">${esc(item.quantite)}</td>
      <td style="text-align:right;padding:2px 0">${formatCurrency(item.prix_unitaire)}</td>
      <td style="text-align:right;padding:2px 0">${formatCurrency(item.total_ligne)}</td>
    </tr>
  `).join("")

  // Calcul ventilation TVA
  const tvaBreakdown = ventilationTva(data)

  const tvaRows = Object.entries(tvaBreakdown).map(([taux, montant]) => 
    `<div class="total-line"><span>TVA ${esc(taux)}%</span><span>${formatCurrency(montant)}</span></div>`
  ).join("")

  return `<!DOCTYPE html>
<html lang="fr">
<head>
  <meta charset="UTF-8">
  <title>Ticket #${esc(data.venteId)}</title>
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
  ${logoValide(data.logoBase64) ? `<div class="center" style="margin-bottom:4px"><img src="data:image/png;base64,${data.logoBase64}" style="max-height:40px;max-width:60mm" alt="" /></div>` : ""}
  <div class="center header">${esc(data.shopName)}</div>
  <div class="center infos">
    ${esc(data.shopAddress)}<br>
    ${esc(data.shopPhone)}<br>
    ${data.shopIce ? `ICE: ${esc(data.shopIce)}` : ""}
    ${data.shopIf ? `<br>IF: ${esc(data.shopIf)}` : ""}
    ${data.shopRc ? `<br>RC: ${esc(data.shopRc)}` : ""}
    ${data.shopPatente ? `<br>Patente: ${esc(data.shopPatente)}` : ""}
  </div>
  ${data.receiptHeader ? `<div class="center infos" style="font-style:italic;margin-top:2px">${esc(data.receiptHeader)}</div>` : ""}
  <div class="divider"></div>
  <div class="infos">
    Facture #${esc(data.venteId)}<br>
    ${formatDateTime(data.date)}<br>
    Caissier: ${esc(data.caissier)}<br>
    Client: ${esc(data.client)}
    ${data.clientIce ? `<br>ICE Client: ${esc(data.clientIce)}` : ""}
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
  <div class="total-line"><span>Paiement</span><span>${esc(data.modePaiement)}</span></div>
  ${data.monnaie > 0 ? `<div class="total-line monnaie"><span>Monnaie rendue</span><span>${formatCurrency(data.monnaie)}</span></div>` : ""}
  ${Object.keys(tvaBreakdown).length > 0 ? `
  <div class="divider"></div>
  <div class="infos" style="text-align:center;font-weight:bold;margin-bottom:2px">Ventilation TVA</div>
  ${tvaRows}
  ` : ""}
  <div class="center footer">${esc(data.receiptFooter)}</div>
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
  if (data.shopIf) builder.line(`IF: ${data.shopIf}`)
  if (data.shopRc) builder.line(`RC: ${data.shopRc}`)
  if (data.shopPatente) builder.line(`Patente: ${data.shopPatente}`)

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
  const tvaBreakdown = ventilationTva(data)

  if (Object.keys(tvaBreakdown).length > 0) {
    builder.line("--------------------------------")
    for (const [taux, montant] of Object.entries(tvaBreakdown)) {
      builder.line(`TVA ${taux}% : ${formatCurrency(montant)}`)
    }
  }
  
  builder.line("--------------------------------")
    .align("center")
    .line(data.receiptFooter)
    .cut()
    .openDrawer()
    
  return builder.toBase64()
}

export function generateFacturePdfBase64(data: ReceiptData): string {
  const doc = new jsPDF({ unit: "mm", format: "a4" })
  const pageWidth = doc.internal.pageSize.getWidth()
  const marginX = 15
  let y = 18

  const primaryColor = data.docPrimaryColor || "#2563eb"
  const r = parseInt(primaryColor.slice(1, 3), 16)
  const g = parseInt(primaryColor.slice(3, 5), 16)
  const b = parseInt(primaryColor.slice(5, 7), 16)

  if (logoValide(data.logoBase64)) {
    try {
      doc.addImage(`data:image/png;base64,${data.logoBase64}`, "PNG", marginX, y - 4, 18, 18)
    } catch { /* skip invalid image */ }
  }
  const logoOffset = logoValide(data.logoBase64) ? 22 : 0

  doc.setFont("helvetica", "bold")
  doc.setFontSize(16)
  doc.text(data.shopName, marginX + logoOffset, y)
  y += 6

  doc.setFont("helvetica", "normal")
  doc.setFontSize(9)
  if (data.shopAddress) { doc.text(data.shopAddress, marginX + logoOffset, y); y += 4.5 }
  if (data.shopPhone) { doc.text(data.shopPhone, marginX + logoOffset, y); y += 4.5 }

  const legalMentions = [
    data.shopIce ? `ICE: ${data.shopIce}` : null,
    data.shopIf ? `IF: ${data.shopIf}` : null,
    data.shopRc ? `RC: ${data.shopRc}` : null,
    data.shopPatente ? `Patente: ${data.shopPatente}` : null,
  ].filter(Boolean).join("   ")
  if (legalMentions) { doc.text(legalMentions, marginX, y); y += 4.5 }

  const docTitle = DOC_TITLES[data.docType || "facture"] || "FACTURE"
  doc.setFont("helvetica", "bold")
  doc.setFontSize(14)
  doc.setTextColor(r, g, b)
  doc.text(docTitle, pageWidth - marginX, 18, { align: "right" })
  doc.setTextColor(0, 0, 0)
  doc.setFont("helvetica", "normal")
  doc.setFontSize(10)
  doc.text(`N° ${data.docNumero || `#${data.venteId}`}`, pageWidth - marginX, 25, { align: "right" })
  doc.text(formatDateTime(data.date), pageWidth - marginX, 30, { align: "right" })

  y = Math.max(y, 32) + 4
  doc.setDrawColor(r, g, b)
  doc.line(marginX, y, pageWidth - marginX, y)
  y += 7

  doc.setFont("helvetica", "bold")
  doc.setFontSize(10)
  doc.text("Client", marginX, y)
  y += 5
  doc.setFont("helvetica", "normal")
  doc.text(data.client, marginX, y)
  y += 4.5
  if (data.clientIce) { doc.text(`ICE: ${data.clientIce}`, marginX, y); y += 4.5 }
  doc.text(`Caissier: ${data.caissier}`, marginX, y)
  y += 8

  const colX = { designation: marginX, qte: 110, pu: 130, tva: 152, total: 170 }
  doc.setFillColor(r + Math.round((255 - r) * 0.85), g + Math.round((255 - g) * 0.85), b + Math.round((255 - b) * 0.85))
  doc.rect(marginX, y - 4.5, pageWidth - marginX * 2, 7, "F")
  doc.setFont("helvetica", "bold")
  doc.setFontSize(9)
  doc.text("Désignation", colX.designation + 1, y)
  doc.text("Qté", colX.qte, y, { align: "right" })
  doc.text("PU TTC", colX.pu, y, { align: "right" })
  doc.text("TVA", colX.tva, y, { align: "right" })
  doc.text("Total TTC", colX.total, y, { align: "right" })
  y += 6

  doc.setFont("helvetica", "normal")
  for (const item of data.items) {
    if (y > 270) { doc.addPage(); y = 20 }
    doc.text(item.designation.substring(0, 55), colX.designation + 1, y)
    doc.text(String(item.quantite), colX.qte, y, { align: "right" })
    doc.text(formatCurrency(item.prix_unitaire), colX.pu, y, { align: "right" })
    doc.text(`${item.tva}%`, colX.tva, y, { align: "right" })
    doc.text(formatCurrency(item.total_ligne), colX.total, y, { align: "right" })
    y += 5.5
  }

  y += 2
  doc.setDrawColor(r, g, b)
  doc.line(marginX, y, pageWidth - marginX, y)
  y += 7

  const totalsX = pageWidth - marginX
  doc.setFontSize(9)
  doc.text("Sous-total TTC", totalsX - 45, y)
  doc.text(formatCurrency(data.montantTotal), totalsX, y, { align: "right" })
  y += 5
  if (data.montantRemise > 0) {
    doc.text("Remise", totalsX - 45, y)
    doc.text(`-${formatCurrency(data.montantRemise)}`, totalsX, y, { align: "right" })
    y += 5
  }
  doc.setFont("helvetica", "bold")
  doc.setFontSize(11)
  doc.text("Net à payer", totalsX - 45, y)
  doc.text(formatCurrency(data.netPaye), totalsX, y, { align: "right" })
  y += 8

  const tvaBreakdown = ventilationTva(data)

  if (Object.keys(tvaBreakdown).length > 0) {
    doc.setFont("helvetica", "bold")
    doc.setFontSize(9)
    doc.text("Ventilation TVA", marginX, y)
    y += 5
    doc.setFont("helvetica", "normal")
    for (const [taux, montant] of Object.entries(tvaBreakdown)) {
      doc.text(`TVA ${taux}% : ${formatCurrency(montant)}`, marginX, y)
      y += 4.5
    }
  }

  doc.setFontSize(8)
  doc.setTextColor(120)
  doc.text(data.receiptFooter, marginX, 285)

  return doc.output("datauristring").split(",")[1]
}

export async function saveFacturePdf(data: ReceiptData): Promise<string> {
  const { invoke } = await import("@/lib/tauri")
  const base64Data = generateFacturePdfBase64(data)
  const filename = `${data.docType || "facture"}_${data.docNumero || data.venteId}`
  return invoke<string>("save_document_pdf", { base64Data, filename })
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
