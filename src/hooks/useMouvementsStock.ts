import { keepPreviousData, useQuery } from "@tanstack/react-query"
import { invoke } from "@/lib/tauri"
import type { MouvementStockLigne } from "@/types/generated/MouvementStockLigne"
import type { Page } from "@/types/generated/Page"

export type MouvementStock = MouvementStockLigne

export const MOUVEMENTS_PAR_PAGE = 100

export function useMouvementsStock(articleId: number | null, mtype: string | null, recherche: string, page: number) {
  return useQuery({
    queryKey: ["mouvements-stock", articleId, mtype, recherche, page],
    queryFn: () => invoke<Page<MouvementStockLigne>>("get_mouvements_stock", {
      articleId,
      mtype,
      recherche: recherche || null,
      debut: null,
      fin: null,
      page,
      parPage: MOUVEMENTS_PAR_PAGE,
    }),
    placeholderData: keepPreviousData,
    staleTime: 30000,
  })
}
