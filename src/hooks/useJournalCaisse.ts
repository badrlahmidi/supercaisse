import { keepPreviousData, useQuery } from "@tanstack/react-query"
import { invoke } from "@/lib/tauri"
import type { EcritureJournal } from "@/types/generated/EcritureJournal"
import type { ListeJournal } from "@/types/generated/ListeJournal"

export type JournalEntry = EcritureJournal
export type SensJournal = "all" | "entree" | "sortie"

export const JOURNAL_PAR_PAGE = 100

interface FiltreJournal {
  dateDebut?: string
  dateFin?: string
  sens: SensJournal
  recherche: string
}

function parametres({ dateDebut, dateFin, sens, recherche }: FiltreJournal) {
  return {
    debut: dateDebut || null,
    fin: dateFin || null,
    sens: sens === "all" ? null : sens,
    recherche: recherche || null,
  }
}

export function useJournalCaisse(filtre: FiltreJournal, page: number) {
  return useQuery({
    queryKey: ["journal", filtre, page],
    queryFn: () => invoke<ListeJournal>("get_journal_caisse", { ...parametres(filtre), page, parPage: JOURNAL_PAR_PAGE }),
    placeholderData: keepPreviousData,
    staleTime: 30000,
  })
}

export async function toutLeJournal(filtre: FiltreJournal) {
  const lignes: EcritureJournal[] = []
  for (let page = 0; ; page++) {
    const r = await invoke<ListeJournal>("get_journal_caisse", { ...parametres(filtre), page, parPage: 500 })
    lignes.push(...r.lignes)
    if (r.lignes.length === 0 || lignes.length >= r.total) return lignes
  }
}
