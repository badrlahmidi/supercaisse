import { Button } from "@/ui/Button"
import { Badge } from "@/ui/Badge"
import { Package, Tag } from "lucide-react"
import type { Category } from "@/types"

interface CategoryPillsProps {
  categories: Category[]
  articles: { categorie_id: number | null; id: number }[]
  activeCategory: number | "all"
  onCategoryChange: (cat: number | "all") => void
}

export default function CategoryPills({ categories, articles, activeCategory, onCategoryChange }: CategoryPillsProps) {
  return (
    <div className="flex gap-2">
      <Button
        variant={activeCategory === "all" ? "default" : "secondary"}
        size="sm"
        onClick={() => onCategoryChange("all")}
        className="whitespace-nowrap rounded-full"
      >
        <Package className="h-4 w-4 mr-1.5" />
        Tous
        <Badge variant="secondary" className="ml-1.5 text-xs px-1.5">{articles.length}</Badge>
      </Button>
      {categories.map((cat) => {
        const count = articles.filter((a) => a.categorie_id === cat.id).length
        return (
          <Button
            key={cat.id}
            variant={activeCategory === cat.id ? "default" : "secondary"}
            size="sm"
            onClick={() => onCategoryChange(cat.id)}
            className="whitespace-nowrap rounded-full"
          >
            <Tag className="h-4 w-4 mr-1.5" />
            {cat.nom}
            <Badge variant="secondary" className="ml-1.5 text-xs px-1.5">{count}</Badge>
          </Button>
        )
      })}
    </div>
  )
}
