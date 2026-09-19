import { useState } from "react"
import { useProductsList } from "@/hooks/useProducts"
import { useAdjustStock } from "@/hooks/useStock"
import { Card, CardContent } from "@/ui/Card"
import { Button } from "@/ui/Button"
import { Input } from "@/ui/Input"
import { Badge } from "@/ui/Badge"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/ui/Table"
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from "@/ui/Dialog"
import { Label } from "@/ui/Label"
import { useForm } from "react-hook-form"
import { zodResolver } from "@hookform/resolvers/zod"
import { z } from "zod"
import { Search, SearchX, AlertTriangle, Package, Loader2, ArrowUpDown, History } from "lucide-react"
import { formatCurrency } from "@/lib/utils"
import { useNavigate } from "react-router-dom"
import PageHeader from "@/components/PageHeader"
import EmptyState from "@/components/EmptyState"

interface Article {
  id: number
  code_barre: string | null
  designation: string
  prix_vente: number
  stock: number
  stock_alerte: number | null
  categorie_nom?: string
}

const stockAdjustSchema = z.object({
  article_id: z.number().min(1),
  quantite: z.number().min(-9999).max(9999).refine(v => v !== 0, "Quantité non nulle requise"),
})

type StockAdjustForm = z.infer<typeof stockAdjustSchema>

export default function Stock() {
  const [search, setSearch] = useState("")
  const [filter, setFilter] = useState<"all" | "low" | "out" | "ok">("all")
  const [adjustingArticle, setAdjustingArticle] = useState<Article | null>(null)
  const [showAdjust, setShowAdjust] = useState(false)

  const { data: articles, isLoading } = useProductsList()

  const adjustMutation = useAdjustStock()

  const form = useForm<StockAdjustForm>({
    resolver: zodResolver(stockAdjustSchema),
    defaultValues: { article_id: 0, quantite: 0 },
  })

  const handleSubmit = (data: StockAdjustForm) => {
    adjustMutation.mutate(data, { onSuccess: () => setShowAdjust(false) })
  }

  const openAdjust = (article: Article) => {
    setAdjustingArticle(article)
    form.reset({ article_id: article.id, quantite: 0 })
    setShowAdjust(true)
  }

  const filtered = articles?.filter((a) => {
    if (search) {
      const q = search.toLowerCase()
      if (!a.designation.toLowerCase().includes(q) && !a.code_barre?.includes(search)) return false
    }
    if (filter === "out") return a.stock <= 0
    if (filter === "low") return a.stock_alerte && a.stock > 0 && a.stock <= a.stock_alerte
    if (filter === "ok") return !(a.stock <= 0 || (a.stock_alerte && a.stock <= a.stock_alerte))
    return true
  }) || []

  const stats = {
    total: filtered.length,
    out: filtered.filter(a => a.stock <= 0).length,
    low: filtered.filter(a => a.stock_alerte && a.stock > 0 && a.stock <= a.stock_alerte).length,
    ok: filtered.filter(a => !(a.stock <= 0 || (a.stock_alerte && a.stock <= a.stock_alerte))).length,
    value: filtered.reduce((s, a) => s + a.stock * a.prix_vente, 0),
  }

  const navigate = useNavigate()

  return (
    <div className="space-y-6">
      <PageHeader title="Gestion du stock" description="Suivi des niveaux de stock">
        <Button variant="outline" onClick={() => navigate("/stock/mouvements")}>
          <History className="h-4 w-4 mr-2" />
          Mouvements
        </Button>
      </PageHeader>

      <div className="grid gap-4 md:grid-cols-5">
        <Card>
          <CardContent className="pt-6">
            <p className="text-sm text-muted-foreground">Total articles</p>
            <p className="text-2xl font-bold">{stats.total}</p>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-6">
            <p className="text-sm text-muted-foreground">Rupture</p>
            <p className="text-2xl font-bold text-destructive">{stats.out}</p>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-6">
            <p className="text-sm text-muted-foreground">Stock bas</p>
            <p className="text-2xl font-bold text-warning">{stats.low}</p>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-6">
            <p className="text-sm text-muted-foreground">OK</p>
            <p className="text-2xl font-bold text-success">{stats.ok}</p>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-6">
            <p className="text-sm text-muted-foreground">Valeur stock</p>
            <p className="text-2xl font-bold">{formatCurrency(stats.value)}</p>
          </CardContent>
        </Card>
      </div>

      <Card>
        <CardContent className="pt-6">
          <div className="flex flex-col sm:flex-row gap-4 mb-4">
            <div className="relative flex-1 max-w-md">
              <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
              <Input
                placeholder="Rechercher par nom ou code-barres..."
                value={search}
                onChange={(e) => setSearch(e.target.value)}
                className="pl-10"
              />
            </div>
            <div className="flex gap-2">
              {(["all", "ok", "low", "out"] as const).map((f) => (
                <Button
                  key={f}
                  variant={filter === f ? "default" : "outline"}
                  size="sm"
                  onClick={() => setFilter(f)}
                >
                  {f === "all" && "Tous"}
                  {f === "ok" && <><Package className="h-3.5 w-3.5 mr-1 text-success" /> OK</>}
                  {f === "low" && <><AlertTriangle className="h-3.5 w-3.5 mr-1 text-warning" /> Bas</>}
                  {f === "out" && <><Package className="h-3.5 w-3.5 mr-1 text-destructive" /> Rupture</>}
                </Button>
              ))}
            </div>
          </div>

          <div className="overflow-x-auto">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Code-barres</TableHead>
                  <TableHead>Désignation</TableHead>
                  <TableHead>Catégorie</TableHead>
                  <TableHead className="text-right">Prix vente</TableHead>
                  <TableHead className="text-right">Stock</TableHead>
                  <TableHead className="text-right">Alerte</TableHead>
                  <TableHead className="text-right">Valeur</TableHead>
                  <TableHead className="w-[120px]">Actions</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {isLoading ? (
                  <TableRow>
                    <TableCell colSpan={8} className="text-center py-8">
                      <Loader2 className="h-8 w-8 animate-spin mx-auto" />
                    </TableCell>
                  </TableRow>
                ) : filtered.map((article) => {
                  const isOut = article.stock <= 0
                  const isLow = article.stock_alerte && article.stock > 0 && article.stock <= article.stock_alerte
                  return (
                    <TableRow key={article.id} className={isOut ? "bg-destructive/5" : isLow ? "bg-warning/5" : ""}>
                      <TableCell className="font-mono text-sm">{article.code_barre || "—"}</TableCell>
                      <TableCell className="font-medium">{article.designation}</TableCell>
                      <TableCell>{article.categorie_nom || "—"}</TableCell>
                      <TableCell className="text-right">{formatCurrency(article.prix_vente)}</TableCell>
                      <TableCell className="text-right font-medium">
                        <span className={isOut ? "text-destructive" : isLow ? "text-warning" : ""}>
                          {article.stock}
                        </span>
                      </TableCell>
                      <TableCell className="text-right">
                        {article.stock_alerte ? (
                          <Badge variant={isLow ? "destructive" : "outline"} className="text-xs">
                            {article.stock_alerte}
                          </Badge>
                        ) : (
                          <span className="text-muted-foreground">—</span>
                        )}
                      </TableCell>
                      <TableCell className="text-right font-medium">
                        {formatCurrency(article.stock * article.prix_vente)}
                      </TableCell>
                      <TableCell>
                        <div className="flex items-center gap-1">
                          <Button variant="ghost" size="icon" onClick={() => openAdjust(article)}>
                            <ArrowUpDown className="h-4 w-4" />
                          </Button>
                        </div>
                      </TableCell>
                    </TableRow>
                  )
                })}
                {!isLoading && !filtered.length && (
                  <TableRow>
                    <TableCell colSpan={8}>
                      <EmptyState icon={<SearchX className="h-12 w-12" />} title="Aucun article trouvé" description="Aucun article ne correspond aux critères" />
                    </TableCell>
                  </TableRow>
                )}
              </TableBody>
            </Table>
          </div>
        </CardContent>
      </Card>

      <Dialog open={showAdjust} onOpenChange={setShowAdjust}>
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle>Ajuster le stock</DialogTitle>
          </DialogHeader>
          {adjustingArticle && (
            <form onSubmit={form.handleSubmit(handleSubmit)} className="space-y-4">
              <div className="space-y-2">
                <Label>Article</Label>
                <p className="font-medium">{adjustingArticle.designation}</p>
                <p className="text-sm text-muted-foreground">Stock actuel: {adjustingArticle.stock}</p>
              </div>
              <div className="space-y-2">
                <Label htmlFor="quantite">Quantité à ajouter/retirer</Label>
                <Input
                  type="number"
                  step="1"
                  {...form.register("quantite", { valueAsNumber: true })}
                  id="quantite"
                  placeholder="Ex: +5 ou -3"
                />
                <p className="text-xs text-muted-foreground">
                  Utilisez un nombre positif pour ajouter, négatif pour retirer
                </p>
                {form.formState.errors.quantite && (
                  <p className="text-sm text-destructive">{form.formState.errors.quantite.message}</p>
                )}
              </div>
              <DialogFooter>
                <Button type="button" variant="outline" onClick={() => setShowAdjust(false)}>
                  Annuler
                </Button>
                <Button type="submit" disabled={adjustMutation.isPending}>
                  {adjustMutation.isPending ? "Mise à jour..." : "Appliquer"}
                </Button>
              </DialogFooter>
            </form>
          )}
        </DialogContent>
      </Dialog>
    </div>
  )
}