import { useQuery } from "@tanstack/react-query"
import { invoke } from "@/lib/tauri"

export interface JournalEntry {
  id: number
  date: string
  type: string
  libelle: string
  montant: number
  mode_paiement?: string
  vente_id?: number
}

export function useJournalCaisse(dateDebut?: string, dateFin?: string) {
  return useQuery({
    queryKey: ["journal", dateDebut, dateFin],
    queryFn: () => invoke<JournalEntry[]>("get_journal_caisse", { debut: dateDebut || null, fin: dateFin || null }),
    staleTime: 30000,
  })
}
