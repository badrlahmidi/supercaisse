import { createContext, useContext, useState, useEffect, useCallback, useRef, type ReactNode } from "react"
import { Navigate, useNavigate, useLocation } from "react-router-dom"
import { invoke } from "@/lib/tauri"

export interface User {
  id: number
  login: string
  nom: string
  role: "admin" | "manager" | "caissier"
}

type PermissionsMap = Record<string, Record<string, boolean>>

interface AuthContextType {
  user: User | null
  login: (login: string, password: string) => Promise<void>
  loginAs: (userData: User) => void
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
    const initAuth = async () => {
      try {
        const stored = localStorage.getItem("supercaisse_user")
        if (stored) {
          const parsed = JSON.parse(stored)
          setUser(parsed)
          await loadPermissions(parsed.role)
        }
      } catch {
        localStorage.removeItem("supercaisse_user")
      } finally {
        setIsLoading(false)
      }
    }
    initAuth()
  }, [])

  const logout_ = useCallback(() => {
    setUser(null)
    localStorage.removeItem("supercaisse_user")
    navigate("/login")
  }, [navigate])

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

  const login = useCallback(async (login: string, password: string) => {
    const userData = await invoke<User | null>("login", { login, password })
    if (!userData) throw new Error("Login ou mot de passe incorrect")
    setUser(userData)
    localStorage.setItem("supercaisse_user", JSON.stringify(userData))
    await loadPermissions(userData.role)
    navigate("/pos")
  }, [navigate, loadPermissions])

  const loginAs = useCallback((userData: User) => {
    setUser(userData)
    localStorage.setItem("supercaisse_user", JSON.stringify(userData))
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
    <AuthContext.Provider value={{ user, login, loginAs, logout, isLoading, hasPermission, permissions, hasModulePermission }}>
      {children}
    </AuthContext.Provider>
  )
}

export function useAuth() {
  const context = useContext(AuthContext)
  if (!context) throw new Error("useAuth must be used within AuthProvider")
  return context
}

export function ProtectedRoute({ children, allowedRoles }: { children: ReactNode; allowedRoles?: string[] }) {
  const { user, isLoading } = useAuth()
  const location = useLocation()

  if (isLoading) return null

  if (!user) {
    return <Navigate to="/login" state={{ from: location }} replace />
  }

  if (allowedRoles && !allowedRoles.includes(user.role)) {
    return <Navigate to="/pos" replace />
  }

  return <>{children}</>
}