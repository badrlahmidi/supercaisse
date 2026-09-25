import { useQuery } from "@tanstack/react-query"
import { invoke } from "@/lib/tauri"

export interface JournalEntry {
  id: number
  date: string
  utilisateur_id: number | null
  jtype: string
  montant: number
  description: string | null
  user_nom: string | null
}

export function useJournalCaisse(dateDebut?: string, dateFin?: string) {
  return useQuery({
    queryKey: ["journal", dateDebut, dateFin],
    queryFn: () => invoke<JournalEntry[]>("get_journal_caisse", { debut: dateDebut || null, fin: dateFin || null }),
    staleTime: 30000,
  })
}
