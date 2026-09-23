import { useState } from "react"
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query"
import { invoke } from "@/lib/tauri"
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
import { toast } from "sonner"
import { Search, SearchX, AlertTriangle, Package, Loader2, ArrowUpDown, History, CalendarClock, Plus, Trash2 } from "lucide-react"
import { formatCurrency, formatDate } from "@/lib/utils"
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
  suivi_lot?: boolean
}

interface ArticleLot {
  id: number
  numero_lot: string | null
  date_peremption: string | null
  quantite: number
  date_reception: string
}

const lotSchema = z.object({
  numero_lot: z.string().optional(),
  date_peremption: z.string().optional(),
  quantite: z.number().min(0.01, "Quantité requise"),
})
type LotForm = z.infer<typeof lotSchema>

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
  const [lotsArticle, setLotsArticle] = useState<Article | null>(null)
  const [showLots, setShowLots] = useState(false)

  const { data: articles, isLoading } = useProductsList()
  const queryClient = useQueryClient()

  const adjustMutation = useAdjustStock()

  const { data: lots } = useQuery({
    queryKey: ["article_lots", lotsArticle?.id],
    queryFn: () => invoke<ArticleLot[]>("get_article_lots", { article_id: lotsArticle?.id }),
    enabled: showLots && !!lotsArticle,
  })

  const lotForm = useForm<LotForm>({
    resolver: zodResolver(lotSchema),
    defaultValues: { numero_lot: "", date_peremption: "", quantite: 0 },
  })

  const invalidateLots = () => {
    queryClient.invalidateQueries({ queryKey: ["article_lots", lotsArticle?.id] })
    queryClient.invalidateQueries({ queryKey: ["articles"] })
  }

  const addLotMutation = useMutation({
    mutationFn: (data: LotForm) => invoke("add_article_lot", {
      article_id: lotsArticle?.id,
      numero_lot: data.numero_lot || null,
      date_peremption: data.date_peremption || null,
      quantite: data.quantite,
    }),
    onSuccess: () => {
      toast.success("Lot ajouté")
      lotForm.reset({ numero_lot: "", date_peremption: "", quantite: 0 })
      invalidateLots()
    },
    onError: (e) => toast.error("Erreur", { description: String(e) }),
  })

  const discardLotMutation = useMutation({
    mutationFn: ({ lot_id, quantite }: { lot_id: number; quantite: number }) =>
      invoke("discard_article_lot", { lot_id, quantite, motif: "peremption" }),
    onSuccess: () => {
      toast.success("Lot retiré du stock")
      invalidateLots()
    },
    onError: (e) => toast.error("Erreur", { description: String(e) }),
  })

  const openLots = (article: Article) => {
    setLotsArticle(article)
    lotForm.reset({ numero_lot: "", date_peremption: "", quantite: 0 })
    setShowLots(true)
  }

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
        <Button variant="outline" onClick={() => navigate("/stock/peremptions")}>
          <CalendarClock className="h-4 w-4 mr-2" />
          Péremptions
        </Button>
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
                          <Button variant="ghost" size="icon" onClick={() => openAdjust(article)} title="Ajuster le stock">
                            <ArrowUpDown className="h-4 w-4" />
                          </Button>
                          {article.suivi_lot && (
                            <Button variant="ghost" size="icon" onClick={() => openLots(article)} title="Lots / péremption">
                              <CalendarClock className="h-4 w-4" />
                            </Button>
                          )}
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

      <Dialog open={showLots} onOpenChange={setShowLots}>
        <DialogContent className="max-w-lg max-h-[90vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle>Lots — {lotsArticle?.designation}</DialogTitle>
          </DialogHeader>
          <div className="space-y-4">
            <form
              onSubmit={lotForm.handleSubmit((data) => addLotMutation.mutate(data))}
              className="grid grid-cols-3 gap-2 items-end p-3 bg-muted/30 rounded-lg"
            >
              <div className="space-y-1">
                <Label htmlFor="numero_lot" className="text-xs">N° lot</Label>
                <Input {...lotForm.register("numero_lot")} id="numero_lot" placeholder="Optionnel" className="h-9" />
              </div>
              <div className="space-y-1">
                <Label htmlFor="date_peremption" className="text-xs">Péremption</Label>
                <Input type="date" {...lotForm.register("date_peremption")} id="date_peremption" className="h-9" />
              </div>
              <div className="space-y-1">
                <Label htmlFor="quantite_lot" className="text-xs">Quantité</Label>
                <Input
                  type="number"
                  step="1"
                  min="0"
                  {...lotForm.register("quantite", { valueAsNumber: true })}
                  id="quantite_lot"
                  className="h-9"
                />
              </div>
              <Button type="submit" size="sm" className="col-span-3" disabled={addLotMutation.isPending}>
                {addLotMutation.isPending ? <Loader2 className="h-4 w-4 animate-spin mr-2" /> : <Plus className="h-4 w-4 mr-2" />}
                Ajouter ce lot (entrée de stock)
              </Button>
            </form>

            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>N° lot</TableHead>
                  <TableHead>Péremption</TableHead>
                  <TableHead className="text-right">Qté restante</TableHead>
                  <TableHead className="w-[50px]" />
                </TableRow>
              </TableHeader>
              <TableBody>
                {lots?.filter((l) => l.quantite > 0).map((lot) => {
                  const isSoon = lot.date_peremption && new Date(lot.date_peremption).getTime() - Date.now() < 1000 * 60 * 60 * 24 * 30
                  const isExpired = lot.date_peremption && new Date(lot.date_peremption).getTime() < Date.now()
                  return (
                    <TableRow key={lot.id} className={isExpired ? "bg-destructive/5" : isSoon ? "bg-warning/5" : ""}>
                      <TableCell className="font-mono text-sm">{lot.numero_lot || "—"}</TableCell>
                      <TableCell className={isExpired ? "text-destructive font-medium" : isSoon ? "text-warning font-medium" : ""}>
                        {lot.date_peremption ? formatDate(lot.date_peremption) : "—"}
                      </TableCell>
                      <TableCell className="text-right">{lot.quantite}</TableCell>
                      <TableCell>
                        <Button
                          variant="ghost"
                          size="icon"
                          title="Retirer du stock (péremption/casse)"
                          onClick={() => discardLotMutation.mutate({ lot_id: lot.id, quantite: lot.quantite })}
                          disabled={discardLotMutation.isPending}
                        >
                          <Trash2 className="h-4 w-4 text-destructive" />
                        </Button>
                      </TableCell>
                    </TableRow>
                  )
                })}
                {!lots?.filter((l) => l.quantite > 0).length && (
                  <TableRow>
                    <TableCell colSpan={4} className="text-center py-6 text-muted-foreground text-sm">
                      Aucun lot en stock pour cet article
                    </TableCell>
                  </TableRow>
                )}
              </TableBody>
            </Table>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setShowLots(false)}>Fermer</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  )
}