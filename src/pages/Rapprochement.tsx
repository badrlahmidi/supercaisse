import { useState } from "react"
import { useQuery } from "@tanstack/react-query"
import { Card, CardContent } from "@/ui/Card"
import { Button } from "@/ui/Button"
import { Input } from "@/ui/Input"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/ui/Table"
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from "@/ui/Dialog"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/ui/Select"
import { Label } from "@/ui/Label"
import { Badge } from "@/ui/Badge"
import { Search, CheckCircle, AlertTriangle, XCircle, Download, FileCheck } from "lucide-react"
import PageHeader from "@/components/PageHeader"
import EmptyState from "@/components/EmptyState"
import { formatCurrency, formatDate, exportCSV } from "@/lib/utils"
import { invoke } from "@/lib/tauri"
import type { AchatResume } from "@/types/generated/AchatResume"

type Achat = AchatResume

type StatutRapprochement = "all" | "conforme" | "ecart" | "non_rapproche"

export default function Rapprochement() {
  const [search, setSearch] = useState("")
  const [statutFilter, setStatutFilter] = useState<StatutRapprochement>("all")
  const [selectedAchat, setSelectedAchat] = useState<Achat | null>(null)
  const [factureRef, setFactureRef] = useState("")
  const [factureMontant, setFactureMontant] = useState("")

  const { data: achats, isLoading } = useQuery({
    queryKey: ["achats"],
    queryFn: () => invoke<Achat[]>("get_achats"),
    staleTime: 30000,
  })

  const getStatut = (achat: Achat) => {
    if (achat.statut_paiement === "paye" && achat.statut_livraison === "recu") return "conforme"
    if (achat.statut_livraison === "recu" && achat.statut_paiement !== "paye") return "ecart"
    return "non_rapproche"
  }

  const filteredAchats = achats?.filter((a) => {
    const matchSearch = a.reference?.toLowerCase().includes(search.toLowerCase()) ||
      a.fournisseur_nom?.toLowerCase().includes(search.toLowerCase())
    const statut = getStatut(a)
    const matchStatut = statutFilter === "all" || statut === statutFilter
    return matchSearch && matchStatut
  })

  const stats = {
    total: achats?.length || 0,
    conforme: achats?.filter((a) => getStatut(a) === "conforme").length || 0,
    ecart: achats?.filter((a) => getStatut(a) === "ecart").length || 0,
    nonRapproche: achats?.filter((a) => getStatut(a) === "non_rapproche").length || 0,
  }

  if (isLoading) {
    return (
      <div className="space-y-6">
        <PageHeader title="Rapprochement" description="Factures fournisseur ↔ bons de réception" />
        <Card>
          <CardContent className="pt-6">
            <div className="space-y-3">
              {[1, 2, 3, 4, 5].map((i) => (
                <div key={i} className="h-12 bg-muted rounded animate-pulse" />
              ))}
            </div>
          </CardContent>
        </Card>
      </div>
    )
  }

  return (
    <div className="space-y-6">
      <PageHeader title="Rapprochement" description="Factures fournisseur ↔ bons de réception">
        <Button variant="outline" onClick={() => {
          if (!filteredAchats) return
          const headers = ["Référence", "Fournisseur", "Date", "Montant", "Livraison", "Paiement", "Statut"]
          const rows = filteredAchats.map((a) => [
            a.reference || "", a.fournisseur_nom || "", formatDate(a.date),
            formatCurrency(a.montant_total), a.statut_livraison, a.statut_paiement, getStatut(a),
          ])
          exportCSV(headers, rows, "rapprochement.csv")
        }}>
          <Download className="h-4 w-4 mr-2" />
          Exporter
        </Button>
      </PageHeader>

      <div className="grid gap-4 md:grid-cols-4">
        <Card>
          <CardContent className="pt-6">
            <div className="text-2xl font-bold">{stats.total}</div>
            <p className="text-sm text-muted-foreground">Total achats</p>
          </CardContent>
        </Card>
        <Card className="border-green-200 dark:border-green-800">
          <CardContent className="pt-6">
            <div className="flex items-center gap-2">
              <CheckCircle className="h-5 w-5 text-green-500" />
              <div className="text-2xl font-bold text-green-600">{stats.conforme}</div>
            </div>
            <p className="text-sm text-muted-foreground">Conformes</p>
          </CardContent>
        </Card>
        <Card className="border-amber-200 dark:border-amber-800">
          <CardContent className="pt-6">
            <div className="flex items-center gap-2">
              <AlertTriangle className="h-5 w-5 text-amber-500" />
              <div className="text-2xl font-bold text-amber-600">{stats.ecart}</div>
            </div>
            <p className="text-sm text-muted-foreground">Écarts</p>
          </CardContent>
        </Card>
        <Card className="border-red-200 dark:border-red-800">
          <CardContent className="pt-6">
            <div className="flex items-center gap-2">
              <XCircle className="h-5 w-5 text-red-500" />
              <div className="text-2xl font-bold text-red-600">{stats.nonRapproche}</div>
            </div>
            <p className="text-sm text-muted-foreground">Non rapprochés</p>
          </CardContent>
        </Card>
      </div>

      <Card>
        <CardContent className="pt-6">
          <div className="flex gap-4 mb-4">
            <div className="relative flex-1 max-w-md">
              <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
              <Input
                placeholder="Rechercher par référence ou fournisseur..."
                value={search}
                onChange={(e) => setSearch(e.target.value)}
                className="pl-10"
              />
            </div>
            <Select value={statutFilter} onValueChange={(v) => setStatutFilter(v as StatutRapprochement)}>
              <SelectTrigger className="w-[200px]">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="all">Tous les statuts</SelectItem>
                <SelectItem value="conforme">Conformes</SelectItem>
                <SelectItem value="ecart">Écarts</SelectItem>
                <SelectItem value="non_rapproche">Non rapprochés</SelectItem>
              </SelectContent>
            </Select>
          </div>

          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Référence</TableHead>
                <TableHead>Fournisseur</TableHead>
                <TableHead>Date</TableHead>
                <TableHead className="text-right">Montant</TableHead>
                <TableHead>Livraison</TableHead>
                <TableHead>Paiement</TableHead>
                <TableHead>Statut</TableHead>
                <TableHead>Actions</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {filteredAchats?.map((achat) => {
                const statut = getStatut(achat)
                return (
                  <TableRow key={achat.id}>
                    <TableCell className="font-mono text-sm">{achat.reference || "—"}</TableCell>
                    <TableCell>{achat.fournisseur_nom || "—"}</TableCell>
                    <TableCell>{formatDate(achat.date)}</TableCell>
                    <TableCell className="text-right font-medium">{formatCurrency(achat.montant_total)}</TableCell>
                    <TableCell>
                      <Badge variant={achat.statut_livraison === "recu" ? "default" : "secondary"}>
                        {achat.statut_livraison === "recu" ? "Reçu" : achat.statut_livraison === "partiel" ? "Partiel" : "En attente"}
                      </Badge>
                    </TableCell>
                    <TableCell>
                      <Badge variant={achat.statut_paiement === "paye" ? "default" : achat.statut_paiement === "partiel" ? "secondary" : "destructive"}>
                        {achat.statut_paiement === "paye" ? "Payé" : achat.statut_paiement === "partiel" ? "Partiel" : "Impayé"}
                      </Badge>
                    </TableCell>
                    <TableCell>
                      {statut === "conforme" && <Badge className="bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200">Conforme</Badge>}
                      {statut === "ecart" && <Badge className="bg-amber-100 text-amber-800 dark:bg-amber-900 dark:text-amber-200">Écart</Badge>}
                      {statut === "non_rapproche" && <Badge className="bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200">Non rapproché</Badge>}
                    </TableCell>
                    <TableCell>
                      <Button variant="ghost" size="sm" onClick={() => { setSelectedAchat(achat); setFactureRef(achat.reference || ""); setFactureMontant(String(achat.montant_total)) }}>
                        <FileCheck className="h-4 w-4 mr-1" />
                        Rapprocher
                      </Button>
                    </TableCell>
                  </TableRow>
                )
              })}
              {!filteredAchats?.length && (
                <TableRow>
                  <TableCell colSpan={8}>
                    <EmptyState
                      icon={<Search className="h-12 w-12" />}
                      title="Aucun achat trouvé"
                      description="Les achats apparaîtront ici pour rapprochement"
                    />
                  </TableCell>
                </TableRow>
              )}
            </TableBody>
          </Table>
        </CardContent>
      </Card>

      <Dialog open={!!selectedAchat} onOpenChange={() => setSelectedAchat(null)}>
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle>Rapprochement — {selectedAchat?.reference || "Sans ref."}</DialogTitle>
          </DialogHeader>
          <div className="space-y-4">
            <div className="rounded-lg bg-muted p-4 space-y-2">
              <div className="flex justify-between text-sm">
                <span className="text-muted-foreground">Fournisseur</span>
                <span className="font-medium">{selectedAchat?.fournisseur_nom || "—"}</span>
              </div>
              <div className="flex justify-between text-sm">
                <span className="text-muted-foreground">Date réception</span>
                <span className="font-medium">{selectedAchat ? formatDate(selectedAchat.date) : ""}</span>
              </div>
              <div className="flex justify-between text-sm">
                <span className="text-muted-foreground">Montant bon de réception</span>
                <span className="font-medium">{selectedAchat ? formatCurrency(selectedAchat.montant_total) : ""}</span>
              </div>
            </div>
            <div className="space-y-2">
              <Label>Référence facture fournisseur</Label>
              <Input value={factureRef} onChange={(e) => setFactureRef(e.target.value)} placeholder="N° facture fournisseur" />
            </div>
            <div className="space-y-2">
              <Label>Montant facture fournisseur</Label>
              <Input type="number" step="0.01" value={factureMontant} onChange={(e) => setFactureMontant(e.target.value)} />
            </div>
            {selectedAchat && factureMontant && (
              <div className="rounded-lg p-3 border">
                {Number(factureMontant) === selectedAchat.montant_total ? (
                  <div className="flex items-center gap-2 text-green-600">
                    <CheckCircle className="h-5 w-5" />
                    <span className="font-medium">Montants conformes</span>
                  </div>
                ) : (
                  <div className="space-y-1">
                    <div className="flex items-center gap-2 text-amber-600">
                      <AlertTriangle className="h-5 w-5" />
                      <span className="font-medium">Écart détecté</span>
                    </div>
                    <p className="text-sm text-muted-foreground">
                      Différence : {formatCurrency(Math.abs(Number(factureMontant) - selectedAchat.montant_total))}
                      ({((Math.abs(Number(factureMontant) - selectedAchat.montant_total) / selectedAchat.montant_total) * 100).toFixed(1)}%)
                    </p>
                  </div>
                )}
              </div>
            )}
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setSelectedAchat(null)}>Fermer</Button>
            <Button onClick={() => setSelectedAchat(null)}>Valider le rapprochement</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  )
}
