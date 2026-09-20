import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query"
import { invoke } from "@/lib/tauri"
import { toast } from "sonner"

export interface Product {
  id: number
  code_barre: string | null
  designation: string
  description?: string
  prix_achat: number
  prix_vente: number
  tva: number
  stock: number
  stock_alerte: number | null
  categorie_id: number | null
  categorie_nom?: string
  fournisseur_id?: number | null
  fournisseur_nom?: string
  actif: boolean
  suivi_lot?: boolean
}

export function useProductsList(search?: string) {
  return useQuery({
    queryKey: ["articles", search ?? "all"],
    queryFn: () => invoke<Product[]>("get_articles", { recherche: search || null }),
    staleTime: search ? 0 : 30000,
  })
}

export function usePOSProducts(search?: string, categoryId?: number | "all") {
  return useQuery({
    queryKey: ["articles", "search", search, categoryId],
    queryFn: () => invoke<Product[]>("get_articles", { recherche: search || null }),
    staleTime: 30000,
  })
}

export function useCreateProduct() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (data: Omit<Product, "id" | "stock" | "actif" | "categorie_nom" | "fournisseur_nom">) =>
      invoke<Product>("add_article", data),
    onSuccess: () => {
      toast.success("Article créé")
      qc.invalidateQueries({ queryKey: ["articles"] })
    },
    onError: (e) => toast.error("Erreur", { description: String(e) }),
  })
}

export function useUpdateProduct() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ id, ...data }: Partial<Product> & { id: number }) =>
      invoke<Product>("update_article", { id, ...data, actif: true }),
    onSuccess: () => {
      toast.success("Article mis à jour")
      qc.invalidateQueries({ queryKey: ["articles"] })
    },
    onError: (e) => toast.error("Erreur", { description: String(e) }),
  })
}

export function useDeleteProduct() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id: number) => invoke("delete_article", { id }),
    onSuccess: () => {
      toast.success("Article supprimé")
      qc.invalidateQueries({ queryKey: ["articles"] })
    },
    onError: (e) => toast.error("Erreur", { description: String(e) }),
  })
}
