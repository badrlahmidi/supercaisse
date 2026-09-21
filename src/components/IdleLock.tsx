import { useState, useEffect, useCallback, useRef } from "react"
import { useAuth } from "@/context/AuthContext"
import { useAppSettings } from "@/hooks/useSettings"
import { invoke } from "@/lib/tauri"
import { Input } from "@/ui/Input"
import { Button } from "@/ui/Button"
import { Lock } from "lucide-react"
import { toast } from "sonner"

export default function IdleLock({ children }: { children: React.ReactNode }) {
  const { user } = useAuth()
  const { data: settings } = useAppSettings()
  const [locked, setLocked] = useState(false)
  const [password, setPassword] = useState("")
  const [error, setError] = useState("")
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null)

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

  const handleUnlock = async () => {
    if (!user) return
    try {
      const result = await invoke<{ id: number } | null>("login", { login: user.login, password })
      if (result) {
        setLocked(false)
        setPassword("")
        setError("")
        resetTimer()
      } else {
        setError("Mot de passe incorrect")
      }
    } catch {
      setError("Erreur de connexion")
    }
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
          <form
            onSubmit={(e) => { e.preventDefault(); handleUnlock() }}
            className="space-y-4"
          >
            <Input
              type="password"
              placeholder="Mot de passe"
              value={password}
              onChange={(e) => { setPassword(e.target.value); setError("") }}
              autoFocus
            />
            {error && <p className="text-sm text-destructive">{error}</p>}
            <Button type="submit" className="w-full">Déverrouiller</Button>
          </form>
        </div>
      </div>
    </>
  )
}
