import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query"
import { invoke } from "@/lib/tauri"
import { toast } from "sonner"

export interface SessionCaisse {
  id: number
  caissier_id: number
  date_ouverture: string
  fond_initial: number
  statut: string
}

export function useCurrentSession(caissierId?: number) {
  return useQuery({
    queryKey: ["session", caissierId],
    queryFn: async () => {
      if (!caissierId) return null
      return invoke<SessionCaisse | null>("get_current_session")
    },
    enabled: !!caissierId,
  })
}

export function useOpenSession() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ fondInitial, magasinId }: { caissierId: number, fondInitial: number, magasinId?: number }) =>
      invoke<number>("open_session", { fondInitial, magasinId }),
    onSuccess: (_, variables) => {
      toast.success("Session de caisse ouverte")
      qc.invalidateQueries({ queryKey: ["session", variables.caissierId] })
    },
    onError: (e) => toast.error("Erreur à l'ouverture", { description: String(e) }),
  })
}

export function useCloseSession() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ sessionId, totalEspecesDeclare }: { sessionId: number, totalEspecesDeclare: number }) =>
      invoke("close_session", { sessionId, totalEspecesDeclare }),
    onSuccess: () => {
      toast.success("Session clôturée avec succès (Z)")
      qc.invalidateQueries({ queryKey: ["session"] })
    },
    onError: (e) => toast.error("Erreur à la clôture", { description: String(e) }),
  })
}
