import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query"
import { invoke } from "@/lib/tauri"
import { toast } from "sonner"

export interface Paiement {
  id: number
  client_id: number
  client_nom: string
  montant: number
  type: string
  reference: string | null
  date: string
}

export function usePaiements(clientId?: number | null) {
  return useQuery({
    queryKey: ["paiements", clientId ?? "all"],
    queryFn: () => invoke<Paiement[]>("get_paiements", { clientId: clientId ?? null }),
  })
}

export function useAddPaiement() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ clientId, montant, ptype, reference }: {
      clientId: number
      montant: number
      ptype: string
      reference?: string
    }) => invoke<number>("add_paiement", { clientId, montant, ptype, reference: reference ?? null }),
    onSuccess: () => {
      toast.success("Paiement enregistré")
      qc.invalidateQueries({ queryKey: ["paiements"] })
      qc.invalidateQueries({ queryKey: ["clients"] })
    },
    onError: (e) => toast.error("Erreur", { description: String(e) }),
  })
}
