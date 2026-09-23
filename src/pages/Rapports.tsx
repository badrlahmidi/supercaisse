import { useState } from "react"
import { useQuery } from "@tanstack/react-query"
import { invoke } from "@/lib/tauri"
import { Card, CardContent, CardHeader, CardTitle } from "@/ui/Card"
import { Button } from "@/ui/Button"
import { Input } from "@/ui/Input"
import { Label } from "@/ui/Label"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/ui/Table"
import { Badge } from "@/ui/Badge"
import { toast } from "sonner"
import { Download, TrendingUp, DollarSign, ReceiptText, Percent, Loader2 } from "lucide-react"
import PageHeader from "@/components/PageHeader"
import { formatCurrency, formatDate, exportCSV } from "@/lib/utils"
import { jsPDF } from "jspdf"

interface RapportDetaille {
  ca_total: number
  total_remises: number
  nb_ventes: number
  marge_brute: number
  tva_collectee: number
  top_articles: Array<{ designation: string; quantite: number; total: number }>
  rotation_stock: Array<{ designation: string; quantite_vendue: number; stock_actuel: number }>
  ventes_par_jour: Array<{ jour: string; total: number; nb: number }>
  par_mode: Array<{ mode: string; total: number; nb: number }>
}

function todayISO() {
  return new Date().toISOString().slice(0, 10)
}
function thirtyDaysAgoISO() {
  const d = new Date()
  d.setDate(d.getDate() - 30)
  return d.toISOString().slice(0, 10)
}

export default function Rapports() {
  const [debut, setDebut] = useState(thirtyDaysAgoISO)
  const [fin, setFin] = useState(todayISO)

  const { data: rapport, isLoading } = useQuery({
    queryKey: ["rapport_detaille", debut, fin],
    queryFn: () => invoke<RapportDetaille>("get_rapport_detaille", { debut, fin }),
  })

  const maxVente = rapport?.ventes_par_jour?.length
    ? Math.max(...rapport.ventes_par_jour.map((v) => v.total), 1)
    : 1

  const exportPDF = () => {
    if (!rapport) return
    const doc = new jsPDF({ unit: "mm", format: "a4" })
    const w = doc.internal.pageSize.getWidth()
    let y = 20

    doc.setFontSize(16)
    doc.text("Rapport d’activité détaillé", w / 2, y, { align: "center" })
    y += 8
    doc.setFontSize(10)
    doc.text(`Période : du ${debut} au ${fin}`, w / 2, y, { align: "center" })
    y += 12

    doc.setFontSize(12)
    doc.text("Indicateurs clés", 15, y)
    y += 7
    doc.setFontSize(10)
    const kpis = [
      ["Chiffre d’affaires", formatCurrency(rapport.ca_total)],
      ["Marge brute", formatCurrency(rapport.marge_brute)],
      ["TVA collectée", formatCurrency(rapport.tva_collectee)],
      ["Nombre de ventes", String(rapport.nb_ventes)],
      ["Total remises", formatCurrency(rapport.total_remises)],
    ]
    kpis.forEach(([label, val]) => {
      doc.text(`${label} : ${val}`, 20, y)
      y += 6
    })
    y += 4

    doc.setFontSize(12)
    doc.text("Top 10 articles", 15, y)
    y += 7
    doc.setFontSize(9)
    rapport.top_articles.forEach((a, i) => {
      doc.text(`${i + 1}. ${a.designation} — Qté: ${a.quantite} — CA: ${formatCurrency(a.total)}`, 20, y)
      y += 5
      if (y > 270) { doc.addPage(); y = 20 }
    })
    y += 4

    if (rapport.par_mode.length) {
      doc.setFontSize(12)
      doc.text("Ventilation par mode de paiement", 15, y)
      y += 7
      doc.setFontSize(9)
      rapport.par_mode.forEach((m) => {
        doc.text(`${m.mode} : ${formatCurrency(m.total)} (${m.nb} transactions)`, 20, y)
        y += 5
        if (y > 270) { doc.addPage(); y = 20 }
      })
    }

    doc.save(`rapport_${debut}_${fin}.pdf`)
    toast.success("PDF exporté")
  }

  const exportRapportCSV = () => {
    if (!rapport) return
    const headers = ["Jour", "CA", "Nb ventes"]
    const rows = rapport.ventes_par_jour.map((v) => [v.jour, String(v.total), String(v.nb)])
    exportCSV(headers, rows, `rapport_${debut}_${fin}.csv`)
    toast.success("CSV exporté")
  }

  return (
    <div className="space-y-6">
      <PageHeader title="Rapports détaillés" description="Analyse de la performance commerciale">
        <div className="flex gap-2">
          <Button variant="outline" onClick={exportRapportCSV} disabled={!rapport}>
            <Download className="h-4 w-4 mr-2" />
            CSV
          </Button>
          <Button variant="outline" onClick={exportPDF} disabled={!rapport}>
            <Download className="h-4 w-4 mr-2" />
            PDF
          </Button>
        </div>
      </PageHeader>

      <Card>
        <CardContent className="pt-6">
          <div className="flex gap-4 items-end flex-wrap">
            <div className="space-y-1">
              <Label>Début</Label>
              <Input type="date" value={debut} onChange={(e) => setDebut(e.target.value)} className="w-44" />
            </div>
            <div className="space-y-1">
              <Label>Fin</Label>
              <Input type="date" value={fin} onChange={(e) => setFin(e.target.value)} className="w-44" />
            </div>
          </div>
        </CardContent>
      </Card>

      {isLoading && (
        <div className="flex items-center justify-center py-12">
          <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
        </div>
      )}

      {rapport && (
        <>
          <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-5">
            <Card>
              <CardContent className="pt-6">
                <div className="flex items-center gap-2 text-sm text-muted-foreground">
                  <DollarSign className="h-4 w-4" />
                  Chiffre d'affaires
                </div>
                <p className="text-2xl font-bold mt-1">{formatCurrency(rapport.ca_total)}</p>
              </CardContent>
            </Card>
            <Card>
              <CardContent className="pt-6">
                <div className="flex items-center gap-2 text-sm text-muted-foreground">
                  <TrendingUp className="h-4 w-4" />
                  Marge brute
                </div>
                <p className="text-2xl font-bold mt-1">{formatCurrency(rapport.marge_brute)}</p>
              </CardContent>
            </Card>
            <Card>
              <CardContent className="pt-6">
                <div className="flex items-center gap-2 text-sm text-muted-foreground">
                  <Percent className="h-4 w-4" />
                  TVA collectée
                </div>
                <p className="text-2xl font-bold mt-1">{formatCurrency(rapport.tva_collectee)}</p>
              </CardContent>
            </Card>
            <Card>
              <CardContent className="pt-6">
                <div className="flex items-center gap-2 text-sm text-muted-foreground">
                  <ReceiptText className="h-4 w-4" />
                  Ventes
                </div>
                <p className="text-2xl font-bold mt-1">{rapport.nb_ventes}</p>
              </CardContent>
            </Card>
            <Card>
              <CardContent className="pt-6">
                <div className="flex items-center gap-2 text-sm text-muted-foreground">
                  <DollarSign className="h-4 w-4" />
                  Remises
                </div>
                <p className="text-2xl font-bold mt-1">{formatCurrency(rapport.total_remises)}</p>
              </CardContent>
            </Card>
          </div>

          <div className="grid gap-6 lg:grid-cols-2">
            <Card>
              <CardHeader>
                <CardTitle className="text-base">Ventes par jour</CardTitle>
              </CardHeader>
              <CardContent>
                {rapport.ventes_par_jour.length > 0 ? (
                  <div className="space-y-2">
                    {rapport.ventes_par_jour.map((v) => (
                      <div key={v.jour} className="flex items-center gap-3">
                        <span className="text-xs text-muted-foreground w-20 shrink-0">{formatDate(v.jour)}</span>
                        <div className="flex-1 h-6 bg-muted rounded-full overflow-hidden">
                          <div
                            className="h-full bg-primary rounded-full transition-all"
                            style={{ width: `${(v.total / maxVente) * 100}%` }}
                          />
                        </div>
                        <span className="text-sm font-medium w-24 text-right shrink-0">{formatCurrency(v.total)}</span>
                      </div>
                    ))}
                  </div>
                ) : (
                  <p className="text-sm text-muted-foreground text-center py-8">Aucune vente sur cette période</p>
                )}
              </CardContent>
            </Card>

            <Card>
              <CardHeader>
                <CardTitle className="text-base">Ventilation par mode de paiement</CardTitle>
              </CardHeader>
              <CardContent>
                {rapport.par_mode.length > 0 ? (
                  <Table>
                    <TableHeader>
                      <TableRow>
                        <TableHead>Mode</TableHead>
                        <TableHead className="text-right">Montant</TableHead>
                        <TableHead className="text-right">Nb</TableHead>
                      </TableRow>
                    </TableHeader>
                    <TableBody>
                      {rapport.par_mode.map((m) => (
                        <TableRow key={m.mode}>
                          <TableCell>
                            <Badge variant="outline">{m.mode}</Badge>
                          </TableCell>
                          <TableCell className="text-right font-medium">{formatCurrency(m.total)}</TableCell>
                          <TableCell className="text-right">{m.nb}</TableCell>
                        </TableRow>
                      ))}
                    </TableBody>
                  </Table>
                ) : (
                  <p className="text-sm text-muted-foreground text-center py-8">Aucune donnée</p>
                )}
              </CardContent>
            </Card>
          </div>

          <div className="grid gap-6 lg:grid-cols-2">
            <Card>
              <CardHeader>
                <CardTitle className="text-base">Top 10 articles vendus</CardTitle>
              </CardHeader>
              <CardContent>
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead>#</TableHead>
                      <TableHead>Article</TableHead>
                      <TableHead className="text-right">Qté</TableHead>
                      <TableHead className="text-right">CA</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {rapport.top_articles.map((a, i) => (
                      <TableRow key={i}>
                        <TableCell className="text-muted-foreground">{i + 1}</TableCell>
                        <TableCell className="font-medium">{a.designation}</TableCell>
                        <TableCell className="text-right">{a.quantite}</TableCell>
                        <TableCell className="text-right">{formatCurrency(a.total)}</TableCell>
                      </TableRow>
                    ))}
                    {!rapport.top_articles.length && (
                      <TableRow>
                        <TableCell colSpan={4} className="text-center py-6 text-muted-foreground">
                          Aucun article vendu
                        </TableCell>
                      </TableRow>
                    )}
                  </TableBody>
                </Table>
              </CardContent>
            </Card>

            <Card>
              <CardHeader>
                <CardTitle className="text-base">Rotation de stock (top 20)</CardTitle>
              </CardHeader>
              <CardContent>
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead>Article</TableHead>
                      <TableHead className="text-right">Vendus</TableHead>
                      <TableHead className="text-right">Stock</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {rapport.rotation_stock.map((r, i) => (
                      <TableRow key={i}>
                        <TableCell className="font-medium">{r.designation}</TableCell>
                        <TableCell className="text-right">{r.quantite_vendue}</TableCell>
                        <TableCell className="text-right">{r.stock_actuel}</TableCell>
                      </TableRow>
                    ))}
                    {!rapport.rotation_stock.length && (
                      <TableRow>
                        <TableCell colSpan={3} className="text-center py-6 text-muted-foreground">
                          Aucune donnée
                        </TableCell>
                      </TableRow>
                    )}
                  </TableBody>
                </Table>
              </CardContent>
            </Card>
          </div>
        </>
      )}
    </div>
  )
}
