import { useState } from "react"
import { useFournisseursList, useCreateFournisseur, useUpdateFournisseur, useDeleteFournisseur } from "@/hooks/useFournisseurs"
import { Card, CardContent } from "@/ui/Card"
import { Button } from "@/ui/Button"
import { Input } from "@/ui/Input"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/ui/Table"
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from "@/ui/Dialog"
import { Label } from "@/ui/Label"
import { useForm } from "react-hook-form"
import { zodResolver } from "@hookform/resolvers/zod"
import { z } from "zod"
import { Plus, Edit, Trash2, Search, Loader2, AlertTriangle, SearchX } from "lucide-react"
import PageHeader from "@/components/PageHeader"
import EmptyState from "@/components/EmptyState"

interface Fournisseur {
  id: number
  nom: string
  adresse: string | null
  telephone: string | null
  ice: string | null
  email: string | null
}

const fournisseurSchema = z.object({
  nom: z.string().min(1, "Nom requis"),
  adresse: z.string().optional().nullable(),
  telephone: z.string().optional().nullable(),
  ice: z.string().optional().nullable(),
  email: z.string().email("Email invalide").optional().nullable(),
})

type FournisseurForm = z.infer<typeof fournisseurSchema>

export default function Fournisseurs() {
  const [search, setSearch] = useState("")
  const [editingFournisseur, setEditingFournisseur] = useState<Fournisseur | null>(null)
  const [showForm, setShowForm] = useState(false)
  const [deleteConfirm, setDeleteConfirm] = useState<Fournisseur | null>(null)

  const { data: fournisseurs, isLoading } = useFournisseursList()

  const createMutation = useCreateFournisseur()

  const updateMutation = useUpdateFournisseur()

  const deleteMutation = useDeleteFournisseur()

  const form = useForm<FournisseurForm>({
    resolver: zodResolver(fournisseurSchema),
    defaultValues: { nom: "", adresse: "", telephone: "", ice: "", email: "" },
  })

  const handleSubmit = (data: FournisseurForm) => {
    if (editingFournisseur) updateMutation.mutate({ id: editingFournisseur.id, ...data }, { onSuccess: () => setEditingFournisseur(null) })
    else createMutation.mutate(data, { onSuccess: () => setShowForm(false) })
  }

  const openEdit = (fournisseur: Fournisseur) => {
    setEditingFournisseur(fournisseur)
    form.reset({
      nom: fournisseur.nom,
      adresse: fournisseur.adresse,
      telephone: fournisseur.telephone,
      ice: fournisseur.ice,
      email: fournisseur.email,
    })
    setShowForm(true)
  }

  const openCreate = () => {
    setEditingFournisseur(null)
    form.reset({ nom: "", adresse: "", telephone: "", ice: "", email: "" })
    setShowForm(true)
  }

  if (isLoading) {
    return (
      <div className="space-y-6">
        <PageHeader title="Fournisseurs" description="Gestion des fournisseurs" />
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
      <PageHeader title="Fournisseurs" description="Gestion des fournisseurs">
        <Button onClick={openCreate}>
          <Plus className="h-4 w-4 mr-2" />
          Nouveau fournisseur
        </Button>
      </PageHeader>

      <Card>
        <CardContent className="pt-6">
          <div className="flex gap-4 mb-4">
            <div className="relative flex-1 max-w-md">
              <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
              <Input
                placeholder="Rechercher un fournisseur..."
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
                  <TableHead>Nom</TableHead>
                  <TableHead>ICE</TableHead>
                  <TableHead>Téléphone</TableHead>
                  <TableHead>Email</TableHead>
                  <TableHead>Adresse</TableHead>
                  <TableHead className="w-[100px]">Actions</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {fournisseurs?.filter((f) =>
                  f.nom.toLowerCase().includes(search.toLowerCase()) ||
                  f.ice?.toLowerCase().includes(search.toLowerCase()) ||
                  f.telephone?.includes(search)
                ).map((fournisseur) => (
                  <TableRow key={fournisseur.id}>
                    <TableCell className="font-medium">{fournisseur.nom}</TableCell>
                    <TableCell>{fournisseur.ice || "—"}</TableCell>
                    <TableCell>{fournisseur.telephone || "—"}</TableCell>
                    <TableCell>{fournisseur.email || "—"}</TableCell>
                    <TableCell>{fournisseur.adresse || "—"}</TableCell>
                    <TableCell>
                      <div className="flex items-center gap-1">
                        <Button variant="ghost" size="icon" onClick={() => openEdit(fournisseur)}>
                          <Edit className="h-4 w-4" />
                        </Button>
                        <Button variant="ghost" size="icon" onClick={() => setDeleteConfirm(fournisseur)}>
                          <Trash2 className="h-4 w-4 text-destructive" />
                        </Button>
                      </div>
                    </TableCell>
                  </TableRow>
                ))}
                {!fournisseurs?.length && (
                  <TableRow>
                    <TableCell colSpan={6}>
                      <EmptyState icon={<SearchX className="h-12 w-12" />} title="Aucun fournisseur trouvé" description="Commencez par ajouter un fournisseur" />
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
            <DialogTitle>{editingFournisseur ? "Modifier le fournisseur" : "Nouveau fournisseur"}</DialogTitle>
          </DialogHeader>
          <form onSubmit={form.handleSubmit(handleSubmit)} className="space-y-4">
            <div className="grid gap-4 md:grid-cols-2">
              <div className="space-y-2">
                <Label htmlFor="nom">Nom *</Label>
                <Input {...form.register("nom")} id="nom" placeholder="Nom du fournisseur" />
                {form.formState.errors.nom && (
                  <p className="text-sm text-destructive">{form.formState.errors.nom.message}</p>
                )}
              </div>
              <div className="space-y-2">
                <Label htmlFor="ice">ICE</Label>
                <Input {...form.register("ice")} id="ice" placeholder="Identifiant Commun de l'Entreprise" />
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
              <div className="space-y-2 md:col-span-2">
                <Label htmlFor="adresse">Adresse</Label>
                <Input {...form.register("adresse")} id="adresse" placeholder="Optionnel" />
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
    </div>
    )
}