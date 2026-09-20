import { useState } from "react"
import { useQuery } from "@tanstack/react-query"
import { invoke } from "@/lib/tauri"
import { useSalesList, useCancelSale } from "@/hooks/useSales"
import { Card, CardContent } from "@/ui/Card"
import { Button } from "@/ui/Button"
import { Input } from "@/ui/Input"
import { Badge } from "@/ui/Badge"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/ui/Table"
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from "@/ui/Dialog"
import { Label } from "@/ui/Label"
import { formatCurrency, formatDateTime, exportCSV } from "@/lib/utils"
import { saveFacturePdf, type ReceiptData } from "@/lib/receipt"
import { Search, Eye, ReceiptText, Loader2, Download, SearchX, Ban, AlertTriangle, MessageCircle, FileText } from "lucide-react"
import { format, subDays } from "date-fns"
import { toast } from "sonner"
import PageHeader from "@/components/PageHeader"
import EmptyState from "@/components/EmptyState"
import type { Settings } from "@/lib/tauri"

interface Vente {
  id: number
  date: string
  client_id: number | null
  caissier_id: number | null
  montant_total: number
  montant_remise: number
  mode_paiement: string
  statut: string
  dtype: string
  numero_facture: string | null
  client_nom: string | null
  caissier_nom: string | null
  client_telephone?: string | null
  client_ice?: string | null
}

interface VenteDetail {
  vente: Vente
  lignes: Array<{
    id: number
    article_id: number
    designation: string
    quantite: number
    prix_unitaire: number
    tva: number
    total_ligne: number
    remise_ligne?: number | null
  }>
}

export default function Ventes() {
  const [search, setSearch] = useState("")
  const [dateDebut, setDateDebut] = useState<string>(format(subDays(new Date(), 30), "yyyy-MM-dd"))
  const [dateFin, setDateFin] = useState<string>(format(new Date(), "yyyy-MM-dd"))
  const [selectedVente, setSelectedVente] = useState<VenteDetail | null>(null)
  const [showDetail, setShowDetail] = useState(false)
  const [cancelConfirm, setCancelConfirm] = useState<Vente | null>(null)
  const [generatingPdfId, setGeneratingPdfId] = useState<number | null>(null)

  const { data: ventes, isLoading } = useSalesList(dateDebut, dateFin)
  const cancelMutation = useCancelSale()

  const { data: settings } = useQuery({
    queryKey: ["settings"],
    queryFn: () => invoke<Settings>("get_settings"),
  })

  const loadDetail = async (venteId: number) => {
    try {
      const detail = await invoke<VenteDetail>("get_vente_details", { venteId })
      setSelectedVente(detail)
      setShowDetail(true)
    } catch (err) {
      console.error(err)
    }
  }

  const generatePdf = async (venteId: number) => {
    setGeneratingPdfId(venteId)
    try {
      const detail = await invoke<VenteDetail>("get_vente_details", { venteId })
      const netPaye = detail.vente.montant_total - detail.vente.montant_remise
      const data: ReceiptData = {
        shopName: settings?.shop_name || "SuperCaisse",
        shopAddress: settings?.shop_address || "",
        shopPhone: settings?.shop_phone || "",
        shopIce: settings?.ice || null,
        shopIf: settings?.if_number || null,
        shopRc: settings?.rc_number || null,
        shopPatente: settings?.patente || null,
        receiptFooter: settings?.receipt_footer || "Merci de votre visite",
        venteId: detail.vente.id,
        docType: detail.vente.dtype,
        docNumero: detail.vente.numero_facture,
        date: detail.vente.date,
        caissier: detail.vente.caissier_nom || "",
        client: detail.vente.client_nom || "Client de passage",
        clientIce: detail.vente.client_ice,
        items: detail.lignes.map((l) => ({
          designation: l.designation,
          quantite: l.quantite,
          prix_unitaire: l.prix_unitaire,
          tva: l.tva,
          total_ligne: l.total_ligne,
          remise_ligne: l.remise_ligne || 0,
        })),
        montantTotal: detail.vente.montant_total,
        montantRemise: detail.vente.montant_remise,
        netPaye,
        modePaiement: detail.vente.mode_paiement,
        monnaie: 0,
      }
      const path = await saveFacturePdf(data)
      toast.success(`PDF généré : ${path}`)
    } catch (err) {
      toast.error(`Échec de la génération du PDF : ${String(err)}`)
    } finally {
      setGeneratingPdfId(null)
    }
  }

  const sendWhatsAppInvoice = (vente: Vente) => {
    if (!vente.client_telephone) return
    const phone = vente.client_telephone.replace(/\s+/g, "").replace(/^0/, "212")
    const docType = vente.dtype.toUpperCase()
    const amount = formatCurrency(vente.montant_total - vente.montant_remise)
    const ref = vente.numero_facture || `#${vente.id}`
    const text = encodeURIComponent(`Bonjour ${vente.client_nom || ""},\n\nVoici le récapitulatif de votre ${docType} ${ref}.\n\nMontant Total : ${amount}\n\nMerci de votre confiance et à bientôt !`)
    window.open(`https://wa.me/${phone}?text=${text}`, "_blank")
  }

  const filteredVentes = ventes?.filter((v) =>
    v.client_nom?.toLowerCase().includes(search.toLowerCase()) ||
    v.caissier_nom?.toLowerCase().includes(search.toLowerCase()) ||
    v.numero_facture?.toLowerCase().includes(search.toLowerCase()) ||
    v.id.toString().includes(search) ||
    v.mode_paiement.toLowerCase().includes(search.toLowerCase())
  ) || []

  const totalVentes = filteredVentes.reduce((s, v) => s + (v.montant_total - v.montant_remise), 0)
  const nbVentes = filteredVentes.length

  return (
    <div className="space-y-6">
      <PageHeader title="Documents de Vente" description="Historique des factures, BL et devis">
        <Button variant="outline" onClick={() => {
          const headers = ["Référence", "Type", "Date", "Client", "Caissier", "Total", "Remise", "Mode paiement", "Statut"]
          const rows = filteredVentes.map((v) => [
            v.numero_facture || String(v.id), v.dtype, formatDateTime(v.date), v.client_nom || "Client de passage",
            v.caissier_nom || "", formatCurrency(v.montant_total), formatCurrency(v.montant_remise),
            v.mode_paiement, v.statut,
          ])
          exportCSV(headers, rows, `documents_${dateDebut}_${dateFin}.csv`)
        }}>
          <Download className="h-4 w-4 mr-2" />
          Exporter
        </Button>
      </PageHeader>

      {/* Stats & Filters */}
      <div className="grid gap-4 md:grid-cols-4">
        <Card>
          <CardContent className="pt-6">
            <div className="flex items-center justify-between">
              <div>
                <p className="text-sm text-muted-foreground">Total ventes</p>
                <p className="text-2xl font-bold">{formatCurrency(totalVentes)}</p>
              </div>
              <div className="p-3 bg-primary/10 rounded-xl">
                <ReceiptText className="h-6 w-6 text-primary" />
              </div>
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-6">
            <div className="flex items-center justify-between">
              <div>
                <p className="text-sm text-muted-foreground">Nombre de ventes</p>
                <p className="text-2xl font-bold">{nbVentes}</p>
              </div>
              <div className="p-3 bg-success/10 rounded-xl">
                <ReceiptText className="h-6 w-6 text-success" />
              </div>
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-6">
            <div className="space-y-2">
              <Label htmlFor="dateDebut">Date début</Label>
              <Input
                id="dateDebut"
                type="date"
                value={dateDebut}
                onChange={(e) => setDateDebut(e.target.value)}
                className="w-full"
              />
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-6">
            <div className="space-y-2">
              <Label htmlFor="dateFin">Date fin</Label>
              <Input
                id="dateFin"
                type="date"
                value={dateFin}
                onChange={(e) => setDateFin(e.target.value)}
                className="w-full"
              />
            </div>
          </CardContent>
        </Card>
      </div>

      <Card>
        <CardContent className="pt-6">
          <div className="flex gap-4 mb-4">
            <div className="relative flex-1 max-w-md">
              <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
              <Input
                placeholder="Rechercher (client, caissier, mode paiement, ID)..."
                value={search}
                onChange={(e) => setSearch(e.target.value)}
                className="pl-10"
              />
            </div>
          </div>

          <div className="overflow-x-auto">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead className="w-[120px]">Référence</TableHead>
                  <TableHead>Type</TableHead>
                  <TableHead>Date</TableHead>
                  <TableHead>Client</TableHead>
                  <TableHead>Caissier</TableHead>
                  <TableHead className="text-right">Total</TableHead>
                  <TableHead>Remise</TableHead>
                  <TableHead>Paiement</TableHead>
                  <TableHead>Statut</TableHead>
                  <TableHead className="w-[60px]">Actions</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {isLoading ? (
                  <TableRow>
                    <TableCell colSpan={9} className="text-center py-8">
                      <Loader2 className="h-8 w-8 animate-spin mx-auto" />
                    </TableCell>
                  </TableRow>
                ) : filteredVentes.map((vente) => (
                  <TableRow key={vente.id}>
                    <TableCell className="font-medium">{vente.numero_facture || `#${vente.id}`}</TableCell>
                    <TableCell>
                      <Badge variant="secondary" className="capitalize">{vente.dtype}</Badge>
                    </TableCell>
                    <TableCell className="text-sm">{formatDateTime(vente.date)}</TableCell>
                    <TableCell>{vente.client_nom || "Client de passage"}</TableCell>
                    <TableCell>{vente.caissier_nom || "—"}</TableCell>
                    <TableCell className="text-right font-medium">{formatCurrency(vente.montant_total)}</TableCell>
                    <TableCell>{vente.montant_remise > 0 ? `-${formatCurrency(vente.montant_remise)}` : "—"}</TableCell>
                    <TableCell>
                      <Badge variant="outline" className="capitalize">{vente.mode_paiement}</Badge>
                    </TableCell>
                    <TableCell>
                      <Badge variant={vente.statut === "validee" ? "success" : "warning"} className="capitalize">
                        {vente.statut}
                      </Badge>
                    </TableCell>
                    <TableCell>
                      <div className="flex items-center gap-1">
                        <Button variant="ghost" size="icon" onClick={() => loadDetail(vente.id)} title="Voir détails">
                          <Eye className="h-4 w-4" />
                        </Button>
                        <Button
                          variant="ghost"
                          size="icon"
                          onClick={() => generatePdf(vente.id)}
                          disabled={generatingPdfId === vente.id}
                          title="Générer le PDF"
                        >
                          {generatingPdfId === vente.id ? (
                            <Loader2 className="h-4 w-4 animate-spin" />
                          ) : (
                            <FileText className="h-4 w-4" />
                          )}
                        </Button>
                        {vente.client_telephone && (
                          <Button variant="ghost" size="icon" onClick={() => sendWhatsAppInvoice(vente)} title="Envoyer par WhatsApp">
                            <MessageCircle className="h-4 w-4 text-[#25D366]" />
                          </Button>
                        )}
                        {vente.statut !== "annulee" && (
                          <Button variant="ghost" size="icon" onClick={() => setCancelConfirm(vente)} title="Annuler le document">
                            <Ban className="h-4 w-4 text-destructive" />
                          </Button>
                        )}
                      </div>
                    </TableCell>
                  </TableRow>
                ))}
                {!isLoading && !filteredVentes.length && (
                  <TableRow>
                    <TableCell colSpan={9}>
                      <EmptyState icon={<SearchX className="h-12 w-12" />} title="Aucune vente trouvée" description="Aucune vente dans cette période" />
                    </TableCell>
                  </TableRow>
                )}
              </TableBody>
            </Table>
          </div>
        </CardContent>
      </Card>

      <Dialog open={showDetail} onOpenChange={setShowDetail}>
        <DialogContent className="max-w-3xl max-h-[90vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle>Détail du document {selectedVente?.vente.numero_facture || `#${selectedVente?.vente.id}`}</DialogTitle>
          </DialogHeader>
          {selectedVente && (
            <div className="space-y-4">
              <div className="grid gap-4 md:grid-cols-4 text-sm">
                <div>
                  <p className="text-muted-foreground">Date</p>
                  <p className="font-medium">{formatDateTime(selectedVente.vente.date)}</p>
                </div>
                <div>
                  <p className="text-muted-foreground">Client</p>
                  <p className="font-medium">{selectedVente.vente.client_nom || "Client de passage"}</p>
                </div>
                <div>
                  <p className="text-muted-foreground">Caissier</p>
                  <p className="font-medium">{selectedVente.vente.caissier_nom || "—"}</p>
                </div>
                <div>
                  <p className="text-muted-foreground">Mode paiement</p>
                  <p className="font-medium capitalize">{selectedVente.vente.mode_paiement}</p>
                </div>
              </div>
              <div className="border-t" />
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Article</TableHead>
                    <TableHead className="text-right">Qté</TableHead>
                    <TableHead className="text-right">Prix unitaire</TableHead>
                    <TableHead>TVA</TableHead>
                    <TableHead className="text-right">Total</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {selectedVente.lignes.map((ligne) => (
                    <TableRow key={ligne.id}>
                      <TableCell>{ligne.designation}</TableCell>
                      <TableCell className="text-right">{ligne.quantite}</TableCell>
                      <TableCell className="text-right">{formatCurrency(ligne.prix_unitaire)}</TableCell>
                      <TableCell>{ligne.tva}%</TableCell>
                      <TableCell className="text-right font-medium">{formatCurrency(ligne.total_ligne)}</TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
              <div className="border-t pt-4 space-y-2 text-right">
                <div className="flex justify-between">
                  <span>Sous-total</span>
                  <span>{formatCurrency(selectedVente.vente.montant_total)}</span>
                </div>
                {selectedVente.vente.montant_remise > 0 && (
                  <div className="flex justify-between text-success">
                    <span>Remise</span>
                    <span>-{formatCurrency(selectedVente.vente.montant_remise)}</span>
                  </div>
                )}
                <div className="flex justify-between text-lg font-bold border-t pt-2">
                  <span>Net payé</span>
                  <span>{formatCurrency(selectedVente.vente.montant_total - selectedVente.vente.montant_remise)}</span>
                </div>
              </div>
            </div>
          )}
          <DialogFooter>
            <Button variant="outline" onClick={() => setShowDetail(false)}>
              Fermer
            </Button>
            {selectedVente && (
              <Button
                onClick={() => generatePdf(selectedVente.vente.id)}
                disabled={generatingPdfId === selectedVente.vente.id}
              >
                {generatingPdfId === selectedVente.vente.id ? (
                  <Loader2 className="h-4 w-4 animate-spin mr-2" />
                ) : (
                  <FileText className="h-4 w-4 mr-2" />
                )}
                Générer PDF
              </Button>
            )}
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog open={!!cancelConfirm} onOpenChange={() => setCancelConfirm(null)}>
        <DialogContent className="max-w-sm">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2 text-destructive">
              <AlertTriangle className="h-5 w-5" />
              Annuler {cancelConfirm?.numero_facture || `#${cancelConfirm?.id}`}
            </DialogTitle>
          </DialogHeader>
          <p className="text-sm text-muted-foreground">
            Êtes-vous sûr de vouloir annuler ce document ? Si c'est une facture ou un BL, le stock sera réajusté. <strong>Cette action est irréversible.</strong>
          </p>
          <DialogFooter className="gap-2">
            <Button variant="outline" onClick={() => setCancelConfirm(null)}>Fermer</Button>
            <Button
              variant="destructive"
              disabled={cancelMutation.isPending}
              onClick={() => {
                if (cancelConfirm) cancelMutation.mutate(cancelConfirm.id, { onSuccess: () => setCancelConfirm(null) })
              }}
            >
              {cancelMutation.isPending ? <Loader2 className="h-4 w-4 animate-spin mr-2" /> : <Ban className="h-4 w-4 mr-2" />}
              Annuler la vente
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  )
}