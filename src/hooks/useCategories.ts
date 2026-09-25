import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query"
import { invoke } from "@/lib/tauri"
import { toast } from "sonner"
import type { Category } from "@/types"

export type { Category }

export function useCategoriesList() {
  return useQuery({
    queryKey: ["categories"],
    queryFn: () => invoke<Category[]>("get_categories"),
    staleTime: 60000,
  })
}

export function useCreateCategory() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (data: Omit<Category, "id">) => invoke<Category>("add_category", data),
    onSuccess: () => { toast.success("Catégorie créée"); qc.invalidateQueries({ queryKey: ["categories"] }) },
    onError: (e) => toast.error(String(e)),
  })
}

export function useUpdateCategory() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ id, ...data }: Partial<Category> & { id: number }) => invoke<Category>("update_category", { id, ...data }),
    onSuccess: () => { toast.success("Catégorie mise à jour"); qc.invalidateQueries({ queryKey: ["categories"] }) },
    onError: (e) => toast.error(String(e)),
  })
}

export function useDeleteCategory() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id: number) => invoke("delete_category", { id }),
    onSuccess: () => { toast.success("Catégorie supprimée"); qc.invalidateQueries({ queryKey: ["categories"] }) },
    onError: (e) => toast.error(String(e)),
  })
}
