import { useState } from "react"
import { useCheques, useUpdateChequeStatus } from "@/hooks/useCheques"
import { Card, CardContent } from "@/ui/Card"
import { Button } from "@/ui/Button"
import { Input } from "@/ui/Input"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/ui/Table"
import { Badge } from "@/ui/Badge"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/ui/Select"
import { Search, Loader2, Download, SearchX, CheckCircle, XCircle } from "lucide-react"
import PageHeader from "@/components/PageHeader"
import EmptyState from "@/components/EmptyState"
import { formatCurrency, formatDate, exportCSV } from "@/lib/utils"
export default function Cheques() {
  const [search, setSearch] = useState("")
  const [statusFilter, setStatusFilter] = useState<string>("all")
  const [typeFilter, setTypeFilter] = useState<string>("all")

  const { data: cheques, isLoading } = useCheques()
  const updateStatus = useUpdateChequeStatus()

  const filteredCheques = cheques?.filter((c) => {
    const matchesSearch = c.numero.toLowerCase().includes(search.toLowerCase()) ||
                          c.banque.toLowerCase().includes(search.toLowerCase()) ||
                          c.client_nom?.toLowerCase().includes(search.toLowerCase()) ||
                          c.fournisseur_nom?.toLowerCase().includes(search.toLowerCase())
    const matchesStatus = statusFilter === "all" || c.statut === statusFilter
    const matchesType = typeFilter === "all" || c.ctype === typeFilter
    return matchesSearch && matchesStatus && matchesType
  }) || []

  const handleExport = () => {
    const headers = ["N° Chèque", "Banque", "Tireur", "Type", "Bénéficiaire/Émetteur", "Montant", "Émission", "Échéance", "Statut"]
    const rows = filteredCheques.map(c => [
      c.numero, c.banque, c.tireur || "", c.ctype === "client" ? "Encaissement" : "Décaissement",
      c.ctype === "client" ? (c.client_nom || "") : (c.fournisseur_nom || ""),
      formatCurrency(c.montant), formatDate(c.date_emission), formatDate(c.date_echeance), c.statut
    ])
    exportCSV(headers, rows, "suivi_cheques.csv")
  }

  if (isLoading) {
    return (
      <div className="space-y-6">
        <PageHeader title="Suivi des Chèques" description="Gestion du portefeuille de chèques" />
        <Card className="animate-pulse h-64" />
      </div>
    )
  }

  return (
    <div className="space-y-6">
      <PageHeader title="Suivi des Chèques" description="Gestion du portefeuille de chèques">
        <Button variant="outline" onClick={handleExport}>
          <Download className="h-4 w-4 mr-2" />
          Exporter CSV
        </Button>
      </PageHeader>

      <Card>
        <CardContent className="pt-6">
          <div className="flex gap-4 mb-4 flex-wrap">
            <div className="relative flex-1 min-w-[250px]">
              <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
              <Input
                placeholder="N°, Banque, Nom..."
                value={search}
                onChange={(e) => setSearch(e.target.value)}
                className="pl-10"
              />
            </div>
            <Select value={typeFilter} onValueChange={setTypeFilter}>
              <SelectTrigger className="w-[180px]">
                <SelectValue placeholder="Type" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="all">Tous les types</SelectItem>
                <SelectItem value="client">Encaissements (Clients)</SelectItem>
                <SelectItem value="fournisseur">Décaissements (Frns)</SelectItem>
              </SelectContent>
            </Select>
            <Select value={statusFilter} onValueChange={setStatusFilter}>
              <SelectTrigger className="w-[180px]">
                <SelectValue placeholder="Statut" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="all">Tous les statuts</SelectItem>
                <SelectItem value="en_attente">En attente</SelectItem>
                <SelectItem value="encaisse">Encaissé</SelectItem>
                <SelectItem value="impaye">Impayé</SelectItem>
              </SelectContent>
            </Select>
          </div>

          <div className="overflow-x-auto">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>N° Chèque</TableHead>
                  <TableHead>Banque / Tireur</TableHead>
                  <TableHead>Sens</TableHead>
                  <TableHead>Partie</TableHead>
                  <TableHead>Échéance</TableHead>
                  <TableHead className="text-right">Montant</TableHead>
                  <TableHead>Statut</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {filteredCheques.map((c) => (
                  <TableRow key={c.id}>
                    <TableCell className="font-medium font-mono">{c.numero}</TableCell>
                    <TableCell>
                      <div className="font-medium">{c.banque}</div>
                      <div className="text-xs text-muted-foreground">{c.tireur}</div>
                    </TableCell>
                    <TableCell>
                      <Badge variant={c.ctype === "client" ? "default" : "secondary"}>
                        {c.ctype === "client" ? "Reçu" : "Émis"}
                      </Badge>
                    </TableCell>
                    <TableCell>{c.ctype === "client" ? c.client_nom : c.fournisseur_nom}</TableCell>
                    <TableCell className="text-sm">
                      <div className={new Date(c.date_echeance) < new Date() && c.statut === "en_attente" ? "text-destructive font-semibold" : ""}>
                        {formatDate(c.date_echeance)}
                      </div>
                    </TableCell>
                    <TableCell className="text-right font-medium">{formatCurrency(c.montant)}</TableCell>
                    <TableCell>
                      <Select value={c.statut} onValueChange={(val) => updateStatus.mutate({ chequeId: c.id, statut: val })}>
                        <SelectTrigger className="h-8 w-[130px]">
                          <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                          <SelectItem value="en_attente">En attente</SelectItem>
                          <SelectItem value="encaisse">Encaissé</SelectItem>
                          <SelectItem value="impaye">Impayé</SelectItem>
                        </SelectContent>
                      </Select>
                    </TableCell>
                  </TableRow>
                ))}
                {filteredCheques.length === 0 && (
                  <TableRow>
                    <TableCell colSpan={7}>
                      <EmptyState
                        icon={<SearchX className="h-12 w-12" />}
                        title="Aucun chèque"
                        description="Aucun chèque ne correspond à vos critères."
                      />
                    </TableCell>
                  </TableRow>
                )}
              </TableBody>
            </Table>
          </div>
        </CardContent>
      </Card>
    </div>
  )
}
