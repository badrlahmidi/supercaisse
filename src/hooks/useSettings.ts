import type { Settings } from "@/types"
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query"
import { invoke } from "@/lib/tauri"
import { toast } from "sonner"

export interface User {
  id: number
  login: string
  nom: string
  role: string
  actif: boolean
}

export type AppSettings = Settings

export function useUsersList() {
  return useQuery({
    queryKey: ["utilisateurs"],
    queryFn: () => invoke<User[]>("get_utilisateurs"),
    staleTime: 60000,
  })
}

export function useAppSettings() {
  return useQuery({
    queryKey: ["settings"],
    queryFn: () => invoke<AppSettings>("get_settings"),
    staleTime: 60000,
  })
}

export function useCreateUser() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (data: Omit<User, "id"> & { password: string }) => invoke<User>("add_utilisateur", data),
    onSuccess: () => { toast.success("Utilisateur créé"); qc.invalidateQueries({ queryKey: ["utilisateurs"] }) },
    onError: (e) => toast.error(String(e)),
  })
}

export function useUpdateUser() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ id, ...data }: Partial<User> & { id: number }) => invoke<User>("update_utilisateur", { id, ...data }),
    onSuccess: () => { toast.success("Utilisateur mis à jour"); qc.invalidateQueries({ queryKey: ["utilisateurs"] }) },
    onError: (e) => toast.error(String(e)),
  })
}

export function useDeleteUser() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id: number) => invoke("delete_utilisateur", { id }),
    onSuccess: () => { toast.success("Utilisateur supprimé"); qc.invalidateQueries({ queryKey: ["utilisateurs"] }) },
    onError: (e) => toast.error(String(e)),
  })
}

export function useUpdateSettings() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (data: Partial<AppSettings>) => invoke("update_settings", data),
    onSuccess: () => { toast.success("Paramètres enregistrés"); qc.invalidateQueries({ queryKey: ["settings"] }) },
    onError: (e) => toast.error(String(e)),
  })
}
