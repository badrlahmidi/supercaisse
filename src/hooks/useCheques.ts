import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query"
import { invoke } from "@/lib/tauri"
import { toast } from "sonner"

export interface Cheque {
  id: number
  numero: string
  banque: string
  tireur: string | null
  montant: number
  date_emission: string
  date_echeance: string
  statut: string
  ctype: string
  client_id: number | null
  fournisseur_id: number | null
  client_nom: string | null
  fournisseur_nom: string | null
}

export function useCheques() {
  return useQuery({
    queryKey: ["cheques"],
    queryFn: () => invoke<Cheque[]>("get_cheques"),
    staleTime: 30000,
  })
}

export function useAddCheque() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (data: Omit<Cheque, "id" | "client_nom" | "fournisseur_nom" | "statut">) => invoke("add_cheque", data),
    onSuccess: () => {
      toast.success("Chèque ajouté")
      qc.invalidateQueries({ queryKey: ["cheques"] })
    },
    onError: (e) => toast.error("Erreur", { description: String(e) }),
  })
}

export function useUpdateChequeStatus() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ chequeId, statut }: { chequeId: number, statut: string }) => invoke("update_cheque_status", { chequeId, statut }),
    onSuccess: () => {
      toast.success("Statut mis à jour")
      qc.invalidateQueries({ queryKey: ["cheques"] })
    },
    onError: (e) => toast.error("Erreur", { description: String(e) }),
  })
}
