import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query"
import { invoke } from "@/lib/tauri"
import { toast } from "sonner"
import type { AchatResume } from "@/types/generated/AchatResume"

export type Achat = AchatResume

export function useAchatsList() {
  return useQuery({
    queryKey: ["achats"],
    queryFn: () => invoke<Achat[]>("get_achats"),
    staleTime: 30000,
  })
}

export function useCreateAchat() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (data: Record<string, unknown>) => invoke<number>("create_achat", data),
    onSuccess: () => {
      toast.success("Achat créé")
      qc.invalidateQueries({ queryKey: ["achats"] })
      qc.invalidateQueries({ queryKey: ["articles"] })
    },
    onError: (e) => toast.error("Erreur", { description: String(e) }),
  })
}

export function useUpdateAchatStatus() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ achatId, statutLivraison, statutPaiement }: { achatId: number, statutLivraison: string, statutPaiement: string }) =>
      invoke("update_achat_status", { achatId, statutLivraison, statutPaiement }),
    onSuccess: () => {
      toast.success("Statut mis à jour", { description: "Le stock a été actualisé si réceptionné" })
      qc.invalidateQueries({ queryKey: ["achats"] })
      qc.invalidateQueries({ queryKey: ["articles"] })
    },
    onError: (e) => toast.error("Erreur de pointage", { description: String(e) }),
  })
}
