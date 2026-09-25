import { useState } from "react"
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query"
import { invoke } from "@/lib/tauri"
import { Card, CardContent, CardHeader, CardTitle } from "@/ui/Card"
import { Button } from "@/ui/Button"
import { Input } from "@/ui/Input"
import { Label } from "@/ui/Label"
import { Badge } from "@/ui/Badge"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/ui/Table"
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from "@/ui/Dialog"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/ui/Select"
import { toast } from "sonner"
import { Plus, ClipboardCheck, Loader2, CheckCircle, AlertTriangle, ArrowLeft } from "lucide-react"
import PageHeader from "@/components/PageHeader"
import EmptyState from "@/components/EmptyState"
import { formatDate, formatDateTime } from "@/lib/utils"

interface Inventaire {
  id: number
  date_debut: string
  date_fin: string | null
  statut: string
  magasin_id: number
  utilisateur_id: number | null
  nb_articles: number
  nb_comptes: number
}

interface InventaireLigne {
  id: number
  article_id: number
  designation: string
  code_barre: string | null
  stock_theorique: number
  stock_compte: number | null
  ecart: number | null
}

interface InventaireDetail {
  id: number
  date_debut: string
  date_fin: string | null
  statut: string
  magasin_id: number
  lignes: InventaireLigne[]
}

interface Magasin {
  id: number
  nom: string
}

export default function Inventaire() {
  const queryClient = useQueryClient()
  const [selectedId, setSelectedId] = useState<number | null>(null)
  const [showCreate, setShowCreate] = useState(false)
  const [newMagasinId, setNewMagasinId] = useState("")
  const [showValidate, setShowValidate] = useState(false)
  const [searchLigne, setSearchLigne] = useState("")

  const { data: inventaires, isLoading } = useQuery({
    queryKey: ["inventaires"],
    queryFn: () => invoke<Inventaire[]>("get_inventaires"),
  })

  const { data: magasins } = useQuery({
    queryKey: ["magasins"],
    queryFn: () => invoke<Magasin[]>("get_magasins"),
  })

  const { data: detail, isLoading: loadingDetail } = useQuery({
    queryKey: ["inventaire", selectedId],
    queryFn: () => invoke<InventaireDetail>("get_inventaire", { inventaireId: selectedId }),
    enabled: !!selectedId,
  })

  const createMutation = useMutation({
    mutationFn: () => invoke<{ id: number }>("create_inventaire", {
      magasinId: parseInt(newMagasinId),
    }),
    onSuccess: (result) => {
      toast.success("Inventaire créé")
      queryClient.invalidateQueries({ queryKey: ["inventaires"] })
      setShowCreate(false)
      setSelectedId(result.id)
    },
    onError: (e) => toast.error("Erreur", { description: String(e) }),
  })

  const updateLigneMutation = useMutation({
    mutationFn: ({ ligneId, stockCompte }: { ligneId: number; stockCompte: number }) =>
      invoke("update_inventaire_ligne", { ligneId, stockCompte }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["inventaire", selectedId] })
    },
    onError: (e) => toast.error("Erreur", { description: String(e) }),
  })

  const validerMutation = useMutation({
    mutationFn: () => invoke("valider_inventaire", {
      inventaireId: selectedId,
    }),
    onSuccess: () => {
      toast.success("Inventaire validé — stock mis à jour")
      queryClient.invalidateQueries({ queryKey: ["inventaires"] })
      queryClient.invalidateQueries({ queryKey: ["inventaire", selectedId] })
      queryClient.invalidateQueries({ queryKey: ["articles"] })
      setShowValidate(false)
    },
    onError: (e) => toast.error("Erreur", { description: String(e) }),
  })

  const filteredLignes = detail?.lignes.filter((l) => {
    if (!searchLigne) return true
    const q = searchLigne.toLowerCase()
    return l.designation.toLowerCase().includes(q) || (l.code_barre && l.code_barre.toLowerCase().includes(q))
  })

  const comptees = detail?.lignes.filter((l) => l.stock_compte !== null).length ?? 0
  const totalLignes = detail?.lignes.length ?? 0
  const totalEcart = detail?.lignes.reduce((sum, l) => sum + (l.ecart ?? 0), 0) ?? 0

  if (selectedId) {
    return (
      <div className="space-y-6">
        <PageHeader
          title={`Inventaire #${selectedId}`}
          description={detail ? `${formatDateTime(detail.date_debut)} — ${detail.statut === "valide" ? "Validé" : "En cours"}` : "Chargement..."}
        >
          <div className="flex gap-2">
            <Button variant="outline" onClick={() => { setSelectedId(null); setSearchLigne("") }}>
              <ArrowLeft className="h-4 w-4 mr-2" />
              Retour
            </Button>
            {detail?.statut === "en_cours" && (
              <Button onClick={() => setShowValidate(true)} disabled={comptees === 0}>
                <CheckCircle className="h-4 w-4 mr-2" />
                Valider l'inventaire
              </Button>
            )}
          </div>
        </PageHeader>

        {loadingDetail ? (
          <div className="flex items-center justify-center py-12">
            <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
          </div>
        ) : detail ? (
          <>
            <div className="grid gap-4 md:grid-cols-3">
              <Card>
                <CardContent className="pt-6">
                  <p className="text-sm text-muted-foreground">Progression</p>
                  <p className="text-2xl font-bold">{comptees} / {totalLignes}</p>
                  <div className="mt-2 h-2 bg-muted rounded-full overflow-hidden">
                    <div
                      className="h-full bg-primary rounded-full transition-all"
                      style={{ width: totalLignes ? `${(comptees / totalLignes) * 100}%` : "0%" }}
                    />
                  </div>
                </CardContent>
              </Card>
              <Card>
                <CardContent className="pt-6">
                  <p className="text-sm text-muted-foreground">Statut</p>
                  <Badge variant={detail.statut === "valide" ? "default" : "warning"} className="mt-1 text-lg">
                    {detail.statut === "valide" ? "Validé" : "En cours"}
                  </Badge>
                </CardContent>
              </Card>
              <Card>
                <CardContent className="pt-6">
                  <p className="text-sm text-muted-foreground">Écart total</p>
                  <p className={`text-2xl font-bold ${totalEcart < 0 ? "text-destructive" : totalEcart > 0 ? "text-green-600" : ""}`}>
                    {totalEcart > 0 ? "+" : ""}{totalEcart}
                  </p>
                </CardContent>
              </Card>
            </div>

            <Card>
              <CardContent className="pt-6">
                <div className="mb-4">
                  <Input
                    placeholder="Rechercher un article (nom ou code-barres)..."
                    value={searchLigne}
                    onChange={(e) => setSearchLigne(e.target.value)}
                    className="max-w-md"
                  />
                </div>
                <div className="overflow-x-auto">
                  <Table>
                    <TableHeader>
                      <TableRow>
                        <TableHead>Code-barres</TableHead>
                        <TableHead>Désignation</TableHead>
                        <TableHead className="text-right">Stock théorique</TableHead>
                        <TableHead className="text-right w-[150px]">Stock compté</TableHead>
                        <TableHead className="text-right">Écart</TableHead>
                      </TableRow>
                    </TableHeader>
                    <TableBody>
                      {filteredLignes?.map((l) => (
                        <TableRow key={l.id}>
                          <TableCell className="font-mono text-sm">{l.code_barre || "—"}</TableCell>
                          <TableCell className="font-medium">{l.designation}</TableCell>
                          <TableCell className="text-right">{l.stock_theorique}</TableCell>
                          <TableCell className="text-right">
                            {detail.statut === "en_cours" ? (
                              <Input
                                type="number"
                                step="1"
                                min="0"
                                className="w-24 ml-auto text-right"
                                defaultValue={l.stock_compte ?? ""}
                                placeholder="—"
                                onBlur={(e) => {
                                  const val = e.target.value
                                  if (val === "") return
                                  const n = parseFloat(val)
                                  if (!isNaN(n) && n !== l.stock_compte) {
                                    updateLigneMutation.mutate({ ligneId: l.id, stockCompte: n })
                                  }
                                }}
                              />
                            ) : (
                              l.stock_compte ?? "—"
                            )}
                          </TableCell>
                          <TableCell className="text-right">
                            {l.ecart !== null ? (
                              <span className={l.ecart < 0 ? "text-destructive font-medium" : l.ecart > 0 ? "text-green-600 font-medium" : ""}>
                                {l.ecart > 0 ? "+" : ""}{l.ecart}
                              </span>
                            ) : (
                              "—"
                            )}
                          </TableCell>
                        </TableRow>
                      ))}
                      {!filteredLignes?.length && (
                        <TableRow>
                          <TableCell colSpan={5} className="text-center py-6 text-muted-foreground">
                            Aucun article
                          </TableCell>
                        </TableRow>
                      )}
                    </TableBody>
                  </Table>
                </div>
              </CardContent>
            </Card>

            <Dialog open={showValidate} onOpenChange={setShowValidate}>
              <DialogContent className="max-w-sm">
                <DialogHeader>
                  <DialogTitle className="flex items-center gap-2">
                    <AlertTriangle className="h-5 w-5 text-warning" />
                    Valider l'inventaire
                  </DialogTitle>
                </DialogHeader>
                <p className="text-sm text-muted-foreground">
                  Cette action va appliquer les écarts au stock réel.
                  <strong> {comptees}</strong> article(s) compté(s) sur {totalLignes}.
                  Les articles non comptés ne seront pas modifiés.
                </p>
                <DialogFooter className="gap-2">
                  <Button variant="outline" onClick={() => setShowValidate(false)}>Annuler</Button>
                  <Button onClick={() => validerMutation.mutate()} disabled={validerMutation.isPending}>
                    {validerMutation.isPending ? <Loader2 className="h-4 w-4 animate-spin mr-2" /> : <CheckCircle className="h-4 w-4 mr-2" />}
                    Confirmer la validation
                  </Button>
                </DialogFooter>
              </DialogContent>
            </Dialog>
          </>
        ) : null}
      </div>
    )
  }

  return (
    <div className="space-y-6">
      <PageHeader title="Inventaire physique" description="Comptage et ajustement du stock réel">
        <Button onClick={() => setShowCreate(true)}>
          <Plus className="h-4 w-4 mr-2" />
          Nouvel inventaire
        </Button>
      </PageHeader>

      {isLoading ? (
        <div className="flex items-center justify-center py-12">
          <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
        </div>
      ) : inventaires?.length ? (
        <Card>
          <CardContent className="pt-6">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>#</TableHead>
                  <TableHead>Date</TableHead>
                  <TableHead>Magasin</TableHead>
                  <TableHead>Progression</TableHead>
                  <TableHead>Statut</TableHead>
                  <TableHead className="w-[100px]">Actions</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {inventaires.map((inv) => {
                  const magasinNom = magasins?.find((m) => m.id === inv.magasin_id)?.nom ?? `#${inv.magasin_id}`
                  return (
                    <TableRow key={inv.id}>
                      <TableCell className="font-mono">{inv.id}</TableCell>
                      <TableCell>{formatDateTime(inv.date_debut)}</TableCell>
                      <TableCell>{magasinNom}</TableCell>
                      <TableCell>
                        <div className="flex items-center gap-2">
                          <div className="w-20 h-2 bg-muted rounded-full overflow-hidden">
                            <div
                              className="h-full bg-primary rounded-full"
                              style={{ width: inv.nb_articles ? `${(inv.nb_comptes / inv.nb_articles) * 100}%` : "0%" }}
                            />
                          </div>
                          <span className="text-sm text-muted-foreground">{inv.nb_comptes}/{inv.nb_articles}</span>
                        </div>
                      </TableCell>
                      <TableCell>
                        <Badge variant={inv.statut === "valide" ? "default" : "warning"}>
                          {inv.statut === "valide" ? "Validé" : "En cours"}
                        </Badge>
                      </TableCell>
                      <TableCell>
                        <Button variant="ghost" size="sm" onClick={() => setSelectedId(inv.id)}>
                          <ClipboardCheck className="h-4 w-4 mr-1" />
                          {inv.statut === "en_cours" ? "Compter" : "Voir"}
                        </Button>
                      </TableCell>
                    </TableRow>
                  )
                })}
              </TableBody>
            </Table>
          </CardContent>
        </Card>
      ) : (
        <Card>
          <CardContent className="py-12">
            <EmptyState
              icon={<ClipboardCheck className="h-12 w-12" />}
              title="Aucun inventaire"
              description="Créez un inventaire pour comparer le stock physique au stock théorique"
            />
          </CardContent>
        </Card>
      )}

      <Dialog open={showCreate} onOpenChange={setShowCreate}>
        <DialogContent className="max-w-sm">
          <DialogHeader>
            <DialogTitle>Nouvel inventaire</DialogTitle>
          </DialogHeader>
          <div className="space-y-4">
            <div className="space-y-2">
              <Label>Boutique *</Label>
              <Select value={newMagasinId} onValueChange={setNewMagasinId}>
                <SelectTrigger>
                  <SelectValue placeholder="Sélectionner une boutique" />
                </SelectTrigger>
                <SelectContent>
                  {magasins?.map((m) => (
                    <SelectItem key={m.id} value={String(m.id)}>{m.nom}</SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setShowCreate(false)}>Annuler</Button>
            <Button
              onClick={() => createMutation.mutate()}
              disabled={!newMagasinId || createMutation.isPending}
            >
              {createMutation.isPending ? <Loader2 className="h-4 w-4 animate-spin mr-2" /> : <Plus className="h-4 w-4 mr-2" />}
              Créer
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  )
}
