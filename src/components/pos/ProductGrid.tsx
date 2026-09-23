import { Badge } from "@/ui/Badge"
import { Package, Tags } from "lucide-react"
import { cn, formatCurrency } from "@/lib/utils"
import type { Article } from "@/types"

interface ProductGridProps {
  articles: Article[]
  onAddToCart: (article: Article) => void
}

export default function ProductGrid({ articles, onAddToCart }: ProductGridProps) {
  if (articles.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center h-full text-muted-foreground">
        <div className="p-4 rounded-full bg-muted mb-4">
          <Package className="h-12 w-12" />
        </div>
        <p className="text-lg font-medium">Aucun produit trouvé</p>
        <p className="text-sm">Essayez de modifier votre recherche ou sélectionnez une catégorie</p>
      </div>
    )
  }

  return (
    <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-3">
      {articles.slice(0, 100).map((article) => {
        const lowStock = !article.a_variantes && article.stock_alerte && article.stock <= article.stock_alerte
        const outOfStock = !article.a_variantes && article.stock <= 0
        return (
          <button
            key={article.id}
            type="button"
            className={cn(
              "relative group flex flex-col rounded-xl border bg-card p-3 text-left transition-all duration-150",
              "hover:shadow-lg hover:shadow-primary/5 hover:border-primary/30 hover:-translate-y-0.5",
              "active:scale-[0.98]",
              outOfStock
                ? "opacity-50 cursor-not-allowed border-muted"
                : lowStock
                ? "border-warning/30 hover:border-warning/50"
                : "border-border hover:border-primary/30"
            )}
            onClick={() => onAddToCart(article)}
            disabled={outOfStock}
          >
            <div className="absolute top-2 right-2">
              {article.a_variantes ? null : outOfStock ? (
                <Badge variant="destructive" className="text-[10px] px-1.5 py-0">Rupture</Badge>
              ) : lowStock ? (
                <Badge variant="warning" className="text-[10px] px-1.5 py-0">{article.stock}</Badge>
              ) : (
                <Badge variant="outline" className="text-[10px] px-1.5 py-0 text-muted-foreground">{article.stock}</Badge>
              )}
            </div>

            {article.categorie_nom && (
              <Badge variant="secondary" className="absolute top-2 left-2 text-[10px] px-1.5 py-0 max-w-[50%] truncate">
                {article.categorie_nom}
              </Badge>
            )}

            <div className="flex flex-col justify-between h-full pt-6">
              <div>
                <p className="font-semibold text-sm leading-tight line-clamp-2">{article.designation}</p>
                {article.a_variantes && (
                  <span className="inline-flex items-center gap-1 mt-1 text-[10px] text-muted-foreground">
                    <Tags className="h-3 w-3" /> Tailles/couleurs
                  </span>
                )}
              </div>
              <div className="mt-2">
                <p className="text-xl font-bold text-primary">{formatCurrency(article.prix_vente)}</p>
              </div>
            </div>

            <div className={cn(
              "absolute inset-0 rounded-xl bg-primary/5 opacity-0 transition-opacity duration-150",
              !outOfStock && "group-hover:opacity-100"
            )} />
          </button>
        )
      })}
    </div>
  )
}
