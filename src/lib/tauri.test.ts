import { describe, it, expect, vi, beforeEach } from "vitest"
import { readFileSync, readdirSync, statSync } from "node:fs"
import { join, resolve } from "node:path"

const tauriInvokeMock = vi.fn()
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...args: unknown[]) => tauriInvokeMock(...args) }))

import { invoke, camelizeArgs, toCamelCase } from "./tauri"

describe("invoke", () => {
  beforeEach(() => {
    tauriInvokeMock.mockReset()
  })

  it("propagates backend errors instead of returning mock data", async () => {
    tauriInvokeMock.mockRejectedValue("Plafond de crédit dépassé")
    await expect(invoke("create_vente", { clientId: 1 })).rejects.toBe("Plafond de crédit dépassé")
  })

  it("propagates login errors instead of falling back to mock users", async () => {
    tauriInvokeMock.mockRejectedValue("database is locked")
    await expect(invoke("login", { login: "admin", password: "admin" })).rejects.toBe("database is locked")
  })

  it("uses development mocks only outside the Tauri runtime", async () => {
    const internals = Object.getOwnPropertyDescriptor(window, "__TAURI_INTERNALS__")
    Reflect.deleteProperty(window, "__TAURI_INTERNALS__")
    try {
      const categories = await invoke<unknown[]>("get_categories")
      expect(Array.isArray(categories)).toBe(true)
      expect(tauriInvokeMock).not.toHaveBeenCalled()
    } finally {
      if (internals) Object.defineProperty(window, "__TAURI_INTERNALS__", internals)
    }
  })

  it("converts top-level argument keys to camelCase", async () => {
    tauriInvokeMock.mockResolvedValue(1)
    await invoke("add_article", { prix_achat: 5, prix_vente: 10, designation: "X" })
    expect(tauriInvokeMock).toHaveBeenCalledWith("add_article", { prixAchat: 5, prixVente: 10, designation: "X" })
  })

  it("leaves nested objects untouched", async () => {
    tauriInvokeMock.mockResolvedValue({ id: 1 })
    const articles = [{ article_id: 3, prix_unitaire: 10 }]
    await invoke("create_vente", { client_id: null, articles, points_utilises: 2 })
    expect(tauriInvokeMock).toHaveBeenCalledWith("create_vente", { clientId: null, articles, pointsUtilises: 2 })
  })
})

describe("camelizeArgs", () => {
  it("keeps camelCase keys unchanged", () => {
    expect(camelizeArgs({ clientId: 1, montant: 2 })).toEqual({ clientId: 1, montant: 2 })
  })

  it("returns undefined for no args", () => {
    expect(camelizeArgs(undefined)).toBeUndefined()
  })

  it("matches Tauri's lowerCamelCase conversion", () => {
    expect(toCamelCase("date_emission")).toBe("dateEmission")
    expect(toCamelCase("fidelite_dh_pour_1_point")).toBe("fideliteDhPour1Point")
  })
})

const ROOT = resolve(__dirname, "../..")

interface RustCommand {
  required: string[]
  all: string[]
}

function splitTopLevel(input: string, sep: string, angleBrackets = true): string[] {
  const opening = angleBrackets ? "<([{" : "([{"
  const closing = angleBrackets ? ">)]}" : ")]}"
  const parts: string[] = []
  let depth = 0
  let current = ""
  for (const ch of input) {
    if (opening.includes(ch)) depth++
    if (closing.includes(ch)) depth--
    if (ch === sep && depth === 0) {
      parts.push(current)
      current = ""
    } else {
      current += ch
    }
  }
  if (current.trim()) parts.push(current)
  return parts
}

function parseRustCommands(): Map<string, RustCommand> {
  const dir = join(ROOT, "src-tauri/src/commands")
  const commands = new Map<string, RustCommand>()
  for (const file of readdirSync(dir).filter((f) => f.endsWith(".rs"))) {
    const src = readFileSync(join(dir, file), "utf8")
    const re = /#\[tauri::command\]\s*pub\s+(?:async\s+)?fn\s+(\w+)\s*\(([\s\S]*?)\)\s*->/g
    for (const m of src.matchAll(re)) {
      const required: string[] = []
      const all: string[] = []
      for (const param of splitTopLevel(m[2], ",")) {
        const [name, ...typeParts] = param.split(":")
        const type = typeParts.join(":").trim()
        if (!name.trim() || /^(State|AppHandle|Window|WebviewWindow)\b/.test(type)) continue
        const key = toCamelCase(name.trim())
        all.push(key)
        if (!type.startsWith("Option<")) required.push(key)
      }
      commands.set(m[1], { required, all })
    }
  }
  return commands
}

function listSourceFiles(dir: string): string[] {
  return readdirSync(dir).flatMap((entry) => {
    const full = join(dir, entry)
    if (statSync(full).isDirectory()) return listSourceFiles(full)
    if (!/\.(ts|tsx)$/.test(entry) || /\.test\.tsx?$/.test(entry) || entry.startsWith("tauri.")) return []
    return [full]
  })
}

function skipString(src: string, i: number): number {
  const quote = src[i]
  i++
  while (i < src.length && src[i] !== quote) {
    if (src[i] === "\\") i++
    else if (quote === "`" && src[i] === "$" && src[i + 1] === "{") i = matchClose(src, i + 1)
    i++
  }
  return i
}

function matchClose(src: string, open: number): number {
  let depth = 0
  for (let i = open; i < src.length; i++) {
    const ch = src[i]
    if (ch === '"' || ch === "'" || ch === "`") {
      i = skipString(src, i)
      continue
    }
    if (ch === "{" || ch === "(" || ch === "[") depth++
    if (ch === "}" || ch === ")" || ch === "]") {
      depth--
      if (depth === 0) return i
    }
  }
  return src.length
}

interface InvokeCall {
  location: string
  cmd: string
  keys: string[] | null
}

function findInvokeCalls(): InvokeCall[] {
  const calls: InvokeCall[] = []
  for (const file of listSourceFiles(join(ROOT, "src"))) {
    const src = readFileSync(file, "utf8")
    for (const m of src.matchAll(/\binvoke\s*(?:<[^()]*?>)?\(\s*"(\w+)"\s*([,)])/g)) {
      const location = `${file.replace(ROOT + "/", "")}:${src.slice(0, m.index).split("\n").length}`
      if (m[2] === ")") {
        calls.push({ location, cmd: m[1], keys: [] })
        continue
      }
      let i = (m.index ?? 0) + m[0].length
      while (/\s/.test(src[i])) i++
      if (src[i] !== "{") {
        calls.push({ location, cmd: m[1], keys: null })
        continue
      }
      const body = src.slice(i + 1, matchClose(src, i))
      const segments = splitTopLevel(body, ",", false).map((s) => s.trim()).filter(Boolean)
      if (segments.some((s) => s.startsWith("..."))) {
        calls.push({ location, cmd: m[1], keys: null })
        continue
      }
      const keys = segments.map((s) => s.match(/^([A-Za-z_$][\w$]*)/)?.[1] ?? "").filter(Boolean)
      calls.push({ location, cmd: m[1], keys: keys.map(toCamelCase) })
    }
  }
  return calls
}

describe("IPC contract with Rust commands", () => {
  const commands = parseRustCommands()
  const calls = findInvokeCalls()
  const registered = readFileSync(join(ROOT, "src-tauri/src/lib.rs"), "utf8")

  it("finds the Rust commands and frontend invoke calls", () => {
    expect(commands.size).toBeGreaterThan(90)
    expect(calls.length).toBeGreaterThan(50)
  })

  it("only invokes commands that exist and are registered", () => {
    const unknown = calls
      .filter((c) => !commands.has(c.cmd) || !registered.includes(`commands::${c.cmd},`))
      .map((c) => `${c.location} ${c.cmd}`)
    expect(unknown).toEqual([])
  })

  it("passes every required argument on literal invoke calls", () => {
    const missing = calls
      .filter((c) => c.keys !== null)
      .flatMap((c) => {
        const absent = commands.get(c.cmd)?.required.filter((k) => !c.keys!.includes(k)) ?? []
        return absent.length ? [`${c.location} ${c.cmd} missing ${absent.join(", ")}`] : []
      })
    expect(missing).toEqual([])
  })

  it("passes no argument unknown to the Rust command", () => {
    const extra = calls
      .filter((c) => c.keys !== null)
      .flatMap((c) => {
        const unknownKeys = c.keys!.filter((k) => !commands.get(c.cmd)?.all.includes(k))
        return unknownKeys.length ? [`${c.location} ${c.cmd} unknown ${unknownKeys.join(", ")}`] : []
      })
    expect(extra).toEqual([])
  })
})
