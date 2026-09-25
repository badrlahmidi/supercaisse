import { useState, useEffect, useCallback, useRef } from "react"
import { useQuery } from "@tanstack/react-query"
import { useAuth, type SessionUser } from "@/context/AuthContext"
import { useAppSettings } from "@/hooks/useSettings"
import { invoke } from "@/lib/tauri"
import { Input } from "@/ui/Input"
import { Button } from "@/ui/Button"
import { Lock, Delete, Check } from "lucide-react"

interface ComptePin {
  login: string
  nom: string
}

const PIN_LONGUEUR_MIN = 4
const PIN_LONGUEUR_MAX = 6

export default function IdleLock({ children }: { children: React.ReactNode }) {
  const { user, loginAs } = useAuth()
  const { data: settings } = useAppSettings()
  const [locked, setLocked] = useState(false)
  const [password, setPassword] = useState("")
  const [pin, setPin] = useState("")
  const [error, setError] = useState("")
  const [mode, setMode] = useState<"password" | "pin">("pin")
  const [compte, setCompte] = useState("")
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  const { data: comptes = [] } = useQuery({
    queryKey: ["comptes-pin"],
    queryFn: () => invoke<ComptePin[]>("get_comptes_pin"),
    enabled: locked,
    meta: { erreurGeree: true },
  })
  const loginPin = compte || user?.login || ""
  const choix = user && !comptes.some((c) => c.login === user.login)
    ? [{ login: user.login, nom: user.nom }, ...comptes]
    : comptes

  const deverrouiller = useCallback(() => {
    setLocked(false)
    setPassword("")
    setPin("")
    setError("")
    setCompte("")
  }, [])

  const timeout = parseInt(settings?.idle_timeout || "300", 10) * 1000

  const resetTimer = useCallback(() => {
    if (locked || !user || timeout <= 0) return
    if (timerRef.current) clearTimeout(timerRef.current)
    timerRef.current = setTimeout(() => setLocked(true), timeout)
  }, [locked, user, timeout])

  useEffect(() => {
    if (!user || timeout <= 0) return
    const events = ["mousedown", "keydown", "touchstart", "scroll"] as const
    events.forEach(e => window.addEventListener(e, resetTimer))
    resetTimer()
    return () => {
      events.forEach(e => window.removeEventListener(e, resetTimer))
      if (timerRef.current) clearTimeout(timerRef.current)
    }
  }, [resetTimer, user, timeout])

  const handleUnlockPassword = async () => {
    if (!user) return
    try {
      const result = await invoke<SessionUser | null>("login", { login: user.login, password })
      if (result) {
        await loginAs(result)
        deverrouiller()
        resetTimer()
      } else {
        setError("Mot de passe incorrect")
      }
    } catch (err) {
      setError(String(err))
    }
  }

  const validerPin = async () => {
    if (pin.length < PIN_LONGUEUR_MIN || !loginPin) return
    try {
      const result = await invoke<SessionUser | null>("login_pin", { login: loginPin, pin })
      if (result) {
        await loginAs(result)
        deverrouiller()
        resetTimer()
      } else {
        setError("PIN incorrect")
        setPin("")
      }
    } catch (err) {
      setError(String(err))
      setPin("")
    }
  }

  const handlePinDigit = (digit: string) => {
    setError("")
    setPin((actuel) => (actuel.length < PIN_LONGUEUR_MAX ? actuel + digit : actuel))
  }

  if (!locked || !user) return <>{children}</>

  return (
    <>
      {children}
      <div className="fixed inset-0 z-[100] flex items-center justify-center bg-background/95 backdrop-blur-sm">
        <div className="w-full max-w-sm space-y-6 rounded-xl border bg-card p-8 shadow-2xl">
          <div className="flex flex-col items-center gap-3">
            <div className="flex h-16 w-16 items-center justify-center rounded-full bg-primary/10">
              <Lock className="h-8 w-8 text-primary" />
            </div>
            <h2 className="text-xl font-bold">Session verrouillée</h2>
            <p className="text-sm text-muted-foreground text-center">
              Connecté en tant que <span className="font-medium">{user.nom}</span>
            </p>
          </div>

          <div className="flex gap-2 justify-center">
            <Button
              variant={mode === "pin" ? "default" : "outline"}
              size="sm"
              onClick={() => { setMode("pin"); setError(""); setPassword(""); setPin("") }}
            >
              PIN rapide
            </Button>
            <Button
              variant={mode === "password" ? "default" : "outline"}
              size="sm"
              onClick={() => { setMode("password"); setError(""); setPassword(""); setPin("") }}
            >
              Mot de passe
            </Button>
          </div>

          {mode === "password" ? (
            <form
              onSubmit={(e) => { e.preventDefault(); handleUnlockPassword() }}
              className="space-y-4"
            >
              <Input
                type="password"
                placeholder="Mot de passe"
                value={password}
                onChange={(e) => { setPassword(e.target.value); setError("") }}
                autoFocus
              />
              {error && <p className="text-sm text-destructive text-center">{error}</p>}
              <Button type="submit" className="w-full">Déverrouiller</Button>
            </form>
          ) : (
            <div className="space-y-4">
              {choix.length > 1 && (
                <select
                  aria-label="Compte"
                  value={loginPin}
                  onChange={(e) => { setCompte(e.target.value); setPin(""); setError("") }}
                  className="h-10 w-full rounded-lg border border-border bg-background px-3 text-sm"
                >
                  {choix.map((c) => (
                    <option key={c.login} value={c.login}>{c.nom}</option>
                  ))}
                </select>
              )}
              <div className="flex justify-center gap-2">
                {Array.from({ length: PIN_LONGUEUR_MAX }, (_, i) => i).map((i) => (
                  <div
                    key={i}
                    className={`h-4 w-4 rounded-full border-2 transition-colors ${
                      i < pin.length ? "bg-primary border-primary" : "border-muted-foreground/30"
                    }`}
                  />
                ))}
              </div>
              {error && <p className="text-sm text-destructive text-center">{error}</p>}
              <p className="text-xs text-muted-foreground text-center">PIN de {PIN_LONGUEUR_MIN} à {PIN_LONGUEUR_MAX} chiffres, puis valider</p>
              <div className="grid grid-cols-3 gap-2">
                {["1", "2", "3", "4", "5", "6", "7", "8", "9"].map((d) => (
                  <Button
                    key={d}
                    variant="outline"
                    className="h-14 text-xl font-bold"
                    onClick={() => handlePinDigit(d)}
                  >
                    {d}
                  </Button>
                ))}
                <Button
                  variant="ghost"
                  className="h-14"
                  aria-label="Effacer"
                  onClick={() => { setPin(""); setError("") }}
                >
                  <Delete className="h-5 w-5" />
                </Button>
                <Button
                  variant="outline"
                  className="h-14 text-xl font-bold"
                  onClick={() => handlePinDigit("0")}
                >
                  0
                </Button>
                <Button
                  className="h-14"
                  aria-label="Valider le PIN"
                  disabled={pin.length < PIN_LONGUEUR_MIN}
                  onClick={validerPin}
                >
                  <Check className="h-5 w-5" />
                </Button>
              </div>
            </div>
          )}
        </div>
      </div>
    </>
  )
}
