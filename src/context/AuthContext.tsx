import { createContext, useContext, useState, useEffect, useCallback, useRef, type ReactNode } from "react"
import { Navigate, useNavigate, useLocation } from "react-router-dom"
import { invoke, onSessionExpired, setSessionToken, getSessionToken } from "@/lib/tauri"
import { toast } from "sonner"
import ChangePasswordRequired from "@/components/ChangePasswordRequired"
import { accesAutorise, routeAccueil } from "@/routes/acces"

export interface User {
  id: number
  login: string
  nom: string
  role: "admin" | "manager" | "caissier"
  must_change_password?: boolean
}

export type SessionUser = User & { token?: string }

type PermissionsMap = Record<string, Record<string, boolean>>

interface AuthContextType {
  user: User | null
  login: (login: string, password: string) => Promise<void>
  loginAs: (userData: SessionUser) => Promise<void>
  completePasswordChange: () => void
  logout: () => void
  isLoading: boolean
  hasPermission: (roles: string[]) => boolean
  permissions: PermissionsMap
  hasModulePermission: (module: string, action: string) => boolean
}

const AuthContext = createContext<AuthContextType | undefined>(undefined)

const AUTO_LOCK_MS = 15 * 60 * 1000

function transformPermissions(rows: Array<{ module: string; action: string; allowed: boolean }>): PermissionsMap {
  const map: PermissionsMap = {}
  for (const row of rows) {
    if (!map[row.module]) map[row.module] = {}
    map[row.module][row.action] = row.allowed
  }
  return map
}

export function AuthProvider({ children }: { children: ReactNode }) {
  const [user, setUser] = useState<User | null>(null)
  const [permissions, setPermissions] = useState<PermissionsMap>({})
  const [isLoading, setIsLoading] = useState(true)
  const navigate = useNavigate()

  const lastActivity = useRef(Date.now())

  const loadPermissions = useCallback(async (role: string) => {
    try {
      const rows = await invoke<Array<{ module: string; action: string; allowed: boolean }>>("get_permissions", { role })
      setPermissions(transformPermissions(rows))
    } catch {
      setPermissions({})
    }
  }, [])

  useEffect(() => {
    localStorage.removeItem("supercaisse_user")
    setIsLoading(false)
  }, [])

  const clearSession = useCallback(() => {
    setSessionToken(null)
    setUser(null)
    setPermissions({})
    navigate("/login")
  }, [navigate])

  const logout_ = useCallback(() => {
    if (getSessionToken()) {
      invoke("logout").catch(() => undefined)
    }
    clearSession()
  }, [clearSession])

  useEffect(() => onSessionExpired(() => {
    toast.error("Session expirée", { description: "Veuillez vous reconnecter" })
    clearSession()
  }), [clearSession])

  useEffect(() => {
    if (!user) return
    const resetActivity = () => { lastActivity.current = Date.now() }
    const checkInactivity = () => {
      if (Date.now() - lastActivity.current > AUTO_LOCK_MS) {
        logout_()
      }
    }
    window.addEventListener("mousedown", resetActivity)
    window.addEventListener("keydown", resetActivity)
    window.addEventListener("touchstart", resetActivity)
    window.addEventListener("scroll", resetActivity, { passive: true })
    const interval = setInterval(checkInactivity, 60000)
    return () => {
      window.removeEventListener("mousedown", resetActivity)
      window.removeEventListener("keydown", resetActivity)
      window.removeEventListener("touchstart", resetActivity)
      window.removeEventListener("scroll", resetActivity)
      clearInterval(interval)
    }
  }, [user, logout_])

  const loginAs = useCallback(async ({ token, ...userData }: SessionUser) => {
    if (token) {
      if (getSessionToken() && getSessionToken() !== token) {
        await invoke("logout").catch(() => undefined)
      }
      setSessionToken(token)
    }
    setUser(userData)
    await loadPermissions(userData.role)
  }, [loadPermissions])

  const login = useCallback(async (login: string, password: string) => {
    const userData = await invoke<SessionUser | null>("login", { login, password })
    if (!userData) throw new Error("Login ou mot de passe incorrect")
    await loginAs(userData)
    navigate("/pos")
  }, [navigate, loginAs])

  const completePasswordChange = useCallback(() => {
    setUser((current) => {
      if (!current) return current
      return { ...current, must_change_password: false }
    })
  }, [])

  const logout = useCallback(() => {
    logout_()
  }, [logout_])

  const hasPermission = useCallback((roles: string[]) => {
    if (!user) return false
    return roles.includes(user.role)
  }, [user])

  const hasModulePermission = useCallback((module: string, action: string): boolean => {
    if (!user) return false
    if (user.role === "admin") return true
    return permissions[module]?.[action] ?? false
  }, [user, permissions])

  if (isLoading) {
    return (
      <div className="flex h-screen w-screen items-center justify-center bg-background">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary" />
      </div>
    )
  }

  return (
    <AuthContext.Provider value={{ user, login, loginAs, completePasswordChange, logout, isLoading, hasPermission, permissions, hasModulePermission }}>
      {children}
    </AuthContext.Provider>
  )
}

export function useAuth() {
  const context = useContext(AuthContext)
  if (!context) throw new Error("useAuth must be used within AuthProvider")
  return context
}

export function ProtectedRoute({ children, chemin }: { children: ReactNode; chemin?: string }) {
  const { user, isLoading, hasModulePermission } = useAuth()
  const location = useLocation()

  if (isLoading) return null

  if (!user) {
    return <Navigate to="/login" state={{ from: location }} replace />
  }

  if (user.must_change_password) {
    return <ChangePasswordRequired />
  }

  const habilitations = { role: user.role, aLaPermission: hasModulePermission }
  if (!chemin || !accesAutorise(chemin, habilitations)) {
    const accueil = routeAccueil(habilitations)
    if (accueil && accueil !== chemin) return <Navigate to={accueil} replace />
    return <AccesRefuse />
  }

  return <>{children}</>
}

function AccesRefuse() {
  return (
    <div className="flex h-64 flex-col items-center justify-center gap-2 text-center">
      <p className="text-lg font-semibold">Accès non autorisé</p>
      <p className="text-sm text-muted-foreground">Aucune page n'est ouverte à votre rôle. Contactez un administrateur.</p>
    </div>
  )
}

