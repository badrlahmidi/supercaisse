import { useState } from "react"
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query"
import { invoke } from "@/lib/tauri"
import { Card, CardContent, CardHeader, CardTitle } from "@/ui/Card"
import { Button } from "@/ui/Button"
import { Input } from "@/ui/Input"
import { Label } from "@/ui/Label"
import { Badge } from "@/ui/Badge"
import { Textarea } from "@/ui/Textarea"
import { Table, TableHeader, TableBody, TableRow, TableHead, TableCell } from "@/ui/Table"
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from "@/ui/Dialog"
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/ui/Tabs"
import { toast } from "sonner"
import { useForm } from "react-hook-form"
import { zodResolver } from "@hookform/resolvers/zod"
import { z } from "zod"
import PageHeader from "@/components/PageHeader"
import { formatCurrency, formatDateTime } from "@/lib/utils"
import { Plus, XCircle, Monitor, Loader2, Banknote, CreditCard, Landmark, ArrowRightLeft } from "lucide-react"
import { sommeDH } from "@/lib/totaux"

interface Caisse {
  id: number
  nom: string
  utilisateur_id: number | null
  statut: string
  ouverture_date: string | null
  fermeture_date: string | null
  fond_initial: number
  recettes_especes: number
  recettes_cb: number
  recettes_cheque: number
  recettes_virement: number
  depenses: number
  ecart: number
  note: string | null
  utilisateur_nom: string | null
}

interface TresoreriePeriode {
  especes: number
  cb: number
  cheque: number
  virement: number
  total: number
}

interface Tresorerie {
  jour: TresoreriePeriode
  semaine: TresoreriePeriode
  mois: TresoreriePeriode
}

const openSchema = z.object({
  nom: z.string().min(1, "Nom requis"),
  fond_initial: z.coerce.number().min(0, "Le fond initial doit être positif"),
})

type OpenForm = z.infer<typeof openSchema>

const closeSchema = z.object({
  note: z.string().optional(),
})

type CloseForm = z.infer<typeof closeSchema>

export default function Caisses() {
  const queryClient = useQueryClient()
  const [showOpen, setShowOpen] = useState(false)
  const [closingCaisse, setClosingCaisse] = useState<Caisse | null>(null)
  const [tresoTab, setTresoTab] = useState("jour")

  const { data: caisses = [], isLoading } = useQuery({
    queryKey: ["caisses"],
    queryFn: () => invoke<Caisse[]>("get_caisses"),
  })

  const { data: tresorerie } = useQuery({
    queryKey: ["tresorerie"],
    queryFn: () => invoke<Tresorerie>("get_tresorerie"),
  })

  const openForm = useForm<OpenForm>({
    resolver: zodResolver(openSchema),
    defaultValues: { nom: "", fond_initial: 0 },
  })

  const closeForm = useForm<CloseForm>({
    resolver: zodResolver(closeSchema),
    defaultValues: { note: "" },
  })

  const openMutation = useMutation({
    mutationFn: (data: OpenForm) =>
      invoke<number>("open_caisse", {
        nom: data.nom,
        fondInitial: data.fond_initial,
        utilisateurId: null,
      }),
    onSuccess: () => {
      toast.success("Caisse ouverte")
      queryClient.invalidateQueries({ queryKey: ["caisses"] })
      queryClient.invalidateQueries({ queryKey: ["tresorerie"] })
      setShowOpen(false)
      openForm.reset()
    },
    onError: (e) => toast.error("Erreur", { description: String(e) }),
  })

  const closeMutation = useMutation({
    mutationFn: (data: { id: number; note?: string }) =>
      invoke("close_caisse", { id: data.id, note: data.note || null }),
    onSuccess: () => {
      toast.success("Caisse fermée")
      queryClient.invalidateQueries({ queryKey: ["caisses"] })
      queryClient.invalidateQueries({ queryKey: ["tresorerie"] })
      setClosingCaisse(null)
      closeForm.reset()
    },
    onError: (e) => toast.error("Erreur", { description: String(e) }),
  })

  const caissesOuvertes = caisses.filter((c) => c.statut === "ouverte")
  const recettesDuJour = tresorerie?.jour?.total ?? 0
  const fondTotal = sommeDH(caissesOuvertes.map((c) => c.fond_initial))

  const totalRecettes = (c: Caisse) =>
    c.recettes_especes + c.recettes_cb + c.recettes_cheque + c.recettes_virement

  const currentTreso = tresorerie
    ? tresoTab === "jour"
      ? tresorerie.jour
      : tresoTab === "semaine"
        ? tresorerie.semaine
        : tresorerie.mois
    : null

  return (
    <div className="space-y-6 p-6">
      <PageHeader title="Gestion des caisses" description="Multi-caisse avec vue trésorerie consolidée">
        <Button onClick={() => setShowOpen(true)}>
          <Plus className="h-4 w-4 mr-2" />
          Ouvrir une caisse
        </Button>
      </PageHeader>

      <div className="grid gap-4 md:grid-cols-3">
        <Card>
          <CardHeader className="flex flex-row items-center justify-between pb-2">
            <CardTitle className="text-sm font-medium">Caisses ouvertes</CardTitle>
            <Monitor className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{caissesOuvertes.length}</div>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="flex flex-row items-center justify-between pb-2">
            <CardTitle className="text-sm font-medium">Recettes du jour</CardTitle>
            <Banknote className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{formatCurrency(recettesDuJour)}</div>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="flex flex-row items-center justify-between pb-2">
            <CardTitle className="text-sm font-medium">Fond total</CardTitle>
            <Landmark className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{formatCurrency(fondTotal)}</div>
          </CardContent>
        </Card>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Liste des caisses</CardTitle>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="flex justify-center py-8">
              <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
            </div>
          ) : caisses.length === 0 ? (
            <p className="text-center text-muted-foreground py-8">Aucune caisse enregistrée</p>
          ) : (
            <div className="overflow-x-auto">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Nom</TableHead>
                    <TableHead>Statut</TableHead>
                    <TableHead>Utilisateur</TableHead>
                    <TableHead>Ouverture</TableHead>
                    <TableHead>Fermeture</TableHead>
                    <TableHead className="text-right">Fond initial</TableHead>
                    <TableHead className="text-right">Total recettes</TableHead>
                    <TableHead className="text-right">Écart</TableHead>
                    <TableHead></TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {caisses.map((c) => (
                    <TableRow key={c.id}>
                      <TableCell className="font-medium">{c.nom}</TableCell>
                      <TableCell>
                        <Badge variant={c.statut === "ouverte" ? "default" : "secondary"}>
                          {c.statut === "ouverte" ? "Ouverte" : "Fermée"}
                        </Badge>
                      </TableCell>
                      <TableCell>{c.utilisateur_nom ?? "—"}</TableCell>
                      <TableCell>{formatDateTime(c.ouverture_date)}</TableCell>
                      <TableCell>{formatDateTime(c.fermeture_date)}</TableCell>
                      <TableCell className="text-right">{formatCurrency(c.fond_initial)}</TableCell>
                      <TableCell className="text-right">{formatCurrency(totalRecettes(c))}</TableCell>
                      <TableCell className="text-right">{formatCurrency(c.ecart)}</TableCell>
                      <TableCell>
                        {c.statut === "ouverte" && (
                          <Button
                            variant="outline"
                            size="sm"
                            onClick={() => {
                              setClosingCaisse(c)
                              closeForm.reset({ note: "" })
                            }}
                          >
                            <XCircle className="h-4 w-4 mr-1" />
                            Fermer
                          </Button>
                        )}
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </div>
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Trésorerie consolidée</CardTitle>
        </CardHeader>
        <CardContent>
          <Tabs value={tresoTab} onValueChange={setTresoTab}>
            <TabsList>
              <TabsTrigger value="jour">Aujourd'hui</TabsTrigger>
              <TabsTrigger value="semaine">Cette semaine</TabsTrigger>
              <TabsTrigger value="mois">Ce mois</TabsTrigger>
            </TabsList>
            <TabsContent value={tresoTab}>
              {currentTreso ? (
                <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4 mt-4">
                  <Card>
                    <CardHeader className="flex flex-row items-center justify-between pb-2">
                      <CardTitle className="text-sm font-medium">Espèces</CardTitle>
                      <Banknote className="h-4 w-4 text-muted-foreground" />
                    </CardHeader>
                    <CardContent>
                      <div className="text-xl font-bold">{formatCurrency(currentTreso.especes)}</div>
                    </CardContent>
                  </Card>
                  <Card>
                    <CardHeader className="flex flex-row items-center justify-between pb-2">
                      <CardTitle className="text-sm font-medium">Carte bancaire</CardTitle>
                      <CreditCard className="h-4 w-4 text-muted-foreground" />
                    </CardHeader>
                    <CardContent>
                      <div className="text-xl font-bold">{formatCurrency(currentTreso.cb)}</div>
                    </CardContent>
                  </Card>
                  <Card>
                    <CardHeader className="flex flex-row items-center justify-between pb-2">
                      <CardTitle className="text-sm font-medium">Chèque</CardTitle>
                      <Landmark className="h-4 w-4 text-muted-foreground" />
                    </CardHeader>
                    <CardContent>
                      <div className="text-xl font-bold">{formatCurrency(currentTreso.cheque)}</div>
                    </CardContent>
                  </Card>
                  <Card>
                    <CardHeader className="flex flex-row items-center justify-between pb-2">
                      <CardTitle className="text-sm font-medium">Virement</CardTitle>
                      <ArrowRightLeft className="h-4 w-4 text-muted-foreground" />
                    </CardHeader>
                    <CardContent>
                      <div className="text-xl font-bold">{formatCurrency(currentTreso.virement)}</div>
                    </CardContent>
                  </Card>
                </div>
              ) : (
                <div className="flex justify-center py-8">
                  <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
                </div>
              )}
              {currentTreso && (
                <div className="mt-4 text-right">
                  <span className="text-sm text-muted-foreground mr-2">Total :</span>
                  <span className="text-lg font-bold">{formatCurrency(currentTreso.total)}</span>
                </div>
              )}
            </TabsContent>
          </Tabs>
        </CardContent>
      </Card>

      <Dialog open={showOpen} onOpenChange={setShowOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Ouvrir une nouvelle caisse</DialogTitle>
          </DialogHeader>
          <form onSubmit={openForm.handleSubmit((data) => openMutation.mutate(data))} className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="nom">Nom de la caisse</Label>
              <Input id="nom" {...openForm.register("nom")} placeholder="Caisse 1" />
              {openForm.formState.errors.nom && (
                <p className="text-sm text-destructive">{openForm.formState.errors.nom.message}</p>
              )}
            </div>
            <div className="space-y-2">
              <Label htmlFor="fond_initial">Fond initial (DH)</Label>
              <Input id="fond_initial" type="number" step="0.01" {...openForm.register("fond_initial")} />
              {openForm.formState.errors.fond_initial && (
                <p className="text-sm text-destructive">{openForm.formState.errors.fond_initial.message}</p>
              )}
            </div>
            <DialogFooter>
              <Button type="button" variant="outline" onClick={() => setShowOpen(false)}>
                Annuler
              </Button>
              <Button type="submit" disabled={openMutation.isPending}>
                {openMutation.isPending && <Loader2 className="h-4 w-4 mr-2 animate-spin" />}
                Ouvrir
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>

      <Dialog open={!!closingCaisse} onOpenChange={(open) => { if (!open) setClosingCaisse(null) }}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Fermer la caisse : {closingCaisse?.nom}</DialogTitle>
          </DialogHeader>
          {closingCaisse && (
            <form
              onSubmit={closeForm.handleSubmit((data) =>
                closeMutation.mutate({ id: closingCaisse.id, note: data.note })
              )}
              className="space-y-4"
            >
              <div className="rounded-lg border p-4 space-y-2">
                <div className="flex justify-between text-sm">
                  <span className="text-muted-foreground">Fond initial</span>
                  <span className="font-medium">{formatCurrency(closingCaisse.fond_initial)}</span>
                </div>
                <div className="flex justify-between text-sm">
                  <span className="text-muted-foreground">Ouverture</span>
                  <span className="font-medium">{formatDateTime(closingCaisse.ouverture_date)}</span>
                </div>
              </div>
              <div className="space-y-2">
                <Label htmlFor="note">Note de fermeture</Label>
                <Textarea id="note" {...closeForm.register("note")} placeholder="Observations..." rows={3} />
              </div>
              <DialogFooter>
                <Button type="button" variant="outline" onClick={() => setClosingCaisse(null)}>
                  Annuler
                </Button>
                <Button type="submit" variant="destructive" disabled={closeMutation.isPending}>
                  {closeMutation.isPending && <Loader2 className="h-4 w-4 mr-2 animate-spin" />}
                  Confirmer la fermeture
                </Button>
              </DialogFooter>
            </form>
          )}
        </DialogContent>
      </Dialog>
    </div>
  )
}
