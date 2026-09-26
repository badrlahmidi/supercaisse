import { useState } from "react"
import { useQuery } from "@tanstack/react-query"
import { Card, CardContent } from "@/ui/Card"
import { Button } from "@/ui/Button"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/ui/Table"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/ui/Tabs"
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from "@/ui/Dialog"
import { Input } from "@/ui/Input"
import { Label } from "@/ui/Label"
import { Textarea } from "@/ui/Textarea"
import { Badge } from "@/ui/Badge"
import { Plus, Store, TrendingUp, Package, Users, MapPin, Edit, Trash2 } from "lucide-react"
import PageHeader from "@/components/PageHeader"
import { formatCurrency } from "@/lib/utils"
import { invoke } from "@/lib/tauri"
import type { StatsMagasin } from "@/types/generated/StatsMagasin"

export default function Boutiques() {
  const [showForm, setShowForm] = useState(false)
  const [editingMagasin, setEditingMagasin] = useState<StatsMagasin | null>(null)
  const [nom, setNom] = useState("")
  const [adresse, setAdresse] = useState("")

  const { data: stats } = useQuery({
    queryKey: ["stats_magasins"],
    queryFn: () => invoke<StatsMagasin[]>("get_stats_magasins"),
    staleTime: 30000,
  })

  const openEdit = (m: StatsMagasin) => {
    setEditingMagasin(m)
    setNom(m.nom)
    setAdresse(m.adresse || "")
    setShowForm(true)
  }

  const openCreate = () => {
    setEditingMagasin(null)
    setNom("")
    setAdresse("")
    setShowForm(true)
  }

  return (
    <div className="space-y-6">
      <PageHeader title="Multi-boutiques" description="Comparateur et gestion des points de vente">
        <Button onClick={openCreate}>
          <Plus className="h-4 w-4 mr-2" />
          Nouvelle boutique
        </Button>
      </PageHeader>

      <div className="grid gap-4 md:grid-cols-4">
        <Card>
          <CardContent className="pt-6">
            <div className="flex items-center gap-2">
              <Store className="h-5 w-5 text-primary" />
              <div className="text-2xl font-bold">{stats?.length ?? "—"}</div>
            </div>
            <p className="text-sm text-muted-foreground">Boutiques</p>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-6">
            <div className="flex items-center gap-2">
              <TrendingUp className="h-5 w-5 text-green-500" />
              <div className="text-2xl font-bold">{formatCurrency(stats?.reduce((s, d) => s + d.ca_mois, 0) ?? 0)}</div>
            </div>
            <p className="text-sm text-muted-foreground">CA total (mois)</p>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-6">
            <div className="flex items-center gap-2">
              <Package className="h-5 w-5 text-blue-500" />
              <div className="text-2xl font-bold">{stats?.reduce((s, d) => s + d.nb_ventes, 0) ?? "—"}</div>
            </div>
            <p className="text-sm text-muted-foreground">Ventes totales</p>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-6">
            <div className="flex items-center gap-2">
              <Users className="h-5 w-5 text-purple-500" />
              <div className="text-2xl font-bold">{stats?.reduce((s, d) => s + d.nb_clients_actifs, 0) ?? "—"}</div>
            </div>
            <p className="text-sm text-muted-foreground">Clients actifs</p>
          </CardContent>
        </Card>
      </div>

      <Tabs defaultValue="comparaison">
        <TabsList>
          <TabsTrigger value="comparaison">Comparaison</TabsTrigger>
          <TabsTrigger value="boutiques">Liste boutiques</TabsTrigger>
        </TabsList>

        <TabsContent value="comparaison" className="mt-4">
          <Card>
            <CardContent className="pt-6">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Boutique</TableHead>
                    <TableHead className="text-right">CA (mois)</TableHead>
                    <TableHead className="text-right">Ventes</TableHead>
                    <TableHead className="text-right">Clients actifs</TableHead>
                    <TableHead className="text-right">Valeur stock</TableHead>
                    <TableHead className="text-right">Panier moyen</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {(stats || []).map((stat) => (
                    <TableRow key={stat.id}>
                      <TableCell className="font-medium">
                        <div className="flex items-center gap-2">
                          <MapPin className="h-4 w-4 text-muted-foreground" />
                          {stat.nom}
                        </div>
                      </TableCell>
                      <TableCell className="text-right font-medium">{formatCurrency(stat.ca_mois)}</TableCell>
                      <TableCell className="text-right">{stat.nb_ventes}</TableCell>
                      <TableCell className="text-right">{stat.nb_clients_actifs}</TableCell>
                      <TableCell className="text-right">{formatCurrency(stat.valeur_stock)}</TableCell>
                      <TableCell className="text-right">
                        {stat.nb_ventes > 0 ? formatCurrency(stat.ca_mois / stat.nb_ventes) : "—"}
                      </TableCell>
                    </TableRow>
                  ))}
                  {!stats?.length && (
                    <TableRow>
                      <TableCell colSpan={6} className="text-center text-muted-foreground py-8">
                        Aucune boutique configurée.
                      </TableCell>
                    </TableRow>
                  )}
                </TableBody>
              </Table>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="boutiques" className="mt-4">
          <Card>
            <CardContent className="pt-6">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Nom</TableHead>
                    <TableHead>Adresse</TableHead>
                    <TableHead>Statut</TableHead>
                    <TableHead>Actions</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {(stats || []).map((m) => (
                    <TableRow key={m.id}>
                      <TableCell className="font-medium">{m.nom}</TableCell>
                      <TableCell>{m.adresse || "—"}</TableCell>
                      <TableCell><Badge>Active</Badge></TableCell>
                      <TableCell>
                        <div className="flex gap-1">
                          <Button variant="ghost" size="icon" onClick={() => openEdit(m)}>
                            <Edit className="h-4 w-4" />
                          </Button>
                          <Button variant="ghost" size="icon">
                            <Trash2 className="h-4 w-4 text-destructive" />
                          </Button>
                        </div>
                      </TableCell>
                    </TableRow>
                  ))}
                  {!stats?.length && (
                    <TableRow>
                      <TableCell colSpan={4} className="text-center text-muted-foreground py-8">
                        Aucune boutique configurée. Ajoutez votre première boutique.
                      </TableCell>
                    </TableRow>
                  )}
                </TableBody>
              </Table>
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>

      <Dialog open={showForm} onOpenChange={setShowForm}>
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle>{editingMagasin ? "Modifier la boutique" : "Nouvelle boutique"}</DialogTitle>
          </DialogHeader>
          <div className="space-y-4">
            <div className="space-y-2">
              <Label>Nom *</Label>
              <Input value={nom} onChange={(e) => setNom(e.target.value)} placeholder="Nom de la boutique" />
            </div>
            <div className="space-y-2">
              <Label>Adresse</Label>
              <Textarea value={adresse} onChange={(e) => setAdresse(e.target.value)} placeholder="Adresse complète" rows={3} />
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setShowForm(false)}>Annuler</Button>
            <Button onClick={() => setShowForm(false)}>Enregistrer</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  )
}
