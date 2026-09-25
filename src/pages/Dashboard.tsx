import { useDashboardStats, useRecentSales } from "@/hooks/useDashboard"
import { useAuth } from "@/context/AuthContext"
import { Card, CardContent, CardHeader, CardTitle } from "@/ui/Card"
import { Badge } from "@/ui/Badge"
import { Button } from "@/ui/Button"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/ui/Table"
import {
  LayoutDashboard,
  Package,
  AlertTriangle,
  CreditCard,
  Users,
  TrendingUp,
  ShoppingCart,
  Plus,
  BarChart2,
  Trophy,
  Star,
  Banknote,
} from "lucide-react"
import PageHeader from "@/components/PageHeader"
import EmptyState from "@/components/EmptyState"
import { cn, formatCurrency, formatNumber, formatDate } from "@/lib/utils"
import { format } from "date-fns"
import { fr } from "date-fns/locale"
import { useState, useEffect } from "react"
import { useNavigate } from "react-router-dom"
import { compteDansCA } from "@/lib/ventes"
import { sommeDH } from "@/lib/totaux"

interface Stats {
  total_ventes_30j: number
  nb_articles: number
  stock_alerte: number
  credit_total: number
  nb_clients: number
  ca_jour: number
  ca_mois: number
  benefice_mois: number
  top_articles: Array<{ designation: string; quantite: number }>
  top_clients: Array<{ nom: string; depense: number }>
}

interface Vente {
  id: number
  date: string
  client_id: number | null
  caissier_id: number | null
  montant_total: number
  montant_remise: number
  mode_paiement: string
  statut: string
  client_nom: string | null
  caissier_nom: string | null
}

const statCards = [
  { key: "ca_jour", label: "CA Aujourd'hui", icon: Banknote, color: "text-emerald-500", bg: "bg-emerald-500/10" },
  { key: "ca_mois", label: "CA ce Mois", icon: TrendingUp, color: "text-primary", bg: "bg-primary/10" },
  { key: "benefice_mois", label: "Marge brute (Mois)", icon: Star, color: "text-purple-500", bg: "bg-purple-500/10" },
  { key: "credit_total", label: "Crédit à recouvrer", icon: CreditCard, color: "text-destructive", bg: "bg-destructive/10" },
  { key: "stock_alerte", label: "Stock en alerte", icon: AlertTriangle, color: "text-warning", bg: "bg-warning/10" },
] as const

export default function Dashboard() {
  const { user } = useAuth()
  const navigate = useNavigate()

  const { data: stats, isLoading: statsLoading } = useDashboardStats()

  const { data: ventes, isLoading: ventesLoading } = useRecentSales()

  // Prepare chart data
  const [chartData, setChartData] = useState<{ label: string; value: number }[]>([])

  useEffect(() => {
    if (!ventes) return
    const days: { label: string; value: number }[] = []
    for (let i = 6; i >= 0; i--) {
      const d = new Date()
      d.setDate(d.getDate() - i)
      const dateStr = d.toISOString().slice(0, 10)
      const montants = ventes
        .filter((v) => v.date && v.date.startsWith(dateStr) && compteDansCA(v))
        .map((v) => sommeDH([v.montant_total, -v.montant_remise]))
      const total = sommeDH(montants)
      days.push({
        label: format(d, "EEE dd", { locale: fr }),
        value: total,
      })
    }
    setChartData(days)
  }, [ventes])

  if (statsLoading) {
    return (
      <div className="space-y-6">
        <PageHeader title="Tableau de bord" description={`Bienvenue, ${user?.nom}`} />
        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-5">
          {statCards.map((_, i) => (
            <Card key={i} className="animate-pulse">
              <CardContent className="pt-6">
                <div className="h-4 w-3/4 bg-muted rounded animate-pulse" />
                <div className="h-8 w-1/2 bg-muted rounded mt-2 animate-pulse" />
              </CardContent>
            </Card>
          ))}
        </div>
      </div>
    )
  }

  const maxValue = Math.max(...chartData.map((d) => d.value), 1)

  return (
    <div className="space-y-6">
      <PageHeader title="Tableau de bord" description={`Bienvenue, ${user?.nom} - Vue d'ensemble de votre activité`}>
        <div className="flex gap-2">
          <Button variant="outline" size="sm">
            <Plus className="h-4 w-4 mr-2" />
            Nouvelle vente
          </Button>
          <Button size="sm" onClick={() => navigate("/pos")}>
            <ShoppingCart className="h-4 w-4 mr-2" />
            Aller à la caisse
          </Button>
        </div>
      </PageHeader>

      {/* Stats Grid */}
      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-5">
        {statCards.map((card) => (
          <Card key={card.key}>
            <CardContent className="pt-6">
              <div className="flex items-center justify-between">
                <div>
                  <p className="text-sm font-medium text-muted-foreground">{card.label}</p>
                  <p className="text-2xl font-bold mt-1">
                    {card.key === "stock_alerte"
                      ? formatNumber(stats?.[card.key] || 0)
                      : formatCurrency(stats?.[card.key as keyof Stats] as number || 0)}
                  </p>
                </div>
                <div className={cn("p-3 rounded-xl", card.bg)}>
                  <card.icon className={cn("h-6 w-6", card.color)} />
                </div>
              </div>
              {card.key === "stock_alerte" && stats && stats.stock_alerte > 0 && (
                <Badge variant="warning" className="mt-2">
                  {stats.stock_alerte} article(s) en alerte
                </Badge>
              )}
              {card.key === "credit_total" && stats && stats.credit_total > 0 && (
                <Badge variant="destructive" className="mt-2">
                  {formatCurrency(stats.credit_total)} à recouvrer
                </Badge>
              )}
            </CardContent>
          </Card>
        ))}
      </div>

      {/* Charts & Recent Sales */}
      <div className="grid gap-6 lg:grid-cols-2">
        {/* Sales Chart */}
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <BarChart2 className="h-5 w-5" />
              Ventes des 7 derniers jours
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="h-64 flex items-end justify-around px-2">
              {chartData.map((day, index) => {
                const height = (day.value / maxValue) * 200
                return (
                  <div key={index} className="flex flex-col items-center gap-2 flex-1 max-w-[80px]">
                    <div
                      className={cn(
                        "w-full rounded-t transition-all duration-300 bg-primary",
                        day.value > 0 ? "hover:bg-primary/80" : "bg-muted"
                      )}
                      style={{ height: `${Math.max(height, day.value > 0 ? 4 : 0)}px` }}
                      title={`${day.label}: ${formatCurrency(day.value)}`}
                    />
                    <span className="text-xs text-muted-foreground text-center w-full px-1">{day.label}</span>
                    {day.value > 0 && (
                      <span className="text-xs font-medium text-primary -mt-2">{formatCurrency(day.value)}</span>
                    )}
                  </div>
                )
              })}
            </div>
          </CardContent>
        </Card>

        {/* Recent Sales */}
        <Card>
          <CardHeader className="flex flex-row items-center justify-between">
            <CardTitle className="flex items-center gap-2">
              <ShoppingCart className="h-5 w-5" />
              Dernières ventes
            </CardTitle>
            <Button variant="ghost" size="sm" onClick={() => navigate("/ventes")}>
              Voir tout
            </Button>
          </CardHeader>
          <CardContent>
            {ventesLoading ? (
              <div className="space-y-3">
                {[1, 2, 3].map((i) => (
                  <div key={i} className="h-12 animate-pulse bg-muted rounded" />
                ))}
              </div>
            ) : ventes && ventes.length > 0 ? (
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead className="w-[60px]">#</TableHead>
                    <TableHead>Client</TableHead>
                    <TableHead className="text-right">Montant</TableHead>
                    <TableHead className="w-[120px]">Paiement</TableHead>
                    <TableHead className="w-[100px]">Date</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {ventes.slice(0, 5).map((vente) => (
                    <TableRow key={vente.id}>
                      <TableCell className="font-medium">#{vente.id}</TableCell>
                      <TableCell>{vente.client_nom || "Client de passage"}</TableCell>
                      <TableCell className="text-right font-medium text-primary">
                        {formatCurrency(vente.montant_total - vente.montant_remise)}
                      </TableCell>
                      <TableCell>
                        <Badge variant="outline" className="capitalize">
                          {vente.mode_paiement}
                        </Badge>
                      </TableCell>
                      <TableCell className="text-muted-foreground">
                        {formatDate(vente.date)}
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            ) : (
              <EmptyState
                icon={<ShoppingCart className="h-12 w-12" />}
                title="Aucune vente pour le moment"
                description="Effectuez votre première vente"
                action={<Button onClick={() => navigate("/pos")}>Effectuer une vente</Button>}
              />
            )}
          </CardContent>
        </Card>
      </div>

      {/* Analytics (Top Articles & Clients) */}
      <div className="grid gap-6 lg:grid-cols-2">
        {/* Top Articles */}
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Trophy className="h-5 w-5 text-amber-500" />
              Top 5 Articles (30 jours)
            </CardTitle>
          </CardHeader>
          <CardContent>
            {statsLoading ? (
              <div className="space-y-3">
                {[1, 2, 3].map((i) => <div key={i} className="h-10 animate-pulse bg-muted rounded" />)}
              </div>
            ) : stats?.top_articles && stats.top_articles.length > 0 ? (
              <div className="space-y-4">
                {stats.top_articles.map((item, i) => (
                  <div key={i} className="flex items-center justify-between">
                    <div className="flex items-center gap-3">
                      <div className="flex h-8 w-8 items-center justify-center rounded-full bg-primary/10 text-primary font-bold text-sm">
                        {i + 1}
                      </div>
                      <p className="font-medium">{item.designation}</p>
                    </div>
                    <Badge variant="secondary">{item.quantite} vendus</Badge>
                  </div>
                ))}
              </div>
            ) : (
              <p className="text-sm text-muted-foreground text-center py-4">Pas assez de données</p>
            )}
          </CardContent>
        </Card>

        {/* Top Clients */}
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Star className="h-5 w-5 text-purple-500" />
              Top 5 Clients (30 jours)
            </CardTitle>
          </CardHeader>
          <CardContent>
            {statsLoading ? (
              <div className="space-y-3">
                {[1, 2, 3].map((i) => <div key={i} className="h-10 animate-pulse bg-muted rounded" />)}
              </div>
            ) : stats?.top_clients && stats.top_clients.length > 0 ? (
              <div className="space-y-4">
                {stats.top_clients.map((client, i) => (
                  <div key={i} className="flex items-center justify-between">
                    <div className="flex items-center gap-3">
                      <div className="flex h-8 w-8 items-center justify-center rounded-full bg-primary/10 text-primary font-bold text-sm">
                        {i + 1}
                      </div>
                      <p className="font-medium">{client.nom}</p>
                    </div>
                    <p className="font-bold text-primary">{formatCurrency(client.depense)}</p>
                  </div>
                ))}
              </div>
            ) : (
              <p className="text-sm text-muted-foreground text-center py-4">Pas assez de données</p>
            )}
          </CardContent>
        </Card>
      </div>

      {/* Quick Actions */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <LayoutDashboard className="h-5 w-5" />
            Raccourcis rapides
          </CardTitle>
        </CardHeader>
        <CardContent>
          <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
            <Button variant="outline" className="h-24 flex-col gap-2" onClick={() => navigate("/pos")}>
              <ShoppingCart className="h-8 w-8" />
              <span>Caisse POS</span>
            </Button>
            <Button variant="outline" className="h-24 flex-col gap-2" onClick={() => navigate("/articles")}>
              <Plus className="h-8 w-8" />
              <span>Ajouter article</span>
            </Button>
            <Button variant="outline" className="h-24 flex-col gap-2" onClick={() => navigate("/stock")}>
              <Package className="h-8 w-8" />
              <span>Voir le stock</span>
            </Button>
            <Button variant="outline" className="h-24 flex-col gap-2" onClick={() => navigate("/clients")}>
              <Users className="h-8 w-8" />
              <span>Gérer clients</span>
            </Button>
          </div>
        </CardContent>
      </Card>
    </div>
  )
}