import { invoke as tauriInvoke } from "@tauri-apps/api/core"

const COMMANDES_PUBLIQUES = new Set(["login", "login_pin"])
const ERREUR_SESSION = "Session invalide ou expirée"

let sessionToken: string | null = null
const sessionExpiredListeners = new Set<() => void>()

export function setSessionToken(token: string | null) {
  sessionToken = token
}

export function getSessionToken(): string | null {
  return sessionToken
}

export function onSessionExpired(listener: () => void): () => void {
  sessionExpiredListeners.add(listener)
  return () => {
    sessionExpiredListeners.delete(listener)
  }
}

export function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window
}

export function toCamelCase(key: string): string {
  return key.replace(/_([a-z0-9])/g, (_, c: string) => c.toUpperCase())
}

export function camelizeArgs(args?: Record<string, unknown>): Record<string, unknown> | undefined {
  if (!args) return args
  return Object.fromEntries(Object.entries(args).map(([k, v]) => [toCamelCase(k), v]))
}

function withToken(cmd: string, args?: Record<string, unknown>): Record<string, unknown> | undefined {
  if (COMMANDES_PUBLIQUES.has(cmd)) return args
  return { ...args, token: sessionToken ?? "" }
}

export async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (isTauriRuntime()) {
    try {
      return await tauriInvoke<T>(cmd, camelizeArgs(withToken(cmd, args)))
    } catch (err) {
      if (String(err).startsWith(ERREUR_SESSION)) {
        sessionToken = null
        sessionExpiredListeners.forEach((listener) => listener())
      }
      throw err
    }
  }
  if (import.meta.env.DEV) {
    const { mockInvoke } = await import("./tauri.mock")
    return mockInvoke<T>(cmd, args)
  }
  throw new Error(`Application hors Tauri : commande "${cmd}" indisponible`)
}
