import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query"
import { invoke } from "@/lib/tauri"
import { toast } from "sonner"
import type { Client } from "@/types"

export type { Client }

export function useClientsList() {
  return useQuery({
    queryKey: ["clients"],
    queryFn: () => invoke<Client[]>("get_clients"),
    staleTime: 30000,
  })
}

export function useCreateClient() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (data: Omit<Client, "id" | "credit_actuel">) =>
      invoke<Client>("add_client", data),
    onSuccess: () => { toast.success("Client créé"); qc.invalidateQueries({ queryKey: ["clients"] }) },
    onError: (e) => toast.error(String(e)),
  })
}

export function useUpdateClient() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ id, ...data }: Partial<Client> & { id: number }) => invoke<Client>("update_client", { id, ...data }),
    onSuccess: () => { toast.success("Client mis à jour"); qc.invalidateQueries({ queryKey: ["clients"] }) },
    onError: (e) => toast.error(String(e)),
  })
}

export function useDeleteClient() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id: number) => invoke("delete_client", { id }),
    onSuccess: () => { toast.success("Client supprimé"); qc.invalidateQueries({ queryKey: ["clients"] }) },
    onError: (e) => toast.error(String(e)),
  })
}

export function useAddPaiement() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ client_id, montant, ptype, reference }: { client_id: number; montant: number; ptype: string; reference?: string }) =>
      invoke("add_paiement", { clientId: client_id, montant, ptype, reference: reference || null }),
    onSuccess: () => {
      toast.success("Paiement enregistré")
      qc.invalidateQueries({ queryKey: ["clients"] })
      qc.invalidateQueries({ queryKey: ["paiements"] })
    },
    onError: (e) => toast.error(String(e)),
  })
}
