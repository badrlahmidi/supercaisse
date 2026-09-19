import { useQuery } from "@tanstack/react-query"
import { invoke } from "@/lib/tauri"

export interface DashboardStats {
  total_ventes_30j: number
  nb_clients: number
  nb_articles: number
  stock_alerte: number
  credit_total: number
  ca_jour: number
  ca_mois: number
  benefice_mois: number
  top_articles: Array<{ designation: string; quantite: number }>
  top_clients: Array<{ nom: string; depense: number }>
}

export interface RecentSale {
  id: number
  date: string
  client_nom?: string
  net_paye: number
  mode_paiement: string
}

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
    queryFn: () => invoke<RecentSale[]>("get_ventes", { debut: null, fin: null }),
    staleTime: 30000,
  })
}
