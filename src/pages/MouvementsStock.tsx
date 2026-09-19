import { useState } from "react"
import { useMouvementsStock } from "@/hooks/useMouvementsStock"
import { useProductsList } from "@/hooks/useProducts"
import { Card, CardContent } from "@/ui/Card"
import { Input } from "@/ui/Input"
import { Badge } from "@/ui/Badge"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/ui/Table"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/ui/Select"
import { Search, Loader2, SearchX, Package, ArrowDownRight, ArrowUpRight } from "lucide-react"
import PageHeader from "@/components/PageHeader"
import EmptyState from "@/components/EmptyState"
import { formatCurrency, formatDateTime } from "@/lib/utils"
import { useDebounce } from "@/hooks/useDebounce"
import { cn } from "@/lib/utils"

const TYPE_LABELS: Record<string, string> = {
  entree: "Entrée",
  sortie: "Sortie",
  vente: "Vente",
  achat: "Achat",
  ajustement: "Ajustement",
  transfert: "Transfert",
}

const TYPE_VARIANTS: Record<string, "success" | "destructive" | "outline" | "warning"> = {
  entree: "success",
  sortie: "destructive",
  vente: "destructive",
  achat: "success",
  ajustement: "warning",
  transfert: "outline",
}

export default function MouvementsStock() {
  const [search, setSearch] = useState("")
  const [debouncedSearch] = useDebounce(search, 300)
  const [articleFilter, setArticleFilter] = useState<string>("all")
  const [typeFilter, setTypeFilter] = useState<string>("all")

  const { data: articles } = useProductsList(debouncedSearch)
  const { data: mouvements, isLoading } = useMouvementsStock()

  const filtered = mouvements?.filter((m) => {
    if (typeFilter !== "all" && m.mtype !== typeFilter) return false
    if (articleFilter !== "all" && m.article_id !== parseInt(articleFilter)) return false
    if (debouncedSearch) {
      const q = debouncedSearch.toLowerCase()
      return m.designation.toLowerCase().includes(q)
    }
    return true
  })

  const stats = {
    entree: mouvements?.filter((m) => m.mtype === "entree" || m.mtype === "achat").reduce((s, m) => s + m.quantite, 0) || 0,
    sortie: mouvements?.filter((m) => m.mtype === "sortie" || m.mtype === "vente").reduce((s, m) => s + m.quantite, 0) || 0,
    total: mouvements?.length || 0,
  }

  return (
    <div className="space-y-6">
      <PageHeader title="Mouvements de stock" description="Historique des entrées et sorties de stock" />

      <div className="grid grid-cols-3 gap-4">
        <Card>
          <CardContent className="pt-4 flex items-center gap-3">
            <div className="p-2 rounded-lg bg-success/10">
              <ArrowDownRight className="h-5 w-5 text-success" />
            </div>
            <div>
              <p className="text-sm text-muted-foreground">Entrées</p>
              <p className="text-xl font-bold text-success">{formatCurrency(stats.entree)}</p>
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-4 flex items-center gap-3">
            <div className="p-2 rounded-lg bg-destructive/10">
              <ArrowUpRight className="h-5 w-5 text-destructive" />
            </div>
            <div>
              <p className="text-sm text-muted-foreground">Sorties</p>
              <p className="text-xl font-bold text-destructive">{formatCurrency(stats.sortie)}</p>
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-4 flex items-center gap-3">
            <div className="p-2 rounded-lg bg-primary/10">
              <Package className="h-5 w-5 text-primary" />
            </div>
            <div>
              <p className="text-sm text-muted-foreground">Opérations</p>
              <p className="text-xl font-bold">{stats.total}</p>
            </div>
          </CardContent>
        </Card>
      </div>

      <Card>
        <CardContent className="pt-6">
          <div className="flex gap-4 mb-4 flex-wrap">
            <div className="relative flex-1 max-w-md">
              <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
              <Input
                placeholder="Rechercher par article..."
                value={search}
                onChange={(e) => setSearch(e.target.value)}
                className="pl-10"
              />
            </div>
            <Select value={typeFilter} onValueChange={setTypeFilter}>
              <SelectTrigger className="w-36">
                <SelectValue placeholder="Type" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="all">Tous</SelectItem>
                <SelectItem value="entree">Entrée</SelectItem>
                <SelectItem value="sortie">Sortie</SelectItem>
                <SelectItem value="vente">Vente</SelectItem>
                <SelectItem value="achat">Achat</SelectItem>
                <SelectItem value="ajustement">Ajustement</SelectItem>
              </SelectContent>
            </Select>
            <Select value={articleFilter} onValueChange={setArticleFilter}>
              <SelectTrigger className="w-48">
                <SelectValue placeholder="Article" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="all">Tous les articles</SelectItem>
                {articles?.map((a) => (
                  <SelectItem key={a.id} value={String(a.id)}>{a.designation}</SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          <div className="overflow-x-auto">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Date</TableHead>
                  <TableHead>Article</TableHead>
                  <TableHead className="text-right">Quantité</TableHead>
                  <TableHead>Type</TableHead>
                  <TableHead>Référence</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {isLoading ? (
                  <TableRow>
                    <TableCell colSpan={5} className="text-center py-8">
                      <Loader2 className="h-6 w-6 animate-spin mx-auto text-muted-foreground" />
                    </TableCell>
                  </TableRow>
                ) : filtered?.length === 0 ? (
                  <TableRow>
                    <TableCell colSpan={5}>
                      <EmptyState
                        icon={<SearchX className="h-12 w-12" />}
                        title="Aucun mouvement trouvé"
                        description="Aucun mouvement de stock ne correspond à votre recherche"
                      />
                    </TableCell>
                  </TableRow>
                ) : (
                  filtered?.map((m) => (
                    <TableRow key={m.id}>
                      <TableCell className="text-sm">{formatDateTime(m.date)}</TableCell>
                      <TableCell className="font-medium">{m.designation}</TableCell>
                      <TableCell className={cn("text-right font-mono font-medium", m.mtype === "sortie" || m.mtype === "vente" ? "text-destructive" : "text-success")}>
                        {m.mtype === "sortie" || m.mtype === "vente" ? "-" : "+"}{m.quantite}
                      </TableCell>
                      <TableCell>
                        <Badge variant={TYPE_VARIANTS[m.mtype] || "outline"}>
                          {TYPE_LABELS[m.mtype] || m.mtype}
                        </Badge>
                      </TableCell>
                      <TableCell className="text-sm text-muted-foreground">
                        {m.reference_type ? `${m.reference_type} #${m.reference_id}` : "—"}
                      </TableCell>
                    </TableRow>
                  ))
                )}
              </TableBody>
            </Table>
          </div>
        </CardContent>
      </Card>
    </div>
  )
}
