import { useMutation, useQueryClient } from "@tanstack/react-query"
import { invoke } from "@/lib/tauri"
import { toast } from "sonner"

export function useAdjustStock() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ article_id, quantite }: { article_id: number; quantite: number }) =>
      invoke("update_article_stock", { article_id, quantite }),
    onSuccess: () => {
      toast.success("Stock mis à jour")
      qc.invalidateQueries({ queryKey: ["articles"] })
    },
    onError: (e) => toast.error("Erreur", { description: String(e) }),
  })
}
