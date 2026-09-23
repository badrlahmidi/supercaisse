import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query"
import { invoke } from "@/lib/tauri"
import { toast } from "sonner"

export interface Sale {
  id: number
  date: string
  client_nom?: string
  caissier_nom: string
  montant_total: number
  montant_remise: number
  net_paye: number
  mode_paiement: string
  statut: string
  numero_facture?: string
  dtype: string
  articles?: SaleLine[]
}

export interface SaleLine {
  id: number
  article_id: number
  designation: string
  quantite: number
  prix_unitaire: number
  tva: number
  total_ligne: number
}

export interface CreateSaleInput {
  clientId: number | null
  caissierId: number
  articles: Array<{ article_id: number; quantite: number; prix_unitaire: number; tva: number }>
  montantRemise: number
  modePaiement: string
  dtype: string
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
