import { MutationCache, QueryCache, QueryClient } from "@tanstack/react-query"
import { toast } from "sonner"
import { estErreurSession } from "@/lib/tauri"

function message(erreur: unknown): string {
  return erreur instanceof Error ? erreur.message : String(erreur)
}

export function creerQueryClient(): QueryClient {
  return new QueryClient({
    queryCache: new QueryCache({
      onError: (erreur, query) => {
        if (estErreurSession(erreur) || query.meta?.erreurGeree) return
        toast.error("Chargement impossible", { id: query.queryHash, description: message(erreur) })
      },
    }),
    mutationCache: new MutationCache({
      onError: (erreur, _variables, _contexte, mutation) => {
        if (estErreurSession(erreur) || mutation.options.onError) return
        toast.error("Opération impossible", { description: message(erreur) })
      },
    }),
    defaultOptions: {
      queries: {
        staleTime: 1000 * 60 * 5,
        retry: false,
        refetchOnWindowFocus: false,
      },
    },
  })
}
