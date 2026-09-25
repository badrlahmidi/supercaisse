import { Button } from "@/ui/Button"
import { ChevronLeft, ChevronRight } from "lucide-react"

interface PaginationProps {
  page: number
  parPage: number
  total: number
  onPageChange: (page: number) => void
}

export default function Pagination({ page, parPage, total, onPageChange }: PaginationProps) {
  if (total === 0) return null
  const pages = Math.max(1, Math.ceil(total / parPage))
  const debut = page * parPage + 1
  const fin = Math.min(total, (page + 1) * parPage)
  return (
    <nav className="flex items-center justify-between gap-4 pt-4" aria-label="Pagination">
      <p className="text-sm text-muted-foreground">
        {debut}–{fin} sur {total}
      </p>
      <div className="flex items-center gap-2">
        <Button variant="outline" size="sm" disabled={page <= 0} onClick={() => onPageChange(page - 1)} aria-label="Page précédente">
          <ChevronLeft className="h-4 w-4" />
        </Button>
        <span className="text-sm tabular-nums">
          Page {page + 1} / {pages}
        </span>
        <Button variant="outline" size="sm" disabled={page >= pages - 1} onClick={() => onPageChange(page + 1)} aria-label="Page suivante">
          <ChevronRight className="h-4 w-4" />
        </Button>
      </div>
    </nav>
  )
}
