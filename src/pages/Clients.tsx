import { useState } from "react"
import { useClientsList, useCreateClient, useUpdateClient, useDeleteClient, useAddPaiement } from "@/hooks/useClients"
import { Card, CardContent } from "@/ui/Card"
import { Button } from "@/ui/Button"
import { Input } from "@/ui/Input"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/ui/Table"
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from "@/ui/Dialog"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/ui/Select"
import { Label } from "@/ui/Label"
import { Textarea } from "@/ui/Textarea"
import { useForm } from "react-hook-form"
import { zodResolver } from "@hookform/resolvers/zod"
import { z } from "zod"
import { Plus, Edit, Trash2, Search, Loader2, AlertTriangle, Download, SearchX, DollarSign, MessageCircle, Star, History, FileText } from "lucide-react"
import { Badge } from "@/ui/Badge"
import PageHeader from "@/components/PageHeader"
import EmptyState from "@/components/EmptyState"
import { formatCurrency, formatDate, exportCSV } from "@/lib/utils"
import { invoke } from "@/lib/tauri"

interface Client {
  id: number
  code: string | null
  nom: string
  adresse: string | null
  telephone: string | null
  email: string | null
  ice: string | null
  credit_plafond: number | null
  credit_actuel: number | null
  points_fidelite: number | null
}

interface MouvementFidelite {
  id: number
  client_id: number
  vente_id: number | null
  points: number
  mtype: "gain" | "depense"
  date: string
  numero_facture: string | null
}

interface ReleveClient {
  client_id: number
  nom: string
  credit_actuel: number
  credit_plafond: number
  ventes: { id: number; date: string; numero_facture: string; montant_total: number; mode_paiement: string; statut: string }[]
  paiements: { id: number; date: string; montant: number; type_paiement: string; reference: string | null }[]
}

const clientSchema = z.object({
  code: z.string().optional().nullable(),
  nom: z.string().min(1, "Nom requis"),
  adresse: z.string().optional().nullable(),
  telephone: z.string().optional().nullable(),
  email: z.string().email("Email invalide").optional().nullable(),
  ice: z.string().optional().nullable(),
  credit_plafond: z.number().min(0).optional().nullable(),
})

type ClientForm = z.infer<typeof clientSchema>

export default function Clients() {
  const [search, setSearch] = useState("")
  const [editingClient, setEditingClient] = useState<Client | null>(null)
  const [showForm, setShowForm] = useState(false)
  const [paymentClient, setPaymentClient] = useState<Client | null>(null)
  const [paymentAmount, setPaymentAmount] = useState("")
  const [paymentType, setPaymentType] = useState("especes")
  const [paymentRef, setPaymentRef] = useState("")

  const { data: clients, isLoading } = useClientsList()

  const createMutation = useCreateClient()
  const updateMutation = useUpdateClient()
  const deleteMutation = useDeleteClient()
  const paiementMutation = useAddPaiement()

  const sendWhatsAppReminder = (client: Client) => {
    if (!client.telephone) return
    const phone = client.telephone.replace(/\s+/g, "").replace(/^0/, "212")
    const amount = formatCurrency(client.credit_actuel || 0)
    const text = encodeURIComponent(`Bonjour ${client.nom},\n\nSauf erreur de notre part, votre compte chez nous présente un solde débiteur de ${amount}.\n\nMerci de bien vouloir régulariser cette situation à votre prochaine visite.\n\nCordialement.`)
    window.open(`https://wa.me/${phone}?text=${text}`, "_blank")
  }

  const [deleteConfirm, setDeleteConfirm] = useState<Client | null>(null)
  const [releveClient, setReleveClient] = useState<Client | null>(null)
  const [releveData, setReleveData] = useState<ReleveClient | null>(null)
  const [releveLoading, setReleveLoading] = useState(false)
  const [loyaltyClient, setLoyaltyClient] = useState<Client | null>(null)
  const [loyaltyHistory, setLoyaltyHistory] = useState<MouvementFidelite[]>([])
  const [loyaltyLoading, setLoyaltyLoading] = useState(false)

  const openReleve = async (client: Client) => {
    setReleveClient(client)
    setReleveLoading(true)
    try {
      const data = await invoke<ReleveClient>("get_releve_client", { clientId: client.id })
      setReleveData(data)
    } catch {
      setReleveData(null)
    } finally {
      setReleveLoading(false)
    }
  }

  const exportReleveCSV = () => {
    if (!releveData || !releveClient) return
    const header = "Type,Date,Référence,Montant,Mode"
    const venteRows = releveData.ventes.map(v =>
      `"Vente","${v.date}","${v.numero_facture}","${v.montant_total}","${v.mode_paiement}"`
    )
    const paiementRows = releveData.paiements.map(p =>
      `"Paiement","${p.date}","${p.reference || ""}","${p.montant}","${p.type_paiement}"`
    )
    const csv = [header, ...venteRows, ...paiementRows].join("\n")
    const blob = new Blob(["﻿" + csv], { type: "text/csv;charset=utf-8" })
    const url = URL.createObjectURL(blob)
    const a = document.createElement("a")
    a.href = url
    a.download = `releve_${releveClient.nom.replace(/\s+/g, "_")}_${new Date().toISOString().slice(0, 10)}.csv`
    a.click()
    URL.revokeObjectURL(url)
  }

  const openLoyaltyHistory = async (client: Client) => {
    setLoyaltyClient(client)
    setLoyaltyLoading(true)
    try {
      const data = await invoke<MouvementFidelite[]>("get_mouvements_fidelite", { clientId: client.id })
      setLoyaltyHistory(data)
    } catch {
      setLoyaltyHistory([])
    } finally {
      setLoyaltyLoading(false)
    }
  }

  const form = useForm<ClientForm>({
    resolver: zodResolver(clientSchema),
    defaultValues: { code: "", nom: "", adresse: "", telephone: "", email: "", ice: "", credit_plafond: 0 },
  })

  const handleSubmit = (data: ClientForm) => {
    if (editingClient) updateMutation.mutate({ id: editingClient.id, ...data }, { onSuccess: () => setEditingClient(null) })
    else createMutation.mutate(data as any, { onSuccess: () => setShowForm(false) })
  }

  const openEdit = (client: Client) => {
    setEditingClient(client)
    form.reset({
      code: client.code,
      nom: client.nom,
      adresse: client.adresse,
      telephone: client.telephone,
      email: client.email,
      ice: client.ice,
      credit_plafond: client.credit_plafond,
    })
    setShowForm(true)
  }

  const openCreate = () => {
    setEditingClient(null)
    form.reset({ code: "", nom: "", adresse: "", telephone: "", email: "", ice: "", credit_plafond: 0 })
    setShowForm(true)
  }

  if (isLoading) {
    return (
      <div className="space-y-6">
        <PageHeader title="Clients" description="Gestion de la clientèle" />
        <Card>
          <CardContent className="pt-6">
            <div className="flex gap-4 mb-4">
              <div className="h-10 w-64 bg-muted rounded animate-pulse" />
            </div>
            <div className="space-y-3">
              {[1,2,3,4,5].map((i) => (
                <div key={i} className="h-12 bg-muted rounded animate-pulse" />
              ))}
            </div>
          </CardContent>
        </Card>
      </div>
    )
  }

  return (
    <div className="space-y-6">
      <PageHeader title="Clients" description="Gestion de la clientèle">
        <div className="flex gap-2">
          <Button variant="outline" onClick={() => {
            if (!clients) return
            const headers = ["Code", "Nom", "Téléphone", "Email", "ICE", "Adresse", "Points fidélité", "Plafond crédit", "Crédit actuel"]
            const rows = clients.map((c) => [
              c.code || "", c.nom, c.telephone || "", c.email || "", c.ice || "",
              c.adresse || "", String(c.points_fidelite ?? 0),
              c.credit_plafond ? formatCurrency(c.credit_plafond) : "",
              c.credit_actuel ? formatCurrency(c.credit_actuel) : "",
            ])
            exportCSV(headers, rows, "clients.csv")
          }}>
            <Download className="h-4 w-4 mr-2" />
            Exporter
          </Button>
          <Button onClick={openCreate}>
            <Plus className="h-4 w-4 mr-2" />
            Nouveau client
          </Button>
        </div>
      </PageHeader>

      <Card>
        <CardContent className="pt-6">
          <div className="flex gap-4 mb-4">
            <div className="relative flex-1 max-w-md">
              <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
              <Input
                placeholder="Rechercher un client..."
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
                  <TableHead>Code</TableHead>
                  <TableHead>Nom</TableHead>
                  <TableHead>Téléphone</TableHead>
                  <TableHead>ICE</TableHead>
                  <TableHead>Email</TableHead>
                  <TableHead className="text-right">Points fidélité</TableHead>
                  <TableHead className="text-right">Plafond crédit</TableHead>
                  <TableHead className="text-right">Crédit actuel</TableHead>
                  <TableHead className="w-[120px]">Actions</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {clients?.filter((c) =>
                  c.nom.toLowerCase().includes(search.toLowerCase()) ||
                  c.code?.toLowerCase().includes(search.toLowerCase()) ||
                  c.telephone?.includes(search)
                ).map((client) => (
                  <TableRow key={client.id}>
                    <TableCell className="font-mono text-sm">{client.code || "—"}</TableCell>
                    <TableCell className="font-medium">{client.nom}</TableCell>
                    <TableCell>{client.telephone || "—"}</TableCell>
                    <TableCell className="font-mono text-xs">{client.ice || "—"}</TableCell>
                    <TableCell>{client.email || "—"}</TableCell>
                    <TableCell className="text-right">
                      {client.points_fidelite && client.points_fidelite > 0 ? (
                        <Badge variant="secondary" className="gap-1 cursor-pointer" onClick={() => openLoyaltyHistory(client)}>
                          <Star className="h-3 w-3" />
                          {client.points_fidelite}
                        </Badge>
                      ) : "—"}
                    </TableCell>
                    <TableCell className="text-right">{client.credit_plafond ? formatCurrency(client.credit_plafond) : "—"}</TableCell>
                    <TableCell className="text-right">
                      <span className={client.credit_actuel && client.credit_actuel > 0 ? "text-destructive font-medium" : ""}>
                        {client.credit_actuel ? formatCurrency(client.credit_actuel) : "—"}
                      </span>
                    </TableCell>
                    <TableCell>
                      <div className="flex items-center gap-1">
                        <Button variant="ghost" size="icon" onClick={() => openEdit(client)}>
                          <Edit className="h-4 w-4" />
                        </Button>
                        <Button variant="ghost" size="icon" onClick={() => openReleve(client)} title="Relevé de compte">
                          <FileText className="h-4 w-4 text-blue-500" />
                        </Button>
                        {client.points_fidelite != null && client.points_fidelite > 0 && (
                          <Button variant="ghost" size="icon" onClick={() => openLoyaltyHistory(client)} title="Historique fidélité">
                            <History className="h-4 w-4 text-amber-500" />
                          </Button>
                        )}
                        {client.credit_actuel && client.credit_actuel > 0 && (
                          <>
                            <Button variant="ghost" size="icon" onClick={() => { setPaymentClient(client); setPaymentAmount(""); setPaymentType("especes"); setPaymentRef("") }} title="Encaisser">
                              <DollarSign className="h-4 w-4 text-success" />
                            </Button>
                            {client.telephone && (
                              <Button variant="ghost" size="icon" onClick={() => sendWhatsAppReminder(client)} title="Relance WhatsApp">
                                <MessageCircle className="h-4 w-4 text-[#25D366]" />
                              </Button>
                            )}
                          </>
                        )}
                        <Button variant="ghost" size="icon" onClick={() => setDeleteConfirm(client)}>
                          <Trash2 className="h-4 w-4 text-destructive" />
                        </Button>
                      </div>
                    </TableCell>
                  </TableRow>
                ))}
                {!clients?.length && (
                  <TableRow>
                    <TableCell colSpan={9}>
                      <EmptyState
                        icon={<SearchX className="h-12 w-12" />}
                        title="Aucun client trouvé"
                        description="Commencez par ajouter un client"
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
        <DialogContent className="max-w-lg">
          <DialogHeader>
            <DialogTitle>{editingClient ? "Modifier le client" : "Nouveau client"}</DialogTitle>
          </DialogHeader>
          <form onSubmit={form.handleSubmit(handleSubmit)} className="space-y-4">
            <div className="grid gap-4 md:grid-cols-2">
              <div className="space-y-2">
                <Label htmlFor="code">Code</Label>
                <Input {...form.register("code")} id="code" placeholder="Auto-généré si vide" />
              </div>
              <div className="space-y-2">
                <Label htmlFor="nom">Nom *</Label>
                <Input {...form.register("nom")} id="nom" placeholder="Nom complet" />
                {form.formState.errors.nom && (
                  <p className="text-sm text-destructive">{form.formState.errors.nom.message}</p>
                )}
              </div>
              <div className="space-y-2 md:col-span-2">
                <Label htmlFor="adresse">Adresse</Label>
                <Textarea {...form.register("adresse")} id="adresse" rows={2} placeholder="Optionnel" />
              </div>
              <div className="space-y-2">
                <Label htmlFor="telephone">Téléphone</Label>
                <Input {...form.register("telephone")} id="telephone" placeholder="Optionnel" />
              </div>
              <div className="space-y-2">
                <Label htmlFor="email">Email</Label>
                <Input type="email" {...form.register("email")} id="email" placeholder="Optionnel" />
                {form.formState.errors.email && (
                  <p className="text-sm text-destructive">{form.formState.errors.email.message}</p>
                )}
              </div>
              <div className="space-y-2">
                <Label htmlFor="ice">ICE (B2B)</Label>
                <Input {...form.register("ice")} id="ice" placeholder="Numéro ICE (15 chiffres)" />
              </div>
              <div className="space-y-2">
                <Label htmlFor="credit_plafond">Plafond crédit</Label>
                <Input
                  type="number"
                  step="0.01"
                  min="0"
                  {...form.register("credit_plafond", { valueAsNumber: true })}
                  id="credit_plafond"
                />
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
      <Dialog open={!!paymentClient} onOpenChange={() => setPaymentClient(null)}>
        <DialogContent className="max-w-sm">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <DollarSign className="h-5 w-5 text-success" />
              Encaisser un paiement
            </DialogTitle>
          </DialogHeader>
          {paymentClient && (
            <div className="space-y-4">
              <p className="text-sm">
                Client: <strong>{paymentClient.nom}</strong>
                <br />
                Crédit actuel: <strong className="text-destructive">{formatCurrency(paymentClient.credit_actuel || 0)}</strong>
              </p>
              <div className="space-y-2">
                <Label htmlFor="payment-amount">Montant *</Label>
                <Input
                  id="payment-amount"
                  type="number"
                  step="0.01"
                  min="0.01"
                  max={paymentClient.credit_actuel || 0}
                  placeholder="0.00"
                  value={paymentAmount}
                  onChange={(e) => setPaymentAmount(e.target.value)}
                  autoFocus
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="payment-type">Mode de paiement</Label>
                <Select value={paymentType} onValueChange={setPaymentType}>
                  <SelectTrigger id="payment-type">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="especes">Espèces</SelectItem>
                    <SelectItem value="carte">Carte bancaire</SelectItem>
                    <SelectItem value="cheque">Chèque</SelectItem>
                    <SelectItem value="virement">Virement</SelectItem>
                  </SelectContent>
                </Select>
              </div>
              <div className="space-y-2">
                <Label htmlFor="payment-ref">Référence (optionnel)</Label>
                <Input
                  id="payment-ref"
                  placeholder="N° chèque, transaction..."
                  value={paymentRef}
                  onChange={(e) => setPaymentRef(e.target.value)}
                />
              </div>
              <DialogFooter className="gap-2">
                <Button variant="outline" onClick={() => setPaymentClient(null)}>Annuler</Button>
                <Button
                  disabled={!paymentAmount || parseFloat(paymentAmount) <= 0 || paiementMutation.isPending}
                  onClick={() => {
                    const montant = parseFloat(paymentAmount)
                    if (!montant || montant <= 0) return
                    paiementMutation.mutate(
                      { client_id: paymentClient.id, montant, ptype: paymentType, reference: paymentRef || undefined },
                      { onSuccess: () => setPaymentClient(null) }
                    )
                  }}
                >
                  {paiementMutation.isPending ? (
                    <Loader2 className="h-4 w-4 animate-spin mr-2" />
                  ) : (
                    <DollarSign className="h-4 w-4 mr-2" />
                  )}
                  Encaisser {paymentAmount ? formatCurrency(parseFloat(paymentAmount)) : ""}
                </Button>
              </DialogFooter>
            </div>
          )}
        </DialogContent>
      </Dialog>

      <Dialog open={!!deleteConfirm} onOpenChange={() => setDeleteConfirm(null)}>
        <DialogContent className="max-w-sm">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2 text-destructive">
              <AlertTriangle className="h-5 w-5" />
              Confirmer la suppression
            </DialogTitle>
          </DialogHeader>
          <p className="text-sm text-muted-foreground">
            Êtes-vous sûr de vouloir supprimer <strong>{deleteConfirm?.nom}</strong> ? Cette action est irréversible.
          </p>
          <DialogFooter className="gap-2">
            <Button variant="outline" onClick={() => setDeleteConfirm(null)}>Annuler</Button>
            <Button
              variant="destructive"
              onClick={() => {
                if (deleteConfirm) deleteMutation.mutate(deleteConfirm.id, { onSuccess: () => setDeleteConfirm(null) })
              }}
              disabled={deleteMutation.isPending}
            >
              {deleteMutation.isPending ? <Loader2 className="h-4 w-4 animate-spin mr-2" /> : <Trash2 className="h-4 w-4 mr-2" />}
              Supprimer
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog open={!!loyaltyClient} onOpenChange={() => setLoyaltyClient(null)}>
        <DialogContent className="max-w-lg">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <Star className="h-5 w-5 text-amber-500" />
              Historique fidélité — {loyaltyClient?.nom}
            </DialogTitle>
          </DialogHeader>
          <div className="space-y-3">
            <p className="text-sm text-muted-foreground">
              Solde actuel : <strong className="text-foreground">{loyaltyClient?.points_fidelite ?? 0} points</strong>
            </p>
            {loyaltyLoading ? (
              <div className="flex items-center justify-center py-8">
                <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
              </div>
            ) : loyaltyHistory.length === 0 ? (
              <p className="text-center py-8 text-sm text-muted-foreground">Aucun mouvement enregistré</p>
            ) : (
              <div className="max-h-80 overflow-y-auto">
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead>Date</TableHead>
                      <TableHead>Type</TableHead>
                      <TableHead>Facture</TableHead>
                      <TableHead className="text-right">Points</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {loyaltyHistory.map((m) => (
                      <TableRow key={m.id}>
                        <TableCell className="text-xs">{new Date(m.date).toLocaleDateString("fr-MA")}</TableCell>
                        <TableCell>
                          <Badge variant={m.mtype === "gain" ? "success" : "destructive"} className="text-xs">
                            {m.mtype === "gain" ? "Gain" : "Utilisé"}
                          </Badge>
                        </TableCell>
                        <TableCell className="font-mono text-xs">{m.numero_facture || "—"}</TableCell>
                        <TableCell className={`text-right font-medium ${m.mtype === "gain" ? "text-green-600" : "text-destructive"}`}>
                          {m.mtype === "gain" ? "+" : "−"}{m.points}
                        </TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              </div>
            )}
          </div>
        </DialogContent>
      </Dialog>
      <Dialog open={!!releveClient} onOpenChange={() => { setReleveClient(null); setReleveData(null) }}>
        <DialogContent className="max-w-2xl max-h-[85vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <FileText className="h-5 w-5" />
              Relevé de compte — {releveClient?.nom}
            </DialogTitle>
          </DialogHeader>
          {releveLoading ? (
            <div className="flex justify-center py-8">
              <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
            </div>
          ) : releveData ? (
            <div className="space-y-4">
              <div className="flex items-center justify-between">
                <div className="text-sm">
                  <span className="text-muted-foreground">Crédit actuel : </span>
                  <span className={releveData.credit_actuel > 0 ? "font-bold text-destructive" : "font-bold"}>
                    {formatCurrency(releveData.credit_actuel)}
                  </span>
                  {releveData.credit_plafond > 0 && (
                    <span className="text-muted-foreground"> / {formatCurrency(releveData.credit_plafond)}</span>
                  )}
                </div>
                <Button variant="outline" size="sm" onClick={exportReleveCSV}>
                  <Download className="h-4 w-4 mr-2" />
                  Export CSV
                </Button>
              </div>

              {releveData.ventes.length > 0 && (
                <div>
                  <p className="text-sm font-medium mb-2">Ventes ({releveData.ventes.length})</p>
                  <Table>
                    <TableHeader>
                      <TableRow>
                        <TableHead>Date</TableHead>
                        <TableHead>Facture</TableHead>
                        <TableHead>Mode</TableHead>
                        <TableHead className="text-right">Montant</TableHead>
                      </TableRow>
                    </TableHeader>
                    <TableBody>
                      {releveData.ventes.map((v) => (
                        <TableRow key={v.id}>
                          <TableCell className="text-xs">{formatDate(v.date)}</TableCell>
                          <TableCell className="font-mono text-xs">{v.numero_facture}</TableCell>
                          <TableCell className="text-xs capitalize">{v.mode_paiement}</TableCell>
                          <TableCell className="text-right font-medium">{formatCurrency(v.montant_total)}</TableCell>
                        </TableRow>
                      ))}
                    </TableBody>
                  </Table>
                </div>
              )}

              {releveData.paiements.length > 0 && (
                <div>
                  <p className="text-sm font-medium mb-2">Paiements ({releveData.paiements.length})</p>
                  <Table>
                    <TableHeader>
                      <TableRow>
                        <TableHead>Date</TableHead>
                        <TableHead>Type</TableHead>
                        <TableHead>Référence</TableHead>
                        <TableHead className="text-right">Montant</TableHead>
                      </TableRow>
                    </TableHeader>
                    <TableBody>
                      {releveData.paiements.map((p) => (
                        <TableRow key={p.id}>
                          <TableCell className="text-xs">{formatDate(p.date)}</TableCell>
                          <TableCell className="text-xs capitalize">{p.type_paiement}</TableCell>
                          <TableCell className="text-xs">{p.reference || "—"}</TableCell>
                          <TableCell className="text-right font-medium text-green-600">{formatCurrency(p.montant)}</TableCell>
                        </TableRow>
                      ))}
                    </TableBody>
                  </Table>
                </div>
              )}

              {releveData.ventes.length === 0 && releveData.paiements.length === 0 && (
                <p className="text-center py-8 text-muted-foreground">Aucune transaction enregistrée</p>
              )}
            </div>
          ) : (
            <p className="text-center py-8 text-muted-foreground">Erreur de chargement</p>
          )}
        </DialogContent>
      </Dialog>
    </div>
    )
}