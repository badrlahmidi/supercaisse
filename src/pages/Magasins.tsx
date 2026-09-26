import { useState } from "react"
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query"
import { invoke } from "@/lib/tauri"
import { Card, CardContent, CardHeader, CardTitle } from "@/ui/Card"
import { Button } from "@/ui/Button"
import { Input } from "@/ui/Input"
import { Label } from "@/ui/Label"
import { Badge } from "@/ui/Badge"
import { Table, TableHeader, TableBody, TableRow, TableHead, TableCell } from "@/ui/Table"
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from "@/ui/Dialog"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/ui/Select"
import { toast } from "sonner"
import { useForm } from "react-hook-form"
import { zodResolver } from "@hookform/resolvers/zod"
import { z } from "zod"
import PageHeader from "@/components/PageHeader"
import { Store, Plus, Pencil, Trash2, AlertTriangle, ArrowRightLeft, Loader2, Package } from "lucide-react"
import type { Magasin } from "@/types/generated/Magasin"
import type { TransfertResume } from "@/types/generated/TransfertResume"
import type { StockMagasin } from "@/types/generated/StockMagasin"

type Transfert = TransfertResume

type StockLine = StockMagasin

const magasinSchema = z.object({
  nom: z.string().min(1, "Nom requis"),
  adresse: z.string().optional(),
})

type MagasinForm = z.infer<typeof magasinSchema>

const transfertSchema = z.object({
  source_id: z.string().min(1, "Source requise"),
  dest_id: z.string().min(1, "Destination requise"),
})

export default function Magasins() {
  const queryClient = useQueryClient()
  const [showForm, setShowForm] = useState(false)
  const [editingId, setEditingId] = useState<number | null>(null)
  const [deleteTarget, setDeleteTarget] = useState<Magasin | null>(null)
  const [showTransfert, setShowTransfert] = useState(false)
  const [transfertLines, setTransfertLines] = useState<{ article_id: number; designation: string; quantite: number }[]>([])
  const [stockMagasinId, setStockMagasinId] = useState<number | null>(null)

  const { data: magasins = [], isLoading } = useQuery({
    queryKey: ["magasins"],
    queryFn: () => invoke<Magasin[]>("get_magasins"),
  })

  const { data: transferts = [] } = useQuery({
    queryKey: ["transferts"],
    queryFn: () => invoke<Transfert[]>("get_transferts"),
  })

  const { data: stockLines = [] } = useQuery({
    queryKey: ["stock_par_magasin", stockMagasinId],
    queryFn: () => invoke<StockLine[]>("get_stock_par_magasin", { magasinId: stockMagasinId }),
    enabled: !!stockMagasinId,
  })

  const form = useForm<MagasinForm>({
    resolver: zodResolver(magasinSchema),
    defaultValues: { nom: "", adresse: "" },
  })

  const transfertForm = useForm<{ source_id: string; dest_id: string }>({
    resolver: zodResolver(transfertSchema),
    defaultValues: { source_id: "", dest_id: "" },
  })

  const addMutation = useMutation({
    mutationFn: (data: MagasinForm) => invoke<number>("add_magasin", { nom: data.nom, adresse: data.adresse || null }),
    onSuccess: () => {
      toast.success("Boutique ajoutée")
      queryClient.invalidateQueries({ queryKey: ["magasins"] })
      closeForm()
    },
    onError: (e) => toast.error("Erreur", { description: String(e) }),
  })

  const updateMutation = useMutation({
    mutationFn: (data: MagasinForm & { id: number }) => invoke("update_magasin", { id: data.id, nom: data.nom, adresse: data.adresse || null }),
    onSuccess: () => {
      toast.success("Boutique modifiée")
      queryClient.invalidateQueries({ queryKey: ["magasins"] })
      closeForm()
    },
    onError: (e) => toast.error("Erreur", { description: String(e) }),
  })

  const deleteMutation = useMutation({
    mutationFn: (id: number) => invoke("delete_magasin", { id }),
    onSuccess: () => {
      toast.success("Boutique supprimée")
      queryClient.invalidateQueries({ queryKey: ["magasins"] })
      setDeleteTarget(null)
    },
    onError: (e) => toast.error("Erreur", { description: String(e) }),
  })

  const createTransfertMutation = useMutation({
    mutationFn: ({ sourceId, destId, articles }: { sourceId: number; destId: number; articles: { article_id: number; quantite: number }[] }) =>
      invoke<number>("create_transfert", { sourceId, destId, articles }),
    onSuccess: () => {
      toast.success("Transfert créé (en attente de validation)")
      queryClient.invalidateQueries({ queryKey: ["transferts"] })
      setShowTransfert(false)
      setTransfertLines([])
      transfertForm.reset()
    },
    onError: (e) => toast.error("Erreur", { description: String(e) }),
  })

  const validateTransfertMutation = useMutation({
    mutationFn: (transfertId: number) => invoke("validate_transfert", { transfertId }),
    onSuccess: () => {
      toast.success("Transfert validé, stock mis à jour")
      queryClient.invalidateQueries({ queryKey: ["transferts"] })
      queryClient.invalidateQueries({ queryKey: ["stock_par_magasin"] })
    },
    onError: (e) => toast.error("Erreur", { description: String(e) }),
  })

  const closeForm = () => {
    setShowForm(false)
    setEditingId(null)
    form.reset({ nom: "", adresse: "" })
  }

  const openEdit = (m: Magasin) => {
    setEditingId(m.id)
    form.reset({ nom: m.nom, adresse: m.adresse || "" })
    setShowForm(true)
  }

  const onSubmit = (data: MagasinForm) => {
    if (editingId) {
      updateMutation.mutate({ ...data, id: editingId })
    } else {
      addMutation.mutate(data)
    }
  }

  const sourceId = transfertForm.watch("source_id")

  return (
    <div className="space-y-6">
      <PageHeader
        title="Boutiques"
        description="Gérez vos points de vente et les transferts de stock inter-boutiques"
      >
        <Button onClick={() => { closeForm(); setShowForm(true) }} className="gap-2">
          <Plus className="h-4 w-4" />
          Nouvelle boutique
        </Button>
      </PageHeader>

      <div className="grid gap-6 lg:grid-cols-2">
        {/* Magasins List */}
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Store className="h-5 w-5" />
              Mes boutiques ({magasins.length})
            </CardTitle>
          </CardHeader>
          <CardContent>
            {isLoading ? (
              <div className="flex justify-center py-8">
                <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
              </div>
            ) : magasins.length === 0 ? (
              <p className="text-center text-muted-foreground py-8">Aucune boutique configurée</p>
            ) : (
              <div className="space-y-3">
                {magasins.map((m, i) => (
                  <div
                    key={m.id}
                    className="flex items-center justify-between rounded-lg border p-4 hover:bg-muted/50 transition-colors"
                  >
                    <div className="flex items-center gap-3">
                      <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-primary/10 text-primary shrink-0">
                        <Store className="h-5 w-5" />
                      </div>
                      <div>
                        <p className="font-medium">{m.nom}</p>
                        {m.adresse && <p className="text-sm text-muted-foreground">{m.adresse}</p>}
                      </div>
                      {i === 0 && <Badge variant="secondary">Principal</Badge>}
                    </div>
                    <div className="flex items-center gap-1">
                      <Button variant="ghost" size="icon" onClick={() => setStockMagasinId(m.id)} title="Voir le stock">
                        <Package className="h-4 w-4" />
                      </Button>
                      <Button variant="ghost" size="icon" onClick={() => openEdit(m)}>
                        <Pencil className="h-4 w-4" />
                      </Button>
                      {i > 0 && (
                        <Button variant="ghost" size="icon" onClick={() => setDeleteTarget(m)}>
                          <Trash2 className="h-4 w-4 text-destructive" />
                        </Button>
                      )}
                    </div>
                  </div>
                ))}
              </div>
            )}
          </CardContent>
        </Card>

        {/* Transferts */}
        <Card>
          <CardHeader className="flex flex-row items-center justify-between">
            <CardTitle className="flex items-center gap-2">
              <ArrowRightLeft className="h-5 w-5" />
              Transferts de stock
            </CardTitle>
            {magasins.length >= 2 && (
              <Button variant="outline" size="sm" onClick={() => setShowTransfert(true)} className="gap-2">
                <Plus className="h-4 w-4" />
                Nouveau transfert
              </Button>
            )}
          </CardHeader>
          <CardContent>
            {magasins.length < 2 ? (
              <p className="text-center text-muted-foreground py-8">
                Ajoutez une 2<sup>e</sup> boutique pour transférer du stock
              </p>
            ) : transferts.length === 0 ? (
              <p className="text-center text-muted-foreground py-8">Aucun transfert enregistré</p>
            ) : (
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Date</TableHead>
                    <TableHead>Source</TableHead>
                    <TableHead>Destination</TableHead>
                    <TableHead>Statut</TableHead>
                    <TableHead className="w-24"></TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {transferts.map((t) => (
                    <TableRow key={t.id}>
                      <TableCell className="text-sm">{new Date(t.date).toLocaleDateString("fr-FR")}</TableCell>
                      <TableCell>{t.source_nom}</TableCell>
                      <TableCell>{t.dest_nom}</TableCell>
                      <TableCell>
                        <Badge variant={t.statut === "valide" ? "default" : "warning"}>
                          {t.statut === "valide" ? "Validé" : "En attente"}
                        </Badge>
                      </TableCell>
                      <TableCell>
                        {t.statut === "en_attente" && (
                          <Button
                            variant="outline"
                            size="sm"
                            disabled={validateTransfertMutation.isPending}
                            onClick={() => validateTransfertMutation.mutate(t.id)}
                          >
                            Valider
                          </Button>
                        )}
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            )}
          </CardContent>
        </Card>
      </div>

      {/* Stock per magasin viewer */}
      {stockMagasinId && (
        <Card>
          <CardHeader className="flex flex-row items-center justify-between">
            <CardTitle className="flex items-center gap-2">
              <Package className="h-5 w-5" />
              Stock — {magasins.find((m) => m.id === stockMagasinId)?.nom}
            </CardTitle>
            <Button variant="ghost" size="sm" onClick={() => setStockMagasinId(null)}>Fermer</Button>
          </CardHeader>
          <CardContent>
            {stockLines.length === 0 ? (
              <p className="text-center text-muted-foreground py-4">Aucun article avec du stock dans cette boutique</p>
            ) : (
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Article</TableHead>
                    <TableHead>Code-barres</TableHead>
                    <TableHead className="text-right">Stock</TableHead>
                    <TableHead className="text-right">Seuil alerte</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {stockLines.map((s) => (
                    <TableRow key={s.id}>
                      <TableCell className="font-medium">{s.designation}</TableCell>
                      <TableCell className="text-muted-foreground">{s.code_barre || "—"}</TableCell>
                      <TableCell className="text-right">
                        <Badge variant={s.stock <= 0 ? "destructive" : s.stock_alerte && s.stock <= s.stock_alerte ? "warning" : "outline"}>
                          {s.stock}
                        </Badge>
                      </TableCell>
                      <TableCell className="text-right text-muted-foreground">{s.stock_alerte ?? "—"}</TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            )}
          </CardContent>
        </Card>
      )}

      {/* Add/Edit Dialog */}
      <Dialog open={showForm} onOpenChange={(open) => { if (!open) closeForm() }}>
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle>{editingId ? "Modifier la boutique" : "Nouvelle boutique"}</DialogTitle>
          </DialogHeader>
          <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="nom">Nom</Label>
              <Input id="nom" {...form.register("nom")} placeholder="Ex: Magasin Centre-Ville" />
              {form.formState.errors.nom && <p className="text-sm text-destructive">{form.formState.errors.nom.message}</p>}
            </div>
            <div className="space-y-2">
              <Label htmlFor="adresse">Adresse</Label>
              <Input id="adresse" {...form.register("adresse")} placeholder="Ex: 45 Bd Zerktouni, Casablanca" />
            </div>
            <DialogFooter>
              <Button type="button" variant="outline" onClick={closeForm}>Annuler</Button>
              <Button type="submit" disabled={addMutation.isPending || updateMutation.isPending}>
                {(addMutation.isPending || updateMutation.isPending) && <Loader2 className="h-4 w-4 animate-spin mr-2" />}
                {editingId ? "Enregistrer" : "Ajouter"}
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>

      {/* Delete Confirmation */}
      <Dialog open={!!deleteTarget} onOpenChange={(open) => { if (!open) setDeleteTarget(null) }}>
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <AlertTriangle className="h-5 w-5 text-destructive" />
              Supprimer la boutique
            </DialogTitle>
          </DialogHeader>
          <p className="text-muted-foreground">
            Voulez-vous vraiment supprimer <strong>{deleteTarget?.nom}</strong> ? Cette action supprimera
            aussi les données de stock associées et ne peut pas être annulée.
          </p>
          <DialogFooter>
            <Button variant="outline" onClick={() => setDeleteTarget(null)}>Annuler</Button>
            <Button
              variant="destructive"
              disabled={deleteMutation.isPending}
              onClick={() => deleteTarget && deleteMutation.mutate(deleteTarget.id)}
            >
              {deleteMutation.isPending && <Loader2 className="h-4 w-4 animate-spin mr-2" />}
              Supprimer
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Transfert Dialog */}
      <Dialog open={showTransfert} onOpenChange={(open) => { if (!open) { setShowTransfert(false); setTransfertLines([]); transfertForm.reset() } }}>
        <DialogContent className="sm:max-w-lg">
          <DialogHeader>
            <DialogTitle>Nouveau transfert de stock</DialogTitle>
          </DialogHeader>
          <form
            onSubmit={transfertForm.handleSubmit((data) => {
              if (data.source_id === data.dest_id) {
                toast.error("La source et la destination doivent être différentes")
                return
              }
              if (transfertLines.length === 0) {
                toast.error("Ajoutez au moins un article au transfert")
                return
              }
              createTransfertMutation.mutate({
                sourceId: parseInt(data.source_id),
                destId: parseInt(data.dest_id),
                articles: transfertLines.map((l) => ({ article_id: l.article_id, quantite: l.quantite })),
              })
            })}
            className="space-y-4"
          >
            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-2">
                <Label>Source</Label>
                <Select value={transfertForm.watch("source_id")} onValueChange={(v) => { transfertForm.setValue("source_id", v); setStockMagasinId(parseInt(v)) }}>
                  <SelectTrigger>
                    <SelectValue placeholder="Boutique source" />
                  </SelectTrigger>
                  <SelectContent>
                    {magasins.map((m) => (
                      <SelectItem key={m.id} value={String(m.id)}>{m.nom}</SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
              <div className="space-y-2">
                <Label>Destination</Label>
                <Select value={transfertForm.watch("dest_id")} onValueChange={(v) => transfertForm.setValue("dest_id", v)}>
                  <SelectTrigger>
                    <SelectValue placeholder="Boutique destination" />
                  </SelectTrigger>
                  <SelectContent>
                    {magasins.filter((m) => String(m.id) !== sourceId).map((m) => (
                      <SelectItem key={m.id} value={String(m.id)}>{m.nom}</SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
            </div>

            {sourceId && stockLines.length > 0 && (
              <div className="space-y-2">
                <Label>Articles à transférer</Label>
                <div className="max-h-48 overflow-y-auto border rounded-lg divide-y">
                  {stockLines.filter((s) => s.stock > 0).map((s) => {
                    const line = transfertLines.find((l) => l.article_id === s.id)
                    return (
                      <div key={s.id} className="flex items-center justify-between px-3 py-2 text-sm">
                        <div className="flex-1">
                          <span className="font-medium">{s.designation}</span>
                          <span className="text-muted-foreground ml-2">(stock: {s.stock})</span>
                        </div>
                        <Input
                          type="number"
                          className="w-20 h-8 text-center"
                          min={0}
                          max={s.stock}
                          value={line?.quantite ?? ""}
                          placeholder="0"
                          onChange={(e) => {
                            const q = parseFloat(e.target.value) || 0
                            if (q <= 0) {
                              setTransfertLines((prev) => prev.filter((l) => l.article_id !== s.id))
                            } else {
                              setTransfertLines((prev) => {
                                const existing = prev.find((l) => l.article_id === s.id)
                                if (existing) return prev.map((l) => l.article_id === s.id ? { ...l, quantite: Math.min(q, s.stock) } : l)
                                return [...prev, { article_id: s.id, designation: s.designation, quantite: Math.min(q, s.stock) }]
                              })
                            }
                          }}
                        />
                      </div>
                    )
                  })}
                </div>
                {transfertLines.length > 0 && (
                  <p className="text-sm text-muted-foreground">
                    {transfertLines.length} article(s) sélectionné(s) — {transfertLines.reduce((sum, l) => sum + l.quantite, 0)} unités au total
                  </p>
                )}
              </div>
            )}

            <DialogFooter>
              <Button type="button" variant="outline" onClick={() => { setShowTransfert(false); setTransfertLines([]); transfertForm.reset() }}>Annuler</Button>
              <Button type="submit" disabled={createTransfertMutation.isPending}>
                {createTransfertMutation.isPending && <Loader2 className="h-4 w-4 animate-spin mr-2" />}
                Créer le transfert
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>
    </div>
  )
}
