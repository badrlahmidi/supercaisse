import type { ListeVentes } from "@/types/generated/ListeVentes"
import { useQuery } from "@tanstack/react-query"
import { invoke } from "@/lib/tauri"
import type { StatsTableauDeBord } from "@/types/generated/StatsTableauDeBord"

export type DashboardStats = StatsTableauDeBord

export function useDashboardStats() {
  return useQuery({
    queryKey: ["stats"],
    queryFn: () => invoke<DashboardStats>("get_stats"),
    staleTime: 30000,
  })
}

export function useRecentSales() {
  return useQuery({
    queryKey: ["ventes", "recent"],
    queryFn: () => invoke<ListeVentes>("get_ventes", { debut: null, fin: null, recherche: null, page: 0, parPage: 5 }),
    staleTime: 30000,
  })
}
