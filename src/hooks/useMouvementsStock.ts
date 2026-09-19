import { useQuery } from "@tanstack/react-query"
import { invoke } from "@/lib/tauri"

export interface MouvementStock {
  id: number
  date: string
  article_id: number
  designation: string
  quantite: number
  mtype: string
  reference_id: number | null
  reference_type: string | null
}

export function useMouvementsStock(articleId?: number | null, debut?: string, fin?: string) {
  return useQuery({
    queryKey: ["mouvements-stock", articleId, debut, fin],
    queryFn: () => invoke<MouvementStock[]>("get_mouvements_stock", {
      articleId: articleId || null,
      debut: debut || null,
      fin: fin || null,
    }),
    staleTime: 30000,
  })
}
