import { invoke as tauriInvoke } from "@tauri-apps/api/core"

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

export async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (isTauriRuntime()) {
    return tauriInvoke<T>(cmd, camelizeArgs(args))
  }
  if (import.meta.env.DEV) {
    const { mockInvoke } = await import("./tauri.mock")
    return mockInvoke<T>(cmd, args)
  }
  throw new Error(`Application hors Tauri : commande "${cmd}" indisponible`)
}
