import { useState } from "react"
import { useAchatsList, useCreateAchat, useUpdateAchatStatus } from "@/hooks/useAchats"
import { useProductsList } from "@/hooks/useProducts"
import { useFournisseursList } from "@/hooks/useFournisseurs"
import { Card, CardContent } from "@/ui/Card"
import { Button } from "@/ui/Button"
import { Input } from "@/ui/Input"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/ui/Table"
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "@/ui/Dialog"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/ui/Select"
import { Label } from "@/ui/Label"
import { useForm } from "react-hook-form"
import { zodResolver } from "@hookform/resolvers/zod"
import { z } from "zod"
import { Plus, Search, Loader2, Trash2, SearchX } from "lucide-react"
import { formatCurrency, formatDate } from "@/lib/utils"
import PageHeader from "@/components/PageHeader"
import EmptyState from "@/components/EmptyState"

interface Achat {
  id: number
  date: string
  fournisseur_id: number | null
  reference: string | null
  montant_total: number
  statut: string
  statut_livraison: string
  statut_paiement: string
  fournisseur_nom: string | null
}

interface Fournisseur {
  id: number
  nom: string
}

interface Article {
  id: number
  designation: string
  prix_achat: number
}

const achatSchema = z.object({
  fournisseur_id: z.number().min(1, "Fournisseur requis"),
  reference: z.string().optional().nullable(),
  articles: z.array(z.object({
    article_id: z.number().min(1, "Article requis"),
    quantite: z.number().min(0.01, "Quantité requise"),
    prix_unitaire: z.number().min(0, "Prix requis"),
  })).min(1, "Au moins un article requis"),
})

type AchatForm = z.infer<typeof achatSchema>

export default function Achats() {
  const [search, setSearch] = useState("")
  const [showForm, setShowForm] = useState(false)
  const [articleLines, setArticleLines] = useState<Array<{article_id: number, quantite: number, prix_unitaire: number}>>([{article_id: 0, quantite: 1, prix_unitaire: 0}])

  const { data: achats, isLoading: _isLoading } = useAchatsList()

  const { data: fournisseurs } = useFournisseursList()

  const { data: articles } = useProductsList()

  const createMutation = useCreateAchat()
  const updateStatusMutation = useUpdateAchatStatus()

  const form = useForm<AchatForm>({
    resolver: zodResolver(achatSchema),
    defaultValues: { fournisseur_id: 0, reference: "", articles: [] },
  })

  const handleSubmit = (data: AchatForm) => {
    createMutation.mutate(
      {...data, articles: articleLines.filter(l => l.article_id > 0)},
      {
        onSuccess: () => {
          setShowForm(false)
          setArticleLines([{article_id: 0, quantite: 1, prix_unitaire: 0}])
        }
      }
    )
  }

  const openCreate = () => setShowForm(true)
  const addLine = () => setArticleLines([...articleLines, {article_id: 0, quantite: 1, prix_unitaire: 0}])
  const removeLine = (idx: number) => setArticleLines(articleLines.filter((_, i) => i !== idx))
  const updateLine = (idx: number, field: string, value: number) => {
    setArticleLines(articleLines.map((l, i) => i === idx ? {...l, [field]: value} : l))
  }

  return (
    <div className="space-y-6">
      <PageHeader title="Achats" description="Gestion des achats fournisseurs">
        <Button onClick={openCreate}>
          <Plus className="h-4 w-4 mr-2" />
          Nouvel achat
        </Button>
      </PageHeader>

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
          </div>

          <div className="overflow-x-auto">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Référence</TableHead>
                  <TableHead>Fournisseur</TableHead>
                  <TableHead>Date</TableHead>
                  <TableHead className="text-right">Montant</TableHead>
                  <TableHead>Livraison</TableHead>
                  <TableHead>Paiement</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {achats?.filter((a) =>
                  a.reference?.toLowerCase().includes(search.toLowerCase()) ||
                  a.fournisseur_nom?.toLowerCase().includes(search.toLowerCase())
                ).map((achat) => (
                  <TableRow key={achat.id}>
                    <TableCell className="font-mono text-sm">{achat.reference || `ACH-${achat.id}`}</TableCell>
                    <TableCell>{achat.fournisseur_nom || "—"}</TableCell>
                    <TableCell>{formatDate(achat.date)}</TableCell>
                    <TableCell className="text-right font-medium">{formatCurrency(achat.montant_total)}</TableCell>
                    <TableCell>
                      <button 
                        onClick={() => updateStatusMutation.mutate({ achatId: achat.id, statutLivraison: achat.statut_livraison === "recu" ? "en_attente" : "recu", statutPaiement: achat.statut_paiement })}
                        disabled={updateStatusMutation.isPending}
                        className={`px-2 py-1 rounded text-xs font-medium cursor-pointer transition-colors ${
                          achat.statut_livraison === "recu" ? "bg-success/10 text-success hover:bg-success/20" :
                          "bg-warning/10 text-warning hover:bg-warning/20"
                        }`}
                      >
                        {achat.statut_livraison === "recu" ? "Reçu (Stock OK)" : "En attente"}
                      </button>
                    </TableCell>
                    <TableCell>
                      <button 
                        onClick={() => updateStatusMutation.mutate({ achatId: achat.id, statutLivraison: achat.statut_livraison, statutPaiement: achat.statut_paiement === "paye" ? "non_paye" : "paye" })}
                        disabled={updateStatusMutation.isPending}
                        className={`px-2 py-1 rounded text-xs font-medium cursor-pointer transition-colors ${
                          achat.statut_paiement === "paye" ? "bg-success/10 text-success hover:bg-success/20" :
                          "bg-destructive/10 text-destructive hover:bg-destructive/20"
                        }`}
                      >
                        {achat.statut_paiement === "paye" ? "Payé" : "Non payé"}
                      </button>
                    </TableCell>
                  </TableRow>
                ))}
                {!achats?.length && (
                  <TableRow>
                    <TableCell colSpan={8}>
                      <EmptyState icon={<SearchX className="h-12 w-12" />} title="Aucun achat trouvé" description="Commencez par enregistrer un achat" />
                    </TableCell>
                  </TableRow>
                )}
              </TableBody>
            </Table>
          </div>
        </CardContent>
      </Card>

      <Dialog open={showForm} onOpenChange={setShowForm}>
        <DialogContent className="max-w-3xl max-h-[90vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle>Nouvel achat fournisseur</DialogTitle>
          </DialogHeader>
          <form onSubmit={form.handleSubmit(handleSubmit)} className="space-y-4">
            <div className="grid gap-4 md:grid-cols-2">
              <div className="space-y-2">
                <Label htmlFor="fournisseur_id">Fournisseur *</Label>
                <Select
                  value={form.watch("fournisseur_id") ? String(form.watch("fournisseur_id")) : ""}
                  onValueChange={(v) => form.setValue("fournisseur_id", v ? parseInt(v) : 0)}
                >
                  <SelectTrigger id="fournisseur_id">
                    <SelectValue placeholder="Sélectionner un fournisseur" />
                  </SelectTrigger>
                  <SelectContent>
                    {fournisseurs?.map((f) => (
                      <SelectItem key={f.id} value={String(f.id)}>{f.nom}</SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
              <div className="space-y-2">
                <Label htmlFor="reference">Référence</Label>
                <Input {...form.register("reference")} id="reference" placeholder="Optionnel" />
              </div>
            </div>

            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <Label>Articles</Label>
                <Button type="button" variant="outline" size="sm" onClick={addLine}>
                  <Plus className="h-3.5 w-3.5 mr-1" />
                  Ajouter ligne
                </Button>
              </div>
              <div className="space-y-2">
                {articleLines.map((line, idx) => (
                  <div key={idx} className="flex gap-2 p-3 bg-muted/30 rounded-lg">
                    <div className="flex-1">
                      <Label>Article</Label>
                      <Select
                        value={String(line.article_id)}
                        onValueChange={(v) => updateLine(idx, "article_id", parseInt(v))}
                      >
                        <SelectTrigger>
                          <SelectValue placeholder="Choisir un article" />
                        </SelectTrigger>
                        <SelectContent>
                          {articles?.map((a) => (
                            <SelectItem key={a.id} value={String(a.id)}>
                              {a.designation} ({formatCurrency(a.prix_achat)})
                            </SelectItem>
                          ))}
                        </SelectContent>
                      </Select>
                    </div>
                    <div className="w-32">
                      <Label>Quantité</Label>
                      <Input
                        type="number"
                        step="0.01"
                        min="0.01"
                        value={line.quantite}
                        onChange={(e) => updateLine(idx, "quantite", parseFloat(e.target.value) || 0)}
                      />
                    </div>
                    <div className="w-40">
                      <Label>Prix unitaire</Label>
                      <Input
                        type="number"
                        step="0.01"
                        min="0"
                        value={line.prix_unitaire}
                        onChange={(e) => updateLine(idx, "prix_unitaire", parseFloat(e.target.value) || 0)}
                      />
                    </div>
                    <div className="flex items-end">
                      <Button type="button" variant="ghost" size="icon" onClick={() => removeLine(idx)}>
                        <Trash2 className="h-4 w-4 text-destructive" />
                      </Button>
                    </div>
                  </div>
                ))}
              </div>
            </div>

            <div className="flex justify-end gap-4 pt-4 border-t">
              <Button type="button" variant="outline" onClick={() => setShowForm(false)}>
                Annuler
              </Button>
              <Button type="submit" disabled={createMutation.isPending}>
                {createMutation.isPending ? (
                  <>
                    <Loader2 className="h-4 w-4 animate-spin mr-2" />
                    Enregistrement...
                  </>
                ) : (
                  "Créer l'achat"
                )}
              </Button>
            </div>
          </form>
        </DialogContent>
      </Dialog>
    </div>
  )
}