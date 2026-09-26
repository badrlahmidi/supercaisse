import { keepPreviousData, useQuery, useMutation, useQueryClient } from "@tanstack/react-query"
import { invoke } from "@/lib/tauri"
import { toast } from "sonner"
import type { LigneVenteSaisie } from "@/types/generated/LigneVenteSaisie"
import type { ModePaiement } from "@/types/generated/ModePaiement"
import type { PaiementSaisi } from "@/types/generated/PaiementSaisi"
import type { TypeDocument } from "@/types/generated/TypeDocument"
import type { VenteCreee } from "@/types/generated/VenteCreee"
import type { VenteResume } from "@/types/generated/VenteResume"
import type { ListeVentes } from "@/types/generated/ListeVentes"

export const VENTES_PAR_PAGE = 50

export interface CreateSaleInput {
  clientId: number | null
  articles: LigneVenteSaisie[]
  remiseGlobalePct?: number
  modePaiement: ModePaiement
  splits?: PaiementSaisi[]
  dtype: TypeDocument
  pointsUtilises?: number
  magasinId?: number | null
}

export function useSalesList(dateDebut: string | undefined, dateFin: string | undefined, recherche: string, page: number) {
  return useQuery({
    queryKey: ["ventes", dateDebut, dateFin, recherche, page],
    queryFn: () => invoke<ListeVentes>("get_ventes", {
      debut: dateDebut || null,
      fin: dateFin || null,
      recherche: recherche || null,
      page,
      parPage: VENTES_PAR_PAGE,
    }),
    placeholderData: keepPreviousData,
    staleTime: 30000,
  })
}

export async function toutesLesVentes(dateDebut: string | undefined, dateFin: string | undefined, recherche: string) {
  const lignes: VenteResume[] = []
  for (let page = 0; ; page++) {
    const r = await invoke<ListeVentes>("get_ventes", {
      debut: dateDebut || null,
      fin: dateFin || null,
      recherche: recherche || null,
      page,
      parPage: 500,
    })
    lignes.push(...r.lignes)
    if (r.lignes.length === 0 || lignes.length >= r.total) return lignes
  }
}

export function useCreateSale() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (data: CreateSaleInput) => invoke<VenteCreee>("create_vente", { ...data }),
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
    mutationFn: ({ venteId, motif }: { venteId: number; motif?: string }) =>
      invoke("annuler_vente", { venteId, motif: motif?.trim() || null }),
    onSuccess: () => {
      toast.success("Vente annulée", { description: "Stock, crédit client et points fidélité réajustés" })
      qc.invalidateQueries({ queryKey: ["clients"] })
      qc.invalidateQueries({ queryKey: ["ventes"] })
      qc.invalidateQueries({ queryKey: ["stats"] })
      qc.invalidateQueries({ queryKey: ["articles"] })
    },
    onError: (e) => toast.error("Erreur d'annulation", { description: String(e) }),
  })
}
