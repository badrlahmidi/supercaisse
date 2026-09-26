import { useQuery } from "@tanstack/react-query"
import { useNavigate } from "react-router-dom"
import { invoke } from "@/lib/tauri"
import { Card, CardContent } from "@/ui/Card"
import { Button } from "@/ui/Button"
import { Badge } from "@/ui/Badge"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/ui/Table"
import { Loader2, Download, ShoppingBag, Package } from "lucide-react"
import { formatCurrency, exportCSV } from "@/lib/utils"
import PageHeader from "@/components/PageHeader"
import EmptyState from "@/components/EmptyState"
import type { ArticleAlerte } from "@/types/generated/ArticleAlerte"

function getUrgencyBadge(stock: number, stockAlerte: number) {
  if (stock <= 0) {
    return <Badge variant="destructive">Rupture</Badge>
  }
  if (stock < stockAlerte / 2) {
    return <Badge variant="warning">Urgent</Badge>
  }
  return <Badge className="border-transparent bg-yellow-500 text-white">Bas</Badge>
}

export default function Reappro() {
  const navigate = useNavigate()

  const { data: articles, isLoading } = useQuery({
    queryKey: ["articles_stock_alerte"],
    queryFn: () => invoke<ArticleAlerte[]>("get_articles_stock_alerte"),
  })

  const totalCost = articles?.reduce(
    (sum, a) => sum + a.prix_achat * Math.max(0, a.suggestion_qte),
    0
  ) ?? 0

  const nbRupture = articles?.filter((a) => a.stock <= 0).length ?? 0
  const nbUrgent = articles?.filter((a) => a.stock > 0 && a.stock < a.stock_alerte / 2).length ?? 0

  const handleExportCSV = () => {
    if (!articles?.length) return
    const headers = [
      "Designation",
      "Categorie",
      "Fournisseur",
      "Stock actuel",
      "Seuil alerte",
      "Qte suggeree",
      "Prix achat",
      "Cout estime",
    ]
    const rows = articles.map((a) => [
      a.designation,
      a.categorie_nom || "",
      a.fournisseur_nom || "",
      String(a.stock),
      String(a.stock_alerte),
      String(Math.max(0, a.suggestion_qte)),
      String(a.prix_achat),
      String(a.prix_achat * Math.max(0, a.suggestion_qte)),
    ])
    exportCSV(headers, rows, `reappro_${new Date().toISOString().slice(0, 10)}.csv`)
  }

  return (
    <div className="space-y-6">
      <PageHeader
        title="Suggestions de reapprovisionnement"
        description="Articles sous le seuil d'alerte avec quantites de commande suggerees"
      >
        <Button variant="outline" onClick={handleExportCSV} disabled={!articles?.length}>
          <Download className="h-4 w-4 mr-2" />
          Exporter CSV
        </Button>
        <Button onClick={() => navigate("/achats")}>
          <ShoppingBag className="h-4 w-4 mr-2" />
          Commander
        </Button>
      </PageHeader>

      <div className="grid gap-4 md:grid-cols-4">
        <Card>
          <CardContent className="pt-6">
            <p className="text-sm text-muted-foreground">Articles en alerte</p>
            <p className="text-2xl font-bold">{articles?.length ?? 0}</p>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-6">
            <p className="text-sm text-muted-foreground">En rupture</p>
            <p className="text-2xl font-bold text-destructive">{nbRupture}</p>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-6">
            <p className="text-sm text-muted-foreground">Urgents</p>
            <p className="text-2xl font-bold text-warning">{nbUrgent}</p>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-6">
            <p className="text-sm text-muted-foreground">Cout total estime</p>
            <p className="text-2xl font-bold">{formatCurrency(totalCost)}</p>
          </CardContent>
        </Card>
      </div>

      <Card>
        <CardContent className="pt-6">
          <div className="overflow-x-auto">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Urgence</TableHead>
                  <TableHead>Designation</TableHead>
                  <TableHead>Categorie</TableHead>
                  <TableHead>Fournisseur</TableHead>
                  <TableHead className="text-right">Stock actuel</TableHead>
                  <TableHead className="text-right">Seuil alerte</TableHead>
                  <TableHead className="text-right">Qte suggeree</TableHead>
                  <TableHead className="text-right">Cout estime</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {isLoading ? (
                  <TableRow>
                    <TableCell colSpan={8} className="text-center py-8">
                      <Loader2 className="h-8 w-8 animate-spin mx-auto" />
                    </TableCell>
                  </TableRow>
                ) : articles?.length ? (
                  articles.map((article) => {
                    const qte = Math.max(0, article.suggestion_qte)
                    return (
                      <TableRow
                        key={article.id}
                        className={
                          article.stock <= 0
                            ? "bg-destructive/5"
                            : article.stock < article.stock_alerte / 2
                              ? "bg-warning/5"
                              : ""
                        }
                      >
                        <TableCell>{getUrgencyBadge(article.stock, article.stock_alerte)}</TableCell>
                        <TableCell className="font-medium">{article.designation}</TableCell>
                        <TableCell>{article.categorie_nom || "—"}</TableCell>
                        <TableCell>{article.fournisseur_nom || "—"}</TableCell>
                        <TableCell className="text-right font-medium">
                          <span
                            className={
                              article.stock <= 0
                                ? "text-destructive"
                                : article.stock < article.stock_alerte / 2
                                  ? "text-warning"
                                  : ""
                            }
                          >
                            {article.stock}
                          </span>
                        </TableCell>
                        <TableCell className="text-right">{article.stock_alerte}</TableCell>
                        <TableCell className="text-right font-medium">{qte}</TableCell>
                        <TableCell className="text-right font-medium">
                          {formatCurrency(article.prix_achat * qte)}
                        </TableCell>
                      </TableRow>
                    )
                  })
                ) : (
                  <TableRow>
                    <TableCell colSpan={8}>
                      <EmptyState
                        icon={<Package className="h-12 w-12" />}
                        title="Aucun article en alerte"
                        description="Tous les articles sont au-dessus de leur seuil d'alerte"
                      />
                    </TableCell>
                  </TableRow>
                )}
              </TableBody>
            </Table>
          </div>

          {articles && articles.length > 0 && (
            <div className="flex justify-end mt-4 pt-4 border-t">
              <div className="text-right">
                <p className="text-sm text-muted-foreground">Cout total estime de reapprovisionnement</p>
                <p className="text-xl font-bold">{formatCurrency(totalCost)}</p>
              </div>
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  )
}
