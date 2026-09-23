import { useState } from "react"
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query"
import { invoke } from "@/lib/tauri"
import { useProductsList, useCreateProduct, useUpdateProduct, useDeleteProduct } from "@/hooks/useProducts"
import { useCategoriesList } from "@/hooks/useCategories"
import { useFournisseursList } from "@/hooks/useFournisseurs"
import { Card, CardContent } from "@/ui/Card"
import { Button } from "@/ui/Button"
import { Input } from "@/ui/Input"
import { Badge } from "@/ui/Badge"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/ui/Table"
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from "@/ui/Dialog"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/ui/Select"
import { Label } from "@/ui/Label"
import { Textarea } from "@/ui/Textarea"
import { useForm } from "react-hook-form"
import { zodResolver } from "@hookform/resolvers/zod"
import { z } from "zod"
import { toast } from "sonner"
import { Plus, Edit, Trash2, Search, Loader2, Download, Upload, SearchX, ImagePlus, X, Tags, Boxes, Printer } from "lucide-react"
import { jsPDF } from "jspdf"
import PageHeader from "@/components/PageHeader"
import EmptyState from "@/components/EmptyState"
import { formatCurrency, exportCSV } from "@/lib/utils"
import { cn } from "@/lib/utils"
import { useDebounce } from "@/hooks/useDebounce"
import type { Article, ArticleVariante, ArticleComposant, Category, Fournisseur } from "@/types"

const varianteSchema = z.object({
  taille: z.string().optional(),
  couleur: z.string().optional(),
  code_barre: z.string().optional(),
  stock_initial: z.number().min(0).default(0),
})
type VarianteForm = z.infer<typeof varianteSchema>

const composantSchema = z.object({
  composant_id: z.number().min(1, "Article requis"),
  quantite: z.number().min(0.01, "Quantité requise"),
})
type ComposantForm = z.infer<typeof composantSchema>

const articleSchema = z.object({
  code_barre: z.string().optional().nullable(),
  designation: z.string().min(1, "Désignation requise"),
  description: z.string().optional().nullable(),
  image_url: z.string().optional().nullable(),
  prix_achat: z.number().min(0, "Prix d'achat requis").default(0),
  prix_vente: z.number().min(0.01, "Prix de vente requis").default(0),
  tva: z.number().min(0).max(100).default(20),
  stock: z.number().min(0).default(0),
  stock_alerte: z.number().min(0).optional().nullable(),
  categorie_id: z.number().optional().nullable(),
  fournisseur_id: z.number().optional().nullable(),
  suivi_lot: z.boolean().default(false),
  prix_grossiste: z.number().min(0).optional().nullable(),
  est_kit: z.boolean().default(false),
})

type ArticleForm = z.infer<typeof articleSchema>

export default function Articles() {
  const [search, setSearch] = useState("")
  const [debouncedSearch] = useDebounce(search, 300)
  const [editingArticle, setEditingArticle] = useState<Article | null>(null)
  const [showForm, setShowForm] = useState(false)
  const [importing, setImporting] = useState(false)
  const [imagePreview, setImagePreview] = useState<string | null>(null)
  const [variantesArticle, setVariantesArticle] = useState<Article | null>(null)
  const [showVariantes, setShowVariantes] = useState(false)
  const [composantsArticle, setComposantsArticle] = useState<Article | null>(null)
  const [showComposants, setShowComposants] = useState(false)
  const [selectedForLabels, setSelectedForLabels] = useState<Set<number>>(new Set())

  const { data: articles, isLoading, refetch } = useProductsList(debouncedSearch)
  const { data: categories } = useCategoriesList()
  const { data: fournisseurs } = useFournisseursList()
  const queryClient = useQueryClient()

  const createMutation = useCreateProduct()
  const updateMutation = useUpdateProduct()
  const deleteMutation = useDeleteProduct()

  const { data: variantes } = useQuery({
    queryKey: ["article_variantes", variantesArticle?.id],
    queryFn: () => invoke<ArticleVariante[]>("get_article_variantes", { article_id: variantesArticle?.id }),
    enabled: showVariantes && !!variantesArticle,
  })

  const varianteForm = useForm<VarianteForm>({
    resolver: zodResolver(varianteSchema),
    defaultValues: { taille: "", couleur: "", code_barre: "", stock_initial: 0 },
  })

  const invalidateVariantes = () => queryClient.invalidateQueries({ queryKey: ["article_variantes", variantesArticle?.id] })

  const addVarianteMutation = useMutation({
    mutationFn: (data: VarianteForm) => invoke("add_article_variante", {
      article_id: variantesArticle?.id,
      taille: data.taille || null,
      couleur: data.couleur || null,
      code_barre: data.code_barre || null,
      stock_initial: data.stock_initial,
    }),
    onSuccess: () => {
      toast.success("Variante ajoutée")
      varianteForm.reset({ taille: "", couleur: "", code_barre: "", stock_initial: 0 })
      invalidateVariantes()
    },
    onError: (e) => toast.error("Erreur", { description: String(e) }),
  })

  const deleteVarianteMutation = useMutation({
    mutationFn: (id: number) => invoke("delete_article_variante", { id }),
    onSuccess: () => { toast.success("Variante supprimée"); invalidateVariantes() },
    onError: (e) => toast.error("Erreur", { description: String(e) }),
  })

  const adjustVarianteStockMutation = useMutation({
    mutationFn: ({ id, quantite }: { id: number; quantite: number }) => invoke("adjust_article_variante_stock", { id, quantite }),
    onSuccess: () => invalidateVariantes(),
    onError: (e) => toast.error("Erreur", { description: String(e) }),
  })

  const openVariantes = (article: Article) => {
    setVariantesArticle(article)
    varianteForm.reset({ taille: "", couleur: "", code_barre: "", stock_initial: 0 })
    setShowVariantes(true)
  }

  const { data: composants } = useQuery({
    queryKey: ["article_composants", composantsArticle?.id],
    queryFn: () => invoke<ArticleComposant[]>("get_article_composants", { article_id: composantsArticle?.id }),
    enabled: showComposants && !!composantsArticle,
  })

  const composantForm = useForm<ComposantForm>({
    resolver: zodResolver(composantSchema),
    defaultValues: { composant_id: 0, quantite: 1 },
  })

  const invalidateComposants = () => queryClient.invalidateQueries({ queryKey: ["article_composants", composantsArticle?.id] })

  const addComposantMutation = useMutation({
    mutationFn: (data: ComposantForm) => invoke("add_article_composant", {
      article_id: composantsArticle?.id,
      composant_id: data.composant_id,
      quantite: data.quantite,
    }),
    onSuccess: () => {
      toast.success("Composant ajouté")
      composantForm.reset({ composant_id: 0, quantite: 1 })
      invalidateComposants()
    },
    onError: (e) => toast.error("Erreur", { description: String(e) }),
  })

  const deleteComposantMutation = useMutation({
    mutationFn: (id: number) => invoke("delete_article_composant", { id }),
    onSuccess: () => { toast.success("Composant retiré"); invalidateComposants() },
    onError: (e) => toast.error("Erreur", { description: String(e) }),
  })

  const openComposants = (article: Article) => {
    setComposantsArticle(article)
    composantForm.reset({ composant_id: 0, quantite: 1 })
    setShowComposants(true)
  }

  const form = useForm<ArticleForm>({
    resolver: zodResolver(articleSchema),
    defaultValues: {
      code_barre: "",
      designation: "",
      description: "",
      prix_achat: 0,
      prix_vente: 0,
      tva: 20,
      stock: 0,
      stock_alerte: null,
      categorie_id: null,
      fournisseur_id: null,
      suivi_lot: false,
      prix_grossiste: null,
      est_kit: false,
    },
  })

  const handleSubmit = (data: ArticleForm) => {
    if (editingArticle) {
      updateMutation.mutate({ id: editingArticle.id, ...data })
    } else {
      createMutation.mutate(data)
    }
    setImagePreview(null)
  }

  const openEdit = (article: Article) => {
    setEditingArticle(article)
    form.reset({
      code_barre: article.code_barre,
      designation: article.designation,
      description: "",
      image_url: article.image_url ?? null,
      prix_achat: article.prix_achat,
      prix_vente: article.prix_vente,
      tva: article.tva,
      stock: article.stock,
      stock_alerte: article.stock_alerte,
      categorie_id: article.categorie_id,
      fournisseur_id: null,
      suivi_lot: article.suivi_lot || false,
      prix_grossiste: article.prix_grossiste ?? null,
      est_kit: article.est_kit || false,
    })
    setImagePreview(article.image_url ?? null)
    setShowForm(true)
  }

  const openCreate = () => {
    setEditingArticle(null)
    form.reset({
      code_barre: "",
      designation: "",
      description: "",
      image_url: null,
      prix_achat: 0,
      prix_vente: 0,
      tva: 20,
      stock: 0,
      stock_alerte: null,
      categorie_id: null,
      fournisseur_id: null,
      suivi_lot: false,
      prix_grossiste: null,
      est_kit: false,
    })
    setImagePreview(null)
    setShowForm(true)
  }

  const printBarcodeLabels = (items: Article[]) => {
    if (!items.length) return
    const doc = new jsPDF({ unit: "mm", format: "a4" })
    const pageW = doc.internal.pageSize.getWidth()
    const cols = 3
    const rows = 10
    const labelW = (pageW - 20) / cols
    const labelH = 27
    const marginX = 10
    const marginY = 10

    items.forEach((article, idx) => {
      if (idx > 0 && idx % (cols * rows) === 0) doc.addPage()
      const posInPage = idx % (cols * rows)
      const col = posInPage % cols
      const row = Math.floor(posInPage / cols)
      const x = marginX + col * labelW
      const y = marginY + row * labelH

      doc.setDrawColor(200)
      doc.rect(x, y, labelW, labelH)

      doc.setFontSize(8)
      const designation = article.designation.length > 28
        ? article.designation.slice(0, 28) + "..."
        : article.designation
      doc.text(designation, x + labelW / 2, y + 6, { align: "center" })

      const barcode = article.code_barre || `INT-${String(article.id).padStart(6, "0")}`
      doc.setFontSize(14)
      doc.text(barcode, x + labelW / 2, y + 14, { align: "center" })

      doc.setFontSize(9)
      doc.text(formatCurrency(article.prix_vente), x + labelW / 2, y + 22, { align: "center" })
    })

    doc.save("etiquettes_codes_barres.pdf")
    toast.success(`${items.length} étiquette(s) générée(s)`)
    setSelectedForLabels(new Set())
  }

  if (isLoading) {
    return (
      <div className="space-y-6">
        <PageHeader title="Articles" description="Gestion du catalogue produits" />
        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
          {[1, 2, 3, 4].map((i) => (
            <Card key={i} className="animate-pulse">
              <CardContent className="pt-6">
                <div className="h-4 w-3/4 bg-muted rounded" />
                <div className="h-8 w-1/2 bg-muted rounded mt-2" />
              </CardContent>
            </Card>
          ))}
        </div>
      </div>
    )
  }

  return (
    <div className="space-y-6">
      <PageHeader title="Articles" description="Gestion du catalogue produits">
        <div className="flex gap-2">
          <input
            type="file"
            accept=".csv"
            className="hidden"
            id="csv-import"
            onChange={async (e) => {
              const file = e.target.files?.[0]
              if (!file) return
              setImporting(true)
              try {
                const text = await file.text()
                const { invoke } = await import("@/lib/tauri")
                const report = await invoke<string>("import_articles_csv", { csvContent: text })
                toast.success("Import terminé", { description: report })
                refetch()
              } catch (err) {
                toast.error("Échec de l'import", { description: String(err) })
              } finally {
                setImporting(false)
                e.target.value = ""
              }
            }}
          />
          {selectedForLabels.size > 0 && (
            <Button variant="outline" onClick={() => printBarcodeLabels(articles?.filter((a) => selectedForLabels.has(a.id)) || [])}>
              <Printer className="h-4 w-4 mr-2" />
              Étiquettes ({selectedForLabels.size})
            </Button>
          )}
          <Button variant="outline" disabled={importing} onClick={() => document.getElementById("csv-import")?.click()}>
            {importing ? <Loader2 className="h-4 w-4 animate-spin mr-2" /> : <Upload className="h-4 w-4 mr-2" />}
            Importer CSV
          </Button>
          <Button variant="outline" onClick={() => {
            if (!articles) return
            const headers = ["Désignation", "Code-barres", "Prix achat", "Prix vente", "TVA", "Stock", "Stock alerte"]
            const rows = articles.map((a) => [
              a.designation,
              a.code_barre || "",
              String(a.prix_achat).replace(".", ","),
              String(a.prix_vente).replace(".", ","),
              String(a.tva).replace(".", ","),
              String(a.stock).replace(".", ","),
              a.stock_alerte != null ? String(a.stock_alerte).replace(".", ",") : "",
            ])
            exportCSV(headers, rows, "articles.csv")
          }}>
            <Download className="h-4 w-4 mr-2" />
            Exporter
          </Button>
          <Button onClick={openCreate}>
            <Plus className="h-4 w-4 mr-2" />
            Nouvel article
          </Button>
        </div>
      </PageHeader>

      <Card>
        <CardContent className="pt-6">
          <div className="flex gap-4 mb-4">
            <div className="relative flex-1 max-w-md">
              <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
              <Input
                placeholder="Rechercher par nom ou code-barres..."
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
                  <TableHead className="w-10">
                    <input
                      type="checkbox"
                      className="h-4 w-4"
                      checked={articles?.length ? selectedForLabels.size === articles.length : false}
                      onChange={(e) => {
                        if (e.target.checked && articles) {
                          setSelectedForLabels(new Set(articles.map((a) => a.id)))
                        } else {
                          setSelectedForLabels(new Set())
                        }
                      }}
                    />
                  </TableHead>
                  <TableHead>Code-barres</TableHead>
                  <TableHead>Désignation</TableHead>
                  <TableHead className="text-right">Prix achat</TableHead>
                  <TableHead className="text-right">Prix vente</TableHead>
                  <TableHead>TVA</TableHead>
                  <TableHead className="text-right">Stock</TableHead>
                  <TableHead>Catégorie</TableHead>
                  <TableHead>Fournisseur</TableHead>
                  <TableHead className="w-[100px]">Actions</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {articles?.map((article) => (
                  <TableRow key={article.id}>
                    <TableCell>
                      <input
                        type="checkbox"
                        className="h-4 w-4"
                        checked={selectedForLabels.has(article.id)}
                        onChange={(e) => {
                          const next = new Set(selectedForLabels)
                          if (e.target.checked) next.add(article.id)
                          else next.delete(article.id)
                          setSelectedForLabels(next)
                        }}
                      />
                    </TableCell>
                    <TableCell className="font-mono text-sm">{article.code_barre || "—"}</TableCell>
                    <TableCell className="font-medium">{article.designation}</TableCell>
                    <TableCell className="text-right">{formatCurrency(article.prix_achat)}</TableCell>
                    <TableCell className="text-right font-medium">{formatCurrency(article.prix_vente)}</TableCell>
                    <TableCell>
                      <Badge variant="outline" className="text-xs">
                        {article.tva}%
                      </Badge>
                    </TableCell>
                    <TableCell className="text-right">
                      <span
                        className={cn(
                          article.stock_alerte && article.stock <= article.stock_alerte
                            ? "text-destructive font-medium"
                            : ""
                        )}
                      >
                        {article.stock}
                      </span>
                      {article.stock_alerte && article.stock <= article.stock_alerte && (
                        <Badge variant="destructive" className="ml-1 text-[10px]">
                          Alerte
                        </Badge>
                      )}
                    </TableCell>
                    <TableCell>{article.categorie_nom || "—"}</TableCell>
                    <TableCell>{article.fournisseur_nom || "—"}</TableCell>
                    <TableCell>
                      <div className="flex items-center gap-1">
                        <Button variant="ghost" size="icon" onClick={() => openEdit(article)} title="Modifier">
                          <Edit className="h-4 w-4" />
                        </Button>
                        <Button variant="ghost" size="icon" onClick={() => openVariantes(article)} title="Déclinaisons taille/couleur">
                          <Tags className="h-4 w-4" />
                        </Button>
                        {article.est_kit && (
                          <Button variant="ghost" size="icon" onClick={() => openComposants(article)} title="Composants du kit">
                            <Boxes className="h-4 w-4" />
                          </Button>
                        )}
                        <Button variant="ghost" size="icon" onClick={() => deleteMutation.mutate(article.id)} title="Supprimer">
                          <Trash2 className="h-4 w-4 text-destructive" />
                        </Button>
                      </div>
                    </TableCell>
                  </TableRow>
                ))}
                {!articles?.length && (
                  <TableRow>
                    <TableCell colSpan={10}>
                      <EmptyState
                        icon={<SearchX className="h-12 w-12" />}
                        title="Aucun article trouvé"
                        description="Aucun article ne correspond à votre recherche"
                      />
                    </TableCell>
                  </TableRow>
                )}
              </TableBody>
            </Table>
          </div>
        </CardContent>
      </Card>

      <Dialog open={showForm} onOpenChange={setShowForm}>
        <DialogContent className="max-w-2xl max-h-[90vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle>{editingArticle ? "Modifier l'article" : "Nouvel article"}</DialogTitle>
          </DialogHeader>
          <form onSubmit={form.handleSubmit(handleSubmit)} className="space-y-4">
            <div className="grid gap-4 md:grid-cols-2">
              <div className="space-y-2">
                <Label htmlFor="code_barre">Code-barres</Label>
                <Input {...form.register("code_barre")} id="code_barre" placeholder="Optionnel" />
              </div>
              <div className="space-y-2">
                <Label htmlFor="designation">Désignation *</Label>
                <Input {...form.register("designation")} id="designation" placeholder="Nom du produit" />
                {form.formState.errors.designation && (
                  <p className="text-sm text-destructive">{form.formState.errors.designation.message}</p>
                )}
              </div>
              <div className="space-y-2 md:col-span-2">
                <Label htmlFor="description">Description</Label>
                <Textarea {...form.register("description")} id="description" rows={2} placeholder="Optionnel" />
              </div>
                <div className="space-y-2 md:col-span-2">
                  <Label>Image produit</Label>
                  <div className="flex items-start gap-3">
                    {imagePreview ? (
                      <div className="relative shrink-0">
                        <img src={imagePreview} alt="Preview" className="h-20 w-20 rounded-lg object-cover border border-border" />
                        <button
                          type="button"
                          onClick={() => { setImagePreview(null); form.setValue("image_url", null) }}
                          className="absolute -top-1.5 -right-1.5 h-5 w-5 rounded-full bg-destructive text-destructive-foreground flex items-center justify-center"
                        >
                          <X className="h-3 w-3" />
                        </button>
                      </div>
                    ) : (
                      <div className="h-20 w-20 rounded-lg border-2 border-dashed border-border flex items-center justify-center shrink-0 bg-muted/30">
                        <ImagePlus className="h-6 w-6 text-muted-foreground/50" />
                      </div>
                    )}
                    <div className="flex-1">
                      <input
                        id="image-upload"
                        type="file"
                        accept="image/*"
                        className="hidden"
                        onChange={(e) => {
                          const file = e.target.files?.[0]
                          if (!file) return
                          if (file.size > 500 * 1024) {
                            toast.error("Image trop lourde", { description: "Max 500 Ko" })
                            return
                          }
                          const reader = new FileReader()
                          reader.onload = (ev) => {
                            const b64 = ev.target?.result as string
                            setImagePreview(b64)
                            form.setValue("image_url", b64)
                          }
                          reader.readAsDataURL(file)
                        }}
                      />
                      <Button
                        type="button"
                        variant="outline"
                        size="sm"
                        onClick={() => document.getElementById("image-upload")?.click()}
                      >
                        <Upload className="h-3.5 w-3.5 mr-1.5" />
                        Choisir une image
                      </Button>
                      <p className="text-xs text-muted-foreground mt-1.5">JPG, PNG, WEBP — max 500 Ko. Stocké en base64.</p>
                    </div>
                  </div>
                </div>
              <div className="space-y-2">
                <Label htmlFor="prix_achat">Prix d'achat</Label>
                <Input
                  type="number"
                  step="0.01"
                  min="0"
                  {...form.register("prix_achat", { valueAsNumber: true })}
                  id="prix_achat"
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="prix_vente">Prix de vente *</Label>
                <Input
                  type="number"
                  step="0.01"
                  min="0.01"
                  {...form.register("prix_vente", { valueAsNumber: true })}
                  id="prix_vente"
                />
                {form.formState.errors.prix_vente && (
                  <p className="text-sm text-destructive">{form.formState.errors.prix_vente.message}</p>
                )}
              </div>
              <div className="space-y-2">
                <Label htmlFor="prix_grossiste">Prix grossiste</Label>
                <Input
                  type="number"
                  step="0.01"
                  min="0"
                  {...form.register("prix_grossiste", { valueAsNumber: true })}
                  id="prix_grossiste"
                  placeholder="Optionnel — client professionnel"
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="tva">TVA (%)</Label>
                <Input
                  type="number"
                  step="0.1"
                  min="0"
                  max="100"
                  {...form.register("tva", { valueAsNumber: true })}
                  id="tva"
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="stock">Stock initial</Label>
                <Input
                  type="number"
                  step="1"
                  min="0"
                  {...form.register("stock", { valueAsNumber: true })}
                  id="stock"
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="stock_alerte">Seuil d'alerte</Label>
                <Input
                  type="number"
                  step="1"
                  min="0"
                  {...form.register("stock_alerte", { valueAsNumber: true })}
                  id="stock_alerte"
                  placeholder="Optionnel"
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="categorie_id">Catégorie</Label>
                <Select
                  value={form.watch("categorie_id") ? String(form.watch("categorie_id")) : "none"}
                  onValueChange={(v) => form.setValue("categorie_id", v !== "none" ? parseInt(v) : null)}
                >
                  <SelectTrigger id="categorie_id">
                    <SelectValue placeholder="Sélectionner une catégorie" />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="none">Aucune</SelectItem>
                    {categories?.map((c) => (
                      <SelectItem key={c.id} value={String(c.id)}>
                        {c.nom}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
              <div className="space-y-2 md:col-span-2 flex items-center gap-2 pt-2">
                <input
                  type="checkbox"
                  id="suivi_lot"
                  className="h-4 w-4"
                  checked={form.watch("suivi_lot")}
                  onChange={(e) => form.setValue("suivi_lot", e.target.checked)}
                />
                <Label htmlFor="suivi_lot" className="cursor-pointer">
                  Suivi de lot / date de péremption (DLC-DLUO) — supermarché, pharmacie
                </Label>
              </div>
              <div className="space-y-2 md:col-span-2 flex items-center gap-2">
                <input
                  type="checkbox"
                  id="est_kit"
                  className="h-4 w-4"
                  checked={form.watch("est_kit")}
                  onChange={(e) => form.setValue("est_kit", e.target.checked)}
                />
                <Label htmlFor="est_kit" className="cursor-pointer">
                  Produit composé / kit (ex. coffret) — le stock des composants est géré dans "Composants" ci-dessous une fois l'article enregistré
                </Label>
              </div>
              <div className="space-y-2">
                <Label htmlFor="fournisseur_id">Fournisseur</Label>
                <Select
                  value={form.watch("fournisseur_id") ? String(form.watch("fournisseur_id")) : "none"}
                  onValueChange={(v) => form.setValue("fournisseur_id", v !== "none" ? parseInt(v) : null)}
                >
                  <SelectTrigger id="fournisseur_id">
                    <SelectValue placeholder="Sélectionner un fournisseur" />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="none">Aucun</SelectItem>
                    {fournisseurs?.map((f) => (
                      <SelectItem key={f.id} value={String(f.id)}>
                        {f.nom}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
            </div>
            <DialogFooter>
              <Button type="button" variant="outline" onClick={() => setShowForm(false)}>
                Annuler
              </Button>
              <Button type="submit" disabled={createMutation.isPending || updateMutation.isPending}>
                {createMutation.isPending || updateMutation.isPending ? (
                  <>
                    <Loader2 className="h-4 w-4 animate-spin mr-2" />
                    Enregistrement...
                  </>
                ) : (
                  "Enregistrer"
                )}
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>

      <Dialog open={showVariantes} onOpenChange={setShowVariantes}>
        <DialogContent className="max-w-2xl max-h-[90vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle>Déclinaisons — {variantesArticle?.designation}</DialogTitle>
          </DialogHeader>
          <div className="space-y-4">
            <p className="text-sm text-muted-foreground">
              Chaque déclinaison (taille/couleur) a son propre code-barres et son propre stock.
              Non encore vendable directement depuis le POS — voir <code>ROADMAP_STATUS.md</code>.
            </p>
            <form
              onSubmit={varianteForm.handleSubmit((data) => addVarianteMutation.mutate(data))}
              className="grid grid-cols-4 gap-2 items-end p-3 bg-muted/30 rounded-lg"
            >
              <div className="space-y-1">
                <Label htmlFor="v_taille" className="text-xs">Taille</Label>
                <Input {...varianteForm.register("taille")} id="v_taille" placeholder="M, 42..." className="h-9" />
              </div>
              <div className="space-y-1">
                <Label htmlFor="v_couleur" className="text-xs">Couleur</Label>
                <Input {...varianteForm.register("couleur")} id="v_couleur" placeholder="Bleu..." className="h-9" />
              </div>
              <div className="space-y-1">
                <Label htmlFor="v_code_barre" className="text-xs">Code-barres</Label>
                <Input {...varianteForm.register("code_barre")} id="v_code_barre" placeholder="Optionnel" className="h-9" />
              </div>
              <div className="space-y-1">
                <Label htmlFor="v_stock" className="text-xs">Stock initial</Label>
                <Input
                  type="number"
                  step="1"
                  min="0"
                  {...varianteForm.register("stock_initial", { valueAsNumber: true })}
                  id="v_stock"
                  className="h-9"
                />
              </div>
              <Button type="submit" size="sm" className="col-span-4" disabled={addVarianteMutation.isPending}>
                {addVarianteMutation.isPending ? <Loader2 className="h-4 w-4 animate-spin mr-2" /> : <Plus className="h-4 w-4 mr-2" />}
                Ajouter cette déclinaison
              </Button>
            </form>

            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Taille</TableHead>
                  <TableHead>Couleur</TableHead>
                  <TableHead>Code-barres</TableHead>
                  <TableHead className="text-right">Stock</TableHead>
                  <TableHead className="w-[90px]" />
                </TableRow>
              </TableHeader>
              <TableBody>
                {variantes?.map((v) => (
                  <TableRow key={v.id}>
                    <TableCell>{v.taille || "—"}</TableCell>
                    <TableCell>{v.couleur || "—"}</TableCell>
                    <TableCell className="font-mono text-sm">{v.code_barre || "—"}</TableCell>
                    <TableCell className="text-right">
                      <div className="flex items-center justify-end gap-1">
                        <Button
                          variant="ghost" size="icon" className="h-6 w-6"
                          onClick={() => adjustVarianteStockMutation.mutate({ id: v.id, quantite: -1 })}
                          disabled={v.stock_dedie <= 0}
                        >-</Button>
                        <span className="w-8 text-center">{v.stock_dedie}</span>
                        <Button
                          variant="ghost" size="icon" className="h-6 w-6"
                          onClick={() => adjustVarianteStockMutation.mutate({ id: v.id, quantite: 1 })}
                        >+</Button>
                      </div>
                    </TableCell>
                    <TableCell>
                      <Button variant="ghost" size="icon" onClick={() => deleteVarianteMutation.mutate(v.id)}>
                        <Trash2 className="h-4 w-4 text-destructive" />
                      </Button>
                    </TableCell>
                  </TableRow>
                ))}
                {!variantes?.length && (
                  <TableRow>
                    <TableCell colSpan={5} className="text-center py-6 text-muted-foreground text-sm">
                      Aucune déclinaison pour cet article
                    </TableCell>
                  </TableRow>
                )}
              </TableBody>
            </Table>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setShowVariantes(false)}>Fermer</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog open={showComposants} onOpenChange={setShowComposants}>
        <DialogContent className="max-w-lg max-h-[90vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle>Composants du kit — {composantsArticle?.designation}</DialogTitle>
          </DialogHeader>
          <div className="space-y-4">
            <p className="text-sm text-muted-foreground">
              Vendre ce kit décrémente automatiquement le stock de chaque composant ci-dessous, au prorata de la quantité indiquée. Le kit lui-même n'a pas de stock propre.
            </p>
            <form
              onSubmit={composantForm.handleSubmit((data) => addComposantMutation.mutate(data))}
              className="grid grid-cols-3 gap-2 items-end p-3 bg-muted/30 rounded-lg"
            >
              <div className="space-y-1 col-span-2">
                <Label htmlFor="composant_id" className="text-xs">Article composant</Label>
                <Select
                  value={composantForm.watch("composant_id") ? String(composantForm.watch("composant_id")) : ""}
                  onValueChange={(v) => composantForm.setValue("composant_id", parseInt(v))}
                >
                  <SelectTrigger id="composant_id" className="h-9">
                    <SelectValue placeholder="Sélectionner un article" />
                  </SelectTrigger>
                  <SelectContent>
                    {articles?.filter((a) => a.id !== composantsArticle?.id).map((a) => (
                      <SelectItem key={a.id} value={String(a.id)}>{a.designation}</SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
              <div className="space-y-1">
                <Label htmlFor="quantite_composant" className="text-xs">Qté / kit</Label>
                <Input
                  type="number"
                  step="0.01"
                  min="0.01"
                  {...composantForm.register("quantite", { valueAsNumber: true })}
                  id="quantite_composant"
                  className="h-9"
                />
              </div>
              <Button type="submit" size="sm" className="col-span-3" disabled={addComposantMutation.isPending}>
                {addComposantMutation.isPending ? <Loader2 className="h-4 w-4 animate-spin mr-2" /> : <Plus className="h-4 w-4 mr-2" />}
                Ajouter ce composant
              </Button>
            </form>

            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Composant</TableHead>
                  <TableHead className="text-right">Stock dispo</TableHead>
                  <TableHead className="text-right">Qté / kit</TableHead>
                  <TableHead className="w-[50px]" />
                </TableRow>
              </TableHeader>
              <TableBody>
                {composants?.map((c) => (
                  <TableRow key={c.id}>
                    <TableCell className="font-medium">{c.designation}</TableCell>
                    <TableCell className="text-right">{c.stock}</TableCell>
                    <TableCell className="text-right">{c.quantite}</TableCell>
                    <TableCell>
                      <Button variant="ghost" size="icon" onClick={() => deleteComposantMutation.mutate(c.id)}>
                        <Trash2 className="h-4 w-4 text-destructive" />
                      </Button>
                    </TableCell>
                  </TableRow>
                ))}
                {!composants?.length && (
                  <TableRow>
                    <TableCell colSpan={4} className="text-center py-6 text-muted-foreground text-sm">
                      Aucun composant défini pour ce kit
                    </TableCell>
                  </TableRow>
                )}
              </TableBody>
            </Table>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setShowComposants(false)}>Fermer</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  )
}