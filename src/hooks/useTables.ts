import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query"
import { invoke } from "@/lib/tauri"
import type { TableResto } from "@/types"

export type { TableResto }

export function useTables() {
  return useQuery({
    queryKey: ["tables_resto"],
    queryFn: () => invoke<TableResto[]>("get_tables"),
  })
}

export function useUpdateTable() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (data: { id: number; statut: string; ticket_id: string | null }) =>
      invoke("update_table_status", data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["tables_resto"] })
    },
  })
}
