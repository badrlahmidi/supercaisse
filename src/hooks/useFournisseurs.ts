import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query"
import { invoke } from "@/lib/tauri"
import { toast } from "sonner"
import type { Fournisseur } from "@/types"

export type { Fournisseur }

export function useFournisseursList() {
  return useQuery({
    queryKey: ["fournisseurs"],
    queryFn: () => invoke<Fournisseur[]>("get_fournisseurs"),
    staleTime: 30000,
  })
}

export function useCreateFournisseur() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (data: Omit<Fournisseur, "id">) => invoke<Fournisseur>("add_fournisseur", data),
    onSuccess: () => { toast.success("Fournisseur créé"); qc.invalidateQueries({ queryKey: ["fournisseurs"] }) },
    onError: (e) => toast.error(String(e)),
  })
}

export function useUpdateFournisseur() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ id, ...data }: Partial<Fournisseur> & { id: number }) => invoke<Fournisseur>("update_fournisseur", { id, ...data }),
    onSuccess: () => { toast.success("Fournisseur mis à jour"); qc.invalidateQueries({ queryKey: ["fournisseurs"] }) },
    onError: (e) => toast.error(String(e)),
  })
}

export function useDeleteFournisseur() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id: number) => invoke("delete_fournisseur", { id }),
    onSuccess: () => { toast.success("Fournisseur supprimé"); qc.invalidateQueries({ queryKey: ["fournisseurs"] }) },
    onError: (e) => toast.error(String(e)),
  })
}
