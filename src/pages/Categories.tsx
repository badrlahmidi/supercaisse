import { useState } from "react"
import { useCategoriesList, useCreateCategory, useUpdateCategory, useDeleteCategory } from "@/hooks/useCategories"
import { Card, CardContent } from "@/ui/Card"
import { Button } from "@/ui/Button"
import { Input } from "@/ui/Input"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/ui/Table"
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from "@/ui/Dialog"
import { Label } from "@/ui/Label"
import { useForm } from "react-hook-form"
import { zodResolver } from "@hookform/resolvers/zod"
import { z } from "zod"
import PageHeader from "@/components/PageHeader"
import EmptyState from "@/components/EmptyState"
import { Plus, Edit, Trash2, Search, Loader2, AlertTriangle, SearchX } from "lucide-react"

interface Category {
  id: number
  nom: string
  description: string | null
}

const categorySchema = z.object({
  nom: z.string().min(1, "Nom requis"),
  description: z.string().optional().nullable(),
})

type CategoryForm = z.infer<typeof categorySchema>

export default function Categories() {
  const [search, setSearch] = useState("")
  const [editingCategory, setEditingCategory] = useState<Category | null>(null)
  const [showForm, setShowForm] = useState(false)
  const [deleteConfirm, setDeleteConfirm] = useState<Category | null>(null)

  const { data: categories, isLoading } = useCategoriesList()

  const createMutation = useCreateCategory()
  const updateMutation = useUpdateCategory()
  const deleteMutation = useDeleteCategory()

  const form = useForm<CategoryForm>({
    resolver: zodResolver(categorySchema),
    defaultValues: { nom: "", description: "" },
  })

  const handleSubmit = (data: CategoryForm) => {
    if (editingCategory) updateMutation.mutate({ id: editingCategory.id, ...data }, { onSuccess: () => setEditingCategory(null) })
    else createMutation.mutate(data, { onSuccess: () => setShowForm(false) })
  }

  const openEdit = (category: Category) => {
    setEditingCategory(category)
    form.reset({ nom: category.nom, description: category.description })
    setShowForm(true)
  }

  const openCreate = () => {
    setEditingCategory(null)
    form.reset({ nom: "", description: "" })
    setShowForm(true)
  }

  if (isLoading) {
    return (
      <div className="space-y-6">
        <PageHeader title="Catégories" description="Gestion des catégories d'articles" />
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
      <PageHeader title="Catégories" description="Gestion des catégories d'articles">
        <Button onClick={openCreate}>
          <Plus className="h-4 w-4 mr-2" />
          Nouvelle catégorie
        </Button>
      </PageHeader>

      <Card>
        <CardContent className="pt-6">
          <div className="flex gap-4 mb-4">
            <div className="relative flex-1 max-w-md">
              <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
              <Input
                placeholder="Rechercher une catégorie..."
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
                  <TableHead>Description</TableHead>
                  <TableHead className="w-[100px]">Actions</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {categories?.filter((c) =>
                  c.nom.toLowerCase().includes(search.toLowerCase()) ||
                  c.description?.toLowerCase().includes(search.toLowerCase())
                ).map((category) => (
                  <TableRow key={category.id}>
                    <TableCell className="font-medium">{category.nom}</TableCell>
                    <TableCell>{category.description || "—"}</TableCell>
                    <TableCell>
                      <div className="flex items-center gap-1">
                        <Button variant="ghost" size="icon" onClick={() => openEdit(category)}>
                          <Edit className="h-4 w-4" />
                        </Button>
                        <Button variant="ghost" size="icon" onClick={() => setDeleteConfirm(category)}>
                          <Trash2 className="h-4 w-4 text-destructive" />
                        </Button>
                      </div>
                    </TableCell>
                  </TableRow>
                ))}
                {!categories?.length && (
                  <TableRow>
                    <TableCell colSpan={3}>
                      <EmptyState
                        icon={<SearchX className="h-12 w-12" />}
                        title="Aucune catégorie trouvée"
                        description="Commencez par créer une catégorie"
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
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle>{editingCategory ? "Modifier la catégorie" : "Nouvelle catégorie"}</DialogTitle>
          </DialogHeader>
          <form onSubmit={form.handleSubmit(handleSubmit)} className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="nom">Nom *</Label>
              <Input {...form.register("nom")} id="nom" placeholder="Nom de la catégorie" />
              {form.formState.errors.nom && (
                <p className="text-sm text-destructive">{form.formState.errors.nom.message}</p>
              )}
            </div>
            <div className="space-y-2">
              <Label htmlFor="description">Description</Label>
              <Input {...form.register("description")} id="description" placeholder="Optionnel" />
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