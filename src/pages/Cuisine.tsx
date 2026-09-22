import { useState } from "react"
import { Card, CardContent } from "@/ui/Card"
import { Button } from "@/ui/Button"
import { Badge } from "@/ui/Badge"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/ui/Tabs"
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from "@/ui/Dialog"
import { Input } from "@/ui/Input"
import { Label } from "@/ui/Label"
import { ChefHat, Clock, CheckCircle, Users, Receipt, HandCoins, Plus, Minus } from "lucide-react"
import PageHeader from "@/components/PageHeader"
import { formatCurrency } from "@/lib/utils"

interface OrderItem {
  id: number
  designation: string
  quantite: number
  notes: string | null
  statut: "en_attente" | "en_preparation" | "pret" | "servi"
  horodatage: string
}

const demoOrders: OrderItem[] = [
  { id: 1, designation: "Tajine Poulet", quantite: 2, notes: "Sans oignons", statut: "en_attente", horodatage: "12:30" },
  { id: 2, designation: "Couscous Royal", quantite: 1, notes: null, statut: "en_preparation", horodatage: "12:25" },
  { id: 3, designation: "Pastilla", quantite: 3, notes: "Extra cannelle", statut: "pret", horodatage: "12:15" },
  { id: 4, designation: "Salade Marocaine", quantite: 2, notes: null, statut: "en_preparation", horodatage: "12:28" },
  { id: 5, designation: "Harira", quantite: 4, notes: null, statut: "en_attente", horodatage: "12:32" },
  { id: 6, designation: "Briouate", quantite: 6, notes: "Amandes", statut: "pret", horodatage: "12:10" },
]

export default function Cuisine() {
  const [orders, setOrders] = useState<OrderItem[]>(demoOrders)
  const [showSplitBill, setShowSplitBill] = useState(false)
  const [showPourboire, setShowPourboire] = useState(false)
  const [splitCount, setSplitCount] = useState(2)
  const [pourboireAmount, setPourboireAmount] = useState("")

  const demoTotal = 850

  const updateStatus = (id: number, newStatut: OrderItem["statut"]) => {
    setOrders((prev) => prev.map((o) => o.id === id ? { ...o, statut: newStatut } : o))
  }

  const getStatusColor = (statut: OrderItem["statut"]) => {
    switch (statut) {
      case "en_attente": return "bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200"
      case "en_preparation": return "bg-amber-100 text-amber-800 dark:bg-amber-900 dark:text-amber-200"
      case "pret": return "bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200"
      case "servi": return "bg-gray-100 text-gray-800 dark:bg-gray-800 dark:text-gray-200"
    }
  }

  const getStatusLabel = (statut: OrderItem["statut"]) => {
    switch (statut) {
      case "en_attente": return "En attente"
      case "en_preparation": return "En préparation"
      case "pret": return "Prêt"
      case "servi": return "Servi"
    }
  }

  const getNextStatus = (statut: OrderItem["statut"]): OrderItem["statut"] | null => {
    switch (statut) {
      case "en_attente": return "en_preparation"
      case "en_preparation": return "pret"
      case "pret": return "servi"
      default: return null
    }
  }

  const grouped = {
    en_attente: orders.filter((o) => o.statut === "en_attente"),
    en_preparation: orders.filter((o) => o.statut === "en_preparation"),
    pret: orders.filter((o) => o.statut === "pret"),
  }

  return (
    <div className="space-y-6">
      <PageHeader title="Cuisine & Service" description="Écran cuisine (KDS), split bill et pourboire">
        <div className="flex gap-2">
          <Button variant="outline" onClick={() => setShowSplitBill(true)}>
            <Users className="h-4 w-4 mr-2" />
            Split bill
          </Button>
          <Button variant="outline" onClick={() => setShowPourboire(true)}>
            <HandCoins className="h-4 w-4 mr-2" />
            Pourboire
          </Button>
        </div>
      </PageHeader>

      <Tabs defaultValue="kds">
        <TabsList>
          <TabsTrigger value="kds">
            <ChefHat className="h-4 w-4 mr-2" />
            Écran Cuisine (KDS)
          </TabsTrigger>
          <TabsTrigger value="split">
            <Receipt className="h-4 w-4 mr-2" />
            Split Bill & Pourboire
          </TabsTrigger>
        </TabsList>

        <TabsContent value="kds" className="mt-4">
          <div className="grid gap-4 md:grid-cols-3">
            <div className="space-y-3">
              <div className="flex items-center gap-2 mb-2">
                <Clock className="h-5 w-5 text-red-500" />
                <h3 className="font-semibold">En attente ({grouped.en_attente.length})</h3>
              </div>
              {grouped.en_attente.map((order) => (
                <Card key={order.id} className="border-red-200 dark:border-red-800">
                  <CardContent className="pt-4 pb-3">
                    <div className="flex justify-between items-start mb-2">
                      <div>
                        <span className="font-medium">{order.designation}</span>
                        <span className="text-muted-foreground ml-2">×{order.quantite}</span>
                      </div>
                      <Badge className={getStatusColor(order.statut)}>{getStatusLabel(order.statut)}</Badge>
                    </div>
                    {order.notes && <p className="text-sm text-amber-600 dark:text-amber-400 mb-2">📝 {order.notes}</p>}
                    <div className="flex justify-between items-center">
                      <span className="text-xs text-muted-foreground">{order.horodatage}</span>
                      <Button size="sm" onClick={() => updateStatus(order.id, "en_preparation")}>Préparer</Button>
                    </div>
                  </CardContent>
                </Card>
              ))}
              {!grouped.en_attente.length && (
                <Card className="border-dashed">
                  <CardContent className="pt-6 pb-6 text-center text-muted-foreground">
                    Aucune commande en attente
                  </CardContent>
                </Card>
              )}
            </div>

            <div className="space-y-3">
              <div className="flex items-center gap-2 mb-2">
                <ChefHat className="h-5 w-5 text-amber-500" />
                <h3 className="font-semibold">En préparation ({grouped.en_preparation.length})</h3>
              </div>
              {grouped.en_preparation.map((order) => (
                <Card key={order.id} className="border-amber-200 dark:border-amber-800">
                  <CardContent className="pt-4 pb-3">
                    <div className="flex justify-between items-start mb-2">
                      <div>
                        <span className="font-medium">{order.designation}</span>
                        <span className="text-muted-foreground ml-2">×{order.quantite}</span>
                      </div>
                      <Badge className={getStatusColor(order.statut)}>{getStatusLabel(order.statut)}</Badge>
                    </div>
                    {order.notes && <p className="text-sm text-amber-600 dark:text-amber-400 mb-2">📝 {order.notes}</p>}
                    <div className="flex justify-between items-center">
                      <span className="text-xs text-muted-foreground">{order.horodatage}</span>
                      <Button size="sm" className="bg-green-600 hover:bg-green-700" onClick={() => updateStatus(order.id, "pret")}>Prêt !</Button>
                    </div>
                  </CardContent>
                </Card>
              ))}
              {!grouped.en_preparation.length && (
                <Card className="border-dashed">
                  <CardContent className="pt-6 pb-6 text-center text-muted-foreground">
                    Rien en préparation
                  </CardContent>
                </Card>
              )}
            </div>

            <div className="space-y-3">
              <div className="flex items-center gap-2 mb-2">
                <CheckCircle className="h-5 w-5 text-green-500" />
                <h3 className="font-semibold">Prêts à servir ({grouped.pret.length})</h3>
              </div>
              {grouped.pret.map((order) => (
                <Card key={order.id} className="border-green-200 dark:border-green-800">
                  <CardContent className="pt-4 pb-3">
                    <div className="flex justify-between items-start mb-2">
                      <div>
                        <span className="font-medium">{order.designation}</span>
                        <span className="text-muted-foreground ml-2">×{order.quantite}</span>
                      </div>
                      <Badge className={getStatusColor(order.statut)}>{getStatusLabel(order.statut)}</Badge>
                    </div>
                    {order.notes && <p className="text-sm text-amber-600 dark:text-amber-400 mb-2">📝 {order.notes}</p>}
                    <div className="flex justify-between items-center">
                      <span className="text-xs text-muted-foreground">{order.horodatage}</span>
                      <Button size="sm" variant="outline" onClick={() => updateStatus(order.id, "servi")}>Servi</Button>
                    </div>
                  </CardContent>
                </Card>
              ))}
              {!grouped.pret.length && (
                <Card className="border-dashed">
                  <CardContent className="pt-6 pb-6 text-center text-muted-foreground">
                    Aucun plat prêt
                  </CardContent>
                </Card>
              )}
            </div>
          </div>
        </TabsContent>

        <TabsContent value="split" className="mt-4">
          <div className="grid gap-6 md:grid-cols-2">
            <Card>
              <CardContent className="pt-6">
                <h3 className="font-semibold mb-4 flex items-center gap-2">
                  <Users className="h-5 w-5" />
                  Split Bill (addition partagée)
                </h3>
                <p className="text-sm text-muted-foreground mb-4">
                  Divisez l'addition entre plusieurs convives. Chaque part peut être payée séparément avec un mode de paiement différent.
                </p>
                <div className="space-y-4">
                  <div className="flex items-center gap-4">
                    <Label>Montant total</Label>
                    <span className="font-bold text-lg">{formatCurrency(demoTotal)}</span>
                  </div>
                  <div className="flex items-center gap-4">
                    <Label>Nombre de parts</Label>
                    <div className="flex items-center gap-2">
                      <Button variant="outline" size="icon" onClick={() => setSplitCount(Math.max(2, splitCount - 1))}>
                        <Minus className="h-4 w-4" />
                      </Button>
                      <span className="text-xl font-bold w-8 text-center">{splitCount}</span>
                      <Button variant="outline" size="icon" onClick={() => setSplitCount(Math.min(20, splitCount + 1))}>
                        <Plus className="h-4 w-4" />
                      </Button>
                    </div>
                  </div>
                  <div className="rounded-lg bg-muted p-4">
                    <div className="text-sm text-muted-foreground mb-1">Part par personne</div>
                    <div className="text-2xl font-bold">{formatCurrency(demoTotal / splitCount)}</div>
                  </div>
                </div>
              </CardContent>
            </Card>

            <Card>
              <CardContent className="pt-6">
                <h3 className="font-semibold mb-4 flex items-center gap-2">
                  <HandCoins className="h-5 w-5" />
                  Pourboire
                </h3>
                <p className="text-sm text-muted-foreground mb-4">
                  Ajoutez un pourboire au ticket. Le montant est enregistré séparément pour le suivi.
                </p>
                <div className="space-y-4">
                  <div className="grid grid-cols-3 gap-2">
                    {[5, 10, 15].map((pct) => (
                      <Button
                        key={pct}
                        variant="outline"
                        className="text-center"
                        onClick={() => setPourboireAmount(String(Math.round(demoTotal * pct / 100)))}
                      >
                        <div>
                          <div className="font-bold">{pct}%</div>
                          <div className="text-xs text-muted-foreground">{formatCurrency(Math.round(demoTotal * pct / 100))}</div>
                        </div>
                      </Button>
                    ))}
                  </div>
                  <div className="space-y-2">
                    <Label>Montant personnalisé</Label>
                    <Input
                      type="number"
                      step="1"
                      min="0"
                      value={pourboireAmount}
                      onChange={(e) => setPourboireAmount(e.target.value)}
                      placeholder="0"
                    />
                  </div>
                  {pourboireAmount && Number(pourboireAmount) > 0 && (
                    <div className="rounded-lg bg-muted p-4">
                      <div className="flex justify-between text-sm">
                        <span>Sous-total</span>
                        <span>{formatCurrency(demoTotal)}</span>
                      </div>
                      <div className="flex justify-between text-sm">
                        <span>Pourboire</span>
                        <span className="text-green-600">+{formatCurrency(Number(pourboireAmount))}</span>
                      </div>
                      <div className="border-t mt-2 pt-2 flex justify-between font-bold">
                        <span>Total</span>
                        <span>{formatCurrency(demoTotal + Number(pourboireAmount))}</span>
                      </div>
                    </div>
                  )}
                </div>
              </CardContent>
            </Card>
          </div>
        </TabsContent>
      </Tabs>

      <Dialog open={showSplitBill} onOpenChange={setShowSplitBill}>
        <DialogContent className="max-w-sm">
          <DialogHeader>
            <DialogTitle>Diviser l'addition</DialogTitle>
          </DialogHeader>
          <div className="space-y-4">
            <div className="flex items-center justify-between">
              <Label>Nombre de parts</Label>
              <div className="flex items-center gap-2">
                <Button variant="outline" size="icon" onClick={() => setSplitCount(Math.max(2, splitCount - 1))}>
                  <Minus className="h-4 w-4" />
                </Button>
                <span className="text-xl font-bold w-8 text-center">{splitCount}</span>
                <Button variant="outline" size="icon" onClick={() => setSplitCount(Math.min(20, splitCount + 1))}>
                  <Plus className="h-4 w-4" />
                </Button>
              </div>
            </div>
            <div className="rounded-lg bg-muted p-4 text-center">
              <div className="text-sm text-muted-foreground">Par personne</div>
              <div className="text-2xl font-bold">{formatCurrency(demoTotal / splitCount)}</div>
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setShowSplitBill(false)}>Fermer</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog open={showPourboire} onOpenChange={setShowPourboire}>
        <DialogContent className="max-w-sm">
          <DialogHeader>
            <DialogTitle>Ajouter un pourboire</DialogTitle>
          </DialogHeader>
          <div className="space-y-4">
            <div className="grid grid-cols-3 gap-2">
              {[5, 10, 15].map((pct) => (
                <Button key={pct} variant="outline" onClick={() => setPourboireAmount(String(Math.round(demoTotal * pct / 100)))}>
                  {pct}% ({formatCurrency(Math.round(demoTotal * pct / 100))})
                </Button>
              ))}
            </div>
            <div className="space-y-2">
              <Label>Montant libre</Label>
              <Input type="number" value={pourboireAmount} onChange={(e) => setPourboireAmount(e.target.value)} placeholder="0" />
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setShowPourboire(false)}>Fermer</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  )
}
