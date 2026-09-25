import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query"
import { invoke } from "@/lib/tauri"
import { toast } from "sonner"
import type { Sale, SaleLine } from "@/types"

export type { Sale, SaleLine }

export interface CreateSaleInput {
  clientId: number | null
  articles: Array<{ article_id: number; variante_id?: number | null; quantite: number; remise_ligne?: number; prix_type?: "public" | "grossiste"; note?: string | null }>
  remiseGlobalePct?: number
  modePaiement: string
  splits?: Array<{ mode: string; montant: number }>
  dtype: string
  pointsUtilises?: number
  magasinId?: number | null
}

export function useSalesList(dateDebut?: string, dateFin?: string) {
  return useQuery({
    queryKey: ["ventes", dateDebut, dateFin],
    queryFn: () => invoke<Sale[]>("get_ventes", { debut: dateDebut || null, fin: dateFin || null }),
    staleTime: 30000,
  })
}

export function useRecentSales() {
  return useQuery({
    queryKey: ["ventes", "recent"],
    queryFn: () => invoke<Sale[]>("get_ventes", { debut: null, fin: null }),
    staleTime: 30000,
  })
}

export function useCreateSale() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (data: CreateSaleInput) => invoke<{ id: number; numero_facture: string }>("create_vente", data),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["ventes"] })
      qc.invalidateQueries({ queryKey: ["stats"] })
      qc.invalidateQueries({ queryKey: ["articles"] })
    },
    onError: (e) => toast.error("Erreur lors de la création de la vente", { description: String(e) }),
  })
}

export function useCancelSale() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (venteId: number) => invoke("annuler_vente", { venteId }),
    onSuccess: () => {
      toast.success("Vente annulée", { description: "Le stock a été réajusté" })
      qc.invalidateQueries({ queryKey: ["ventes"] })
      qc.invalidateQueries({ queryKey: ["stats"] })
      qc.invalidateQueries({ queryKey: ["articles"] })
    },
    onError: (e) => toast.error("Erreur d'annulation", { description: String(e) }),
  })
}
