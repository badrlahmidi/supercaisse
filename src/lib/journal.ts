import { invoke as tauriInvoke } from "@tauri-apps/api/core"
import { isTauriRuntime } from "@/lib/tauri"

export type NiveauJournal = "error" | "warn" | "info"

const MESSAGES_PAR_MINUTE = 30

let fenetre = 0
let compteur = 0

export function formaterErreur(erreur: unknown): string {
  if (erreur instanceof Error) {
    const pile = erreur.stack ?? ""
    return pile.includes(erreur.message) ? pile : `${erreur.name}: ${erreur.message}\n${pile}`.trim()
  }
  if (typeof erreur === "string") return erreur
  try {
    return JSON.stringify(erreur)
  } catch {
    return String(erreur)
  }
}

function autorise(maintenant: number): boolean {
  const minute = Math.floor(maintenant / 60000)
  if (minute !== fenetre) {
    fenetre = minute
    compteur = 0
  }
  compteur += 1
  return compteur <= MESSAGES_PAR_MINUTE
}

export function journaliser(niveau: NiveauJournal, message: string): void {
  if (!autorise(Date.now())) return
  if (!isTauriRuntime()) {
    console[niveau](message)
    return
  }
  tauriInvoke("journaliser_frontend", { niveau, message }).catch(() => undefined)
}

export function installerJournalGlobal(): void {
  window.addEventListener("error", (evenement) => {
    journaliser("error", `Erreur non gérée : ${formaterErreur(evenement.error ?? evenement.message)}`)
  })
  window.addEventListener("unhandledrejection", (evenement) => {
    journaliser("error", `Promesse rejetée non gérée : ${formaterErreur(evenement.reason)}`)
  })
}
