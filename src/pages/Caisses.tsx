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
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/ui/Tabs"
import { toast } from "sonner"
import { useForm } from "react-hook-form"
import { zodResolver } from "@hookform/resolvers/zod"
import { z } from "zod"
import PageHeader from "@/components/PageHeader"
import { formatCurrency, formatDateTime } from "@/lib/utils"
import { XCircle, Monitor, Loader2, Banknote, CreditCard, Landmark, ArrowRightLeft } from "lucide-react"
import { sommeDH } from "@/lib/totaux"
import type { SessionSupervision } from "@/types/generated/SessionSupervision"

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

const closeSchema = z.object({
  total_especes_declare: z.coerce.number().min(0, "Le montant doit être positif"),
})

type CloseForm = z.infer<typeof closeSchema>

const totalRecettes = (s: SessionSupervision) =>
  sommeDH([s.recettes_especes, s.recettes_cb, s.recettes_cheque, s.recettes_virement])

const especesAttendues = (s: SessionSupervision) =>
  s.especes_attendu ?? sommeDH([s.fond_initial, s.recettes_especes, s.entrees, -s.sorties])

export default function Caisses() {
  const queryClient = useQueryClient()
  const [closingSession, setClosingSession] = useState<SessionSupervision | null>(null)
  const [tresoTab, setTresoTab] = useState("jour")

  const { data: sessions = [], isLoading } = useQuery({
    queryKey: ["caisses"],
    queryFn: () => invoke<SessionSupervision[]>("get_caisses"),
  })

  const { data: tresorerie } = useQuery({
    queryKey: ["tresorerie"],
    queryFn: () => invoke<Tresorerie>("get_tresorerie"),
  })

  const closeForm = useForm<CloseForm>({
    resolver: zodResolver(closeSchema),
    defaultValues: { total_especes_declare: 0 },
  })

  const closeMutation = useMutation({
    mutationFn: (data: { sessionId: number; totalEspecesDeclare: number }) =>
      invoke("close_session", data),
    onSuccess: () => {
      toast.success("Session clôturée")
      queryClient.invalidateQueries({ queryKey: ["caisses"] })
      queryClient.invalidateQueries({ queryKey: ["tresorerie"] })
      queryClient.invalidateQueries({ queryKey: ["session"] })
      setClosingSession(null)
      closeForm.reset()
    },
    onError: (e) => toast.error("Erreur à la clôture", { description: String(e) }),
  })

  const sessionsOuvertes = sessions.filter((s) => s.statut === "ouverte")
  const recettesDuJour = tresorerie?.jour?.total ?? 0
  const fondTotal = sommeDH(sessionsOuvertes.map((s) => s.fond_initial))

  const currentTreso = tresorerie
    ? tresoTab === "jour"
      ? tresorerie.jour
      : tresoTab === "semaine"
        ? tresorerie.semaine
        : tresorerie.mois
    : null

  return (
    <div className="space-y-6 p-6">
      <PageHeader title="Gestion des caisses" description="Sessions ouvertes par les caissiers depuis le point de vente, et trésorerie consolidée" />

      <div className="grid gap-4 md:grid-cols-3">
        <Card>
          <CardHeader className="flex flex-row items-center justify-between pb-2">
            <CardTitle className="text-sm font-medium">Sessions ouvertes</CardTitle>
            <Monitor className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{sessionsOuvertes.length}</div>
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
          <CardTitle>Sessions de caisse</CardTitle>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="flex justify-center py-8">
              <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
            </div>
          ) : sessions.length === 0 ? (
            <p className="text-center text-muted-foreground py-8">Aucune session de caisse. Les caissiers ouvrent leur session depuis le point de vente.</p>
          ) : (
            <div className="overflow-x-auto">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Session</TableHead>
                    <TableHead>Caissier</TableHead>
                    <TableHead>Statut</TableHead>
                    <TableHead>Ouverture</TableHead>
                    <TableHead>Clôture</TableHead>
                    <TableHead className="text-right">Fond initial</TableHead>
                    <TableHead className="text-right">Recettes</TableHead>
                    <TableHead className="text-right">Espèces attendues</TableHead>
                    <TableHead className="text-right">Écart</TableHead>
                    <TableHead><span className="sr-only">Actions</span></TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {sessions.map((s) => (
                    <TableRow key={s.id}>
                      <TableCell className="font-medium">
                        #{s.id}
                        {s.magasin_nom && <span className="block text-xs text-muted-foreground">{s.magasin_nom}</span>}
                      </TableCell>
                      <TableCell>{s.caissier_nom ?? "—"}</TableCell>
                      <TableCell>
                        <Badge variant={s.statut === "ouverte" ? "default" : "secondary"}>
                          {s.statut === "ouverte" ? "Ouverte" : "Clôturée"}
                        </Badge>
                      </TableCell>
                      <TableCell>{formatDateTime(s.date_ouverture)}</TableCell>
                      <TableCell>{s.date_cloture ? formatDateTime(s.date_cloture) : "—"}</TableCell>
                      <TableCell className="text-right">{formatCurrency(s.fond_initial)}</TableCell>
                      <TableCell className="text-right">{formatCurrency(totalRecettes(s))}</TableCell>
                      <TableCell className="text-right">{formatCurrency(especesAttendues(s))}</TableCell>
                      <TableCell className="text-right">
                        {s.ecart === null ? "—" : (
                          <span className={s.ecart === 0 ? "" : "text-destructive font-medium"}>{formatCurrency(s.ecart)}</span>
                        )}
                      </TableCell>
                      <TableCell>
                        {s.statut === "ouverte" && (
                          <Button
                            variant="outline"
                            size="sm"
                            onClick={() => {
                              setClosingSession(s)
                              closeForm.reset({ total_especes_declare: especesAttendues(s) })
                            }}
                          >
                            <XCircle className="h-4 w-4 mr-1" />
                            Clôturer
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

      <Dialog open={!!closingSession} onOpenChange={(open) => { if (!open) setClosingSession(null) }}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Clôturer la session #{closingSession?.id} ({closingSession?.caissier_nom ?? "—"})</DialogTitle>
          </DialogHeader>
          {closingSession && (
            <form
              onSubmit={closeForm.handleSubmit((data) =>
                closeMutation.mutate({ sessionId: closingSession.id, totalEspecesDeclare: data.total_especes_declare })
              )}
              className="space-y-4"
            >
              <div className="rounded-lg border p-4 space-y-2">
                <div className="flex justify-between text-sm">
                  <span className="text-muted-foreground">Fond initial</span>
                  <span className="font-medium">{formatCurrency(closingSession.fond_initial)}</span>
                </div>
                <div className="flex justify-between text-sm">
                  <span className="text-muted-foreground">Ventes en espèces</span>
                  <span className="font-medium">{formatCurrency(closingSession.recettes_especes)}</span>
                </div>
                <div className="flex justify-between text-sm">
                  <span className="text-muted-foreground">Entrées de caisse</span>
                  <span className="font-medium">{formatCurrency(closingSession.entrees)}</span>
                </div>
                <div className="flex justify-between text-sm">
                  <span className="text-muted-foreground">Sorties de caisse</span>
                  <span className="font-medium">{formatCurrency(-closingSession.sorties)}</span>
                </div>
                <div className="flex justify-between text-sm">
                  <span className="text-muted-foreground">Espèces attendues</span>
                  <span className="font-medium">{formatCurrency(especesAttendues(closingSession))}</span>
                </div>
              </div>
              <div className="space-y-2">
                <Label htmlFor="total_especes_declare">Espèces comptées (DH)</Label>
                <Input id="total_especes_declare" type="number" step="0.01" min="0" {...closeForm.register("total_especes_declare")} />
                {closeForm.formState.errors.total_especes_declare && (
                  <p className="text-sm text-destructive">{closeForm.formState.errors.total_especes_declare.message}</p>
                )}
              </div>
              <DialogFooter>
                <Button type="button" variant="outline" onClick={() => setClosingSession(null)}>
                  Annuler
                </Button>
                <Button type="submit" variant="destructive" disabled={closeMutation.isPending}>
                  {closeMutation.isPending && <Loader2 className="h-4 w-4 mr-2 animate-spin" />}
                  Confirmer la clôture
                </Button>
              </DialogFooter>
            </form>
          )}
        </DialogContent>
      </Dialog>
    </div>
  )
}
