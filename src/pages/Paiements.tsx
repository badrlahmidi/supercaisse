import { useState } from "react"
import { usePaiements, useAddPaiement } from "@/hooks/usePaiements"
import { useClientsList } from "@/hooks/useClients"
import { Card, CardContent } from "@/ui/Card"
import { Button } from "@/ui/Button"
import { Input } from "@/ui/Input"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/ui/Table"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/ui/Select"
import { Label } from "@/ui/Label"
import { useForm } from "react-hook-form"
import { zodResolver } from "@hookform/resolvers/zod"
import { z } from "zod"
import { Loader2, Plus, SearchX, Receipt } from "lucide-react"
import PageHeader from "@/components/PageHeader"
import EmptyState from "@/components/EmptyState"
import { formatCurrency } from "@/lib/utils"
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from "@/ui/Dialog"

const paiementSchema = z.object({
  client_id: z.number().min(1, "Client requis"),
  montant: z.number().min(0.01, "Montant invalide"),
  ptype: z.string().min(1, "Mode de paiement requis"),
  reference: z.string().optional(),
})

type PaiementForm = z.infer<typeof paiementSchema>

const PAYMENT_MODES = [
  { value: "especes", label: "Espèces" },
  { value: "carte", label: "Carte Bancaire" },
  { value: "cheque", label: "Chèque" },
  { value: "virement", label: "Virement" },
]

export default function Paiements() {
  const [selectedClientFilter, setSelectedClientFilter] = useState<number | null>(null)
  const [showForm, setShowForm] = useState(false)

  const { data: paiements, isLoading } = usePaiements(selectedClientFilter)
  const { data: clients } = useClientsList()
  const addMutation = useAddPaiement()

  const form = useForm<PaiementForm>({
    resolver: zodResolver(paiementSchema),
    defaultValues: {
      montant: 0,
      ptype: "especes",
      reference: "",
    },
  })

  const handleSubmit = (data: PaiementForm) => {
    addMutation.mutate({
      clientId: data.client_id,
      montant: data.montant,
      ptype: data.ptype,
      reference: data.reference,
    }, {
      onSuccess: () => {
        setShowForm(false)
        form.reset()
      }
    })
  }

  if (isLoading) {
    return (
      <div className="space-y-6">
        <PageHeader title="Paiements" description="Historique des encaissements clients" />
        <Card className="animate-pulse h-64" />
      </div>
    )
  }

  return (
    <div className="space-y-6">
      <PageHeader title="Paiements" description="Historique des encaissements clients">
        <div className="flex gap-2">
          <Select
            value={selectedClientFilter ? String(selectedClientFilter) : "all"}
            onValueChange={(v) => setSelectedClientFilter(v !== "all" ? parseInt(v) : null)}
          >
            <SelectTrigger className="w-[200px] h-9">
              <SelectValue placeholder="Tous les clients" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">Tous les clients</SelectItem>
              {clients?.map((c) => (
                <SelectItem key={c.id} value={String(c.id)}>{c.nom}</SelectItem>
              ))}
            </SelectContent>
          </Select>
          <Button onClick={() => setShowForm(true)}>
            <Plus className="h-4 w-4 mr-2" />
            Nouveau paiement
          </Button>
        </div>
      </PageHeader>

      <Card>
        <CardContent className="pt-6">
          <div className="overflow-x-auto">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Date</TableHead>
                  <TableHead>Client</TableHead>
                  <TableHead>Mode</TableHead>
                  <TableHead>Référence</TableHead>
                  <TableHead className="text-right">Montant</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {paiements?.map((p) => (
                  <TableRow key={p.id}>
                    <TableCell className="whitespace-nowrap">
                      {new Date(p.date).toLocaleString("fr-FR")}
                    </TableCell>
                    <TableCell className="font-medium">{p.client_nom}</TableCell>
                    <TableCell className="capitalize">{p.type}</TableCell>
                    <TableCell className="text-muted-foreground text-sm">{p.reference || "—"}</TableCell>
                    <TableCell className="text-right font-medium text-success">
                      +{formatCurrency(p.montant)}
                    </TableCell>
                  </TableRow>
                ))}
                {!paiements?.length && (
                  <TableRow>
                    <TableCell colSpan={5}>
                      <EmptyState
                        icon={<SearchX className="h-12 w-12" />}
                        title="Aucun paiement"
                        description="Aucun encaissement trouvé pour ce filtre."
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
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Enregistrer un paiement</DialogTitle>
          </DialogHeader>
          <form onSubmit={form.handleSubmit(handleSubmit)} className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="client_id">Client *</Label>
              <Select
                value={form.watch("client_id") ? String(form.watch("client_id")) : ""}
                onValueChange={(v) => form.setValue("client_id", parseInt(v))}
              >
                <SelectTrigger id="client_id">
                  <SelectValue placeholder="Sélectionner un client" />
                </SelectTrigger>
                <SelectContent>
                  {clients?.map((c) => (
                    <SelectItem key={c.id} value={String(c.id)}>
                      {c.nom} {c.credit_actuel > 0 ? `(Crédit: ${formatCurrency(c.credit_actuel)})` : ""}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              {form.formState.errors.client_id && (
                <p className="text-sm text-destructive">{form.formState.errors.client_id.message}</p>
              )}
            </div>

            <div className="space-y-2">
              <Label htmlFor="montant">Montant *</Label>
              <Input
                type="number"
                step="0.01"
                min="0.01"
                {...form.register("montant", { valueAsNumber: true })}
                id="montant"
              />
              {form.formState.errors.montant && (
                <p className="text-sm text-destructive">{form.formState.errors.montant.message}</p>
              )}
            </div>

            <div className="space-y-2">
              <Label htmlFor="ptype">Mode de paiement *</Label>
              <Select
                value={form.watch("ptype")}
                onValueChange={(v) => form.setValue("ptype", v)}
              >
                <SelectTrigger id="ptype">
                  <SelectValue placeholder="Mode de paiement" />
                </SelectTrigger>
                <SelectContent>
                  {PAYMENT_MODES.map((m) => (
                    <SelectItem key={m.value} value={m.value}>
                      {m.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>

            <div className="space-y-2">
              <Label htmlFor="reference">Référence (ex: Numéro chèque)</Label>
              <Input {...form.register("reference")} id="reference" placeholder="Optionnel" />
            </div>

            <DialogFooter>
              <Button type="button" variant="outline" onClick={() => setShowForm(false)}>
                Annuler
              </Button>
              <Button type="submit" disabled={addMutation.isPending}>
                {addMutation.isPending ? (
                  <>
                    <Loader2 className="h-4 w-4 animate-spin mr-2" />
                    Enregistrement...
                  </>
                ) : (
                  <>
                    <Receipt className="h-4 w-4 mr-2" />
                    Valider le paiement
                  </>
                )}
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>
    </div>
  )
}
