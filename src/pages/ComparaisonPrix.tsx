import { useState, useMemo } from "react"
import { useQuery } from "@tanstack/react-query"
import { invoke } from "@/lib/tauri"
import { Card, CardContent } from "@/ui/Card"
import { Button } from "@/ui/Button"
import { Input } from "@/ui/Input"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/ui/Table"
import { Loader2, Download, Search, Scale } from "lucide-react"
import { formatCurrency, exportCSV } from "@/lib/utils"
import PageHeader from "@/components/PageHeader"
import EmptyState from "@/components/EmptyState"
import type { ComparaisonArticle } from "@/types/generated/ComparaisonArticle"

type ArticleComparaison = ComparaisonArticle

export default function ComparaisonPrix() {
  const [search, setSearch] = useState("")

  const { data: articles, isLoading } = useQuery({
    queryKey: ["compare_fournisseur_prices"],
    queryFn: () => invoke<ArticleComparaison[]>("compare_fournisseur_prices", {}),
  })

  const filtered = useMemo(() => {
    if (!articles) return []
    if (!search.trim()) return articles.filter((a) => a.fournisseurs.length > 0)
    const q = search.toLowerCase()
    return articles
      .filter((a) => a.fournisseurs.length > 0)
      .filter(
        (a) =>
          a.designation.toLowerCase().includes(q) ||
          (a.code_barre && a.code_barre.toLowerCase().includes(q))
      )
  }, [articles, search])

  const multiSupplierCount = useMemo(
    () => filtered.filter((a) => a.fournisseurs.length > 1).length,
    [filtered]
  )

  const allFournisseurs = useMemo(() => {
    const names = new Set<string>()
    filtered.forEach((a) => a.fournisseurs.forEach((f) => names.add(f.fournisseur_nom)))
    return Array.from(names).sort()
  }, [filtered])

  const handleExportCSV = () => {
    if (!filtered.length) return
    const headers = [
      "Designation",
      "Code barre",
      ...allFournisseurs,
      "Ecart %",
    ]
    const rows = filtered.map((article) => {
      const prices = allFournisseurs.map((fname) => {
        const f = article.fournisseurs.find((fp) => fp.fournisseur_nom === fname)
        return f ? String(f.prix_unitaire) : ""
      })
      const prixValues = article.fournisseurs.map((f) => f.prix_unitaire)
      const min = Math.min(...prixValues)
      const max = Math.max(...prixValues)
      const ecart = min > 0 && prixValues.length > 1 ? (((max - min) / min) * 100).toFixed(1) : ""
      return [article.designation, article.code_barre || "", ...prices, ecart]
    })
    exportCSV(headers, rows, `comparaison_prix_${new Date().toISOString().slice(0, 10)}.csv`)
  }

  return (
    <div className="space-y-6">
      <PageHeader
        title="Comparaison prix fournisseurs"
        description="Comparez les prix d'achat par article entre les differents fournisseurs"
      >
        <Button variant="outline" onClick={handleExportCSV} disabled={!filtered.length}>
          <Download className="h-4 w-4 mr-2" />
          Exporter CSV
        </Button>
      </PageHeader>

      <div className="grid gap-4 md:grid-cols-3">
        <Card>
          <CardContent className="pt-6">
            <p className="text-sm text-muted-foreground">Articles compares</p>
            <p className="text-2xl font-bold">{filtered.length}</p>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-6">
            <p className="text-sm text-muted-foreground">Fournisseurs</p>
            <p className="text-2xl font-bold">{allFournisseurs.length}</p>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-6">
            <p className="text-sm text-muted-foreground">Multi-fournisseurs</p>
            <p className="text-2xl font-bold">{multiSupplierCount}</p>
          </CardContent>
        </Card>
      </div>

      <Card>
        <CardContent className="pt-6 space-y-4">
          <div className="max-w-sm">
            <Input
              placeholder="Rechercher un article..."
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              leftIcon={<Search className="h-4 w-4" />}
            />
          </div>

          <div className="overflow-x-auto">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Designation</TableHead>
                  <TableHead>Code barre</TableHead>
                  {allFournisseurs.map((name) => (
                    <TableHead key={name} className="text-right">
                      {name}
                    </TableHead>
                  ))}
                  <TableHead className="text-right">Ecart</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {isLoading ? (
                  <TableRow>
                    <TableCell colSpan={3 + allFournisseurs.length} className="text-center py-8">
                      <Loader2 className="h-8 w-8 animate-spin mx-auto" />
                    </TableCell>
                  </TableRow>
                ) : filtered.length ? (
                  filtered.map((article) => {
                    const prixValues = article.fournisseurs.map((f) => f.prix_unitaire)
                    const min = Math.min(...prixValues)
                    const max = Math.max(...prixValues)
                    const ecart =
                      min > 0 && prixValues.length > 1
                        ? ((max - min) / min) * 100
                        : 0

                    return (
                      <TableRow key={article.article_id}>
                        <TableCell className="font-medium">{article.designation}</TableCell>
                        <TableCell className="text-muted-foreground">
                          {article.code_barre || "—"}
                        </TableCell>
                        {allFournisseurs.map((fname) => {
                          const f = article.fournisseurs.find(
                            (fp) => fp.fournisseur_nom === fname
                          )
                          if (!f) {
                            return (
                              <TableCell key={fname} className="text-right text-muted-foreground">
                                —
                              </TableCell>
                            )
                          }
                          const isCheapest = f.prix_unitaire === min && prixValues.length > 1
                          const isMostExpensive = f.prix_unitaire === max && prixValues.length > 1 && max !== min
                          return (
                            <TableCell
                              key={fname}
                              className={`text-right font-medium ${
                                isCheapest
                                  ? "text-green-600 dark:text-green-400"
                                  : isMostExpensive
                                    ? "text-red-600 dark:text-red-400"
                                    : ""
                              }`}
                            >
                              {formatCurrency(f.prix_unitaire)}
                            </TableCell>
                          )
                        })}
                        <TableCell className="text-right">
                          {prixValues.length > 1 && ecart > 0 ? (
                            <span
                              className={
                                ecart > 20
                                  ? "text-red-600 dark:text-red-400 font-medium"
                                  : ecart > 10
                                    ? "text-yellow-600 dark:text-yellow-400 font-medium"
                                    : "text-muted-foreground"
                              }
                            >
                              {ecart.toFixed(1)}%
                            </span>
                          ) : (
                            <span className="text-muted-foreground">—</span>
                          )}
                        </TableCell>
                      </TableRow>
                    )
                  })
                ) : (
                  <TableRow>
                    <TableCell colSpan={3 + allFournisseurs.length}>
                      <EmptyState
                        icon={<Scale className="h-12 w-12" />}
                        title="Aucune donnee de comparaison"
                        description="Les prix apparaitront ici une fois des achats enregistres aupres de plusieurs fournisseurs"
                      />
                    </TableCell>
                  </TableRow>
                )}
              </TableBody>
            </Table>
          </div>
        </CardContent>
      </Card>
    </div>
  )
}
