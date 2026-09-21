import { useMemo } from "react"
import { NavLink, useNavigate } from "react-router-dom"
import { useAuth } from "@/context/AuthContext"
import { useUIStore } from "@/store/ui"
import { Button } from "@/ui/Button"
import { cn } from "@/lib/utils"
import {
  LayoutDashboard,
  ShoppingCart,
  Package,
  Users,
  Truck,
  ShoppingBag,
  Archive,
  ReceiptText,
  Banknote,
  Settings,
  LogOut,
  X,
  ChevronLeft,
  ChevronRight,
  Sun,
  Moon,
  BarChart3,
  Store,
  Shield,
  ClipboardCheck,
  FileBarChart,
  type LucideIcon,
} from "lucide-react"

interface NavItem {
  path: string
  label: string
  icon: LucideIcon
  roles: string[]
}

interface NavGroup {
  label: string
  items: NavItem[]
}

const navGroups: NavGroup[] = [
  {
    label: "Raccourcis",
    items: [
      { path: "/pos", label: "Caisse POS", icon: ShoppingCart, roles: ["admin", "manager", "caissier"] },
      { path: "/dashboard", label: "Tableau de bord", icon: LayoutDashboard, roles: ["admin", "manager", "caissier"] },
    ],
  },
  {
    label: "Gestion",
    items: [
      { path: "/articles", label: "Articles", icon: Package, roles: ["admin", "manager"] },
      { path: "/categories", label: "Catégories", icon: Archive, roles: ["admin", "manager"] },
      { path: "/clients", label: "Clients", icon: Users, roles: ["admin", "manager"] },
      { path: "/paiements", label: "Paiements clients", icon: Banknote, roles: ["admin", "manager"] },
      { path: "/fournisseurs", label: "Fournisseurs", icon: Truck, roles: ["admin", "manager"] },
    ],
  },
  {
    label: "Ventes & Achats",
    items: [
      { path: "/ventes", label: "Ventes", icon: ReceiptText, roles: ["admin", "manager"] },
      { path: "/achats", label: "Achats", icon: ShoppingBag, roles: ["admin", "manager"] },
      { path: "/stock", label: "Stock", icon: Package, roles: ["admin", "manager"] },
      { path: "/inventaire", label: "Inventaire", icon: ClipboardCheck, roles: ["admin", "manager"] },
    ],
  },
  {
    label: "Finance",
    items: [
      { path: "/journal", label: "Journal de caisse", icon: BarChart3, roles: ["admin", "manager"] },
      { path: "/cheques", label: "Suivi des chèques", icon: Banknote, roles: ["admin", "manager"] },
      { path: "/rapports", label: "Rapports", icon: FileBarChart, roles: ["admin", "manager"] },
    ],
  },
  {
    label: "Système",
    items: [
      { path: "/magasins", label: "Boutiques", icon: Store, roles: ["admin"] },
      { path: "/audit", label: "Journal d'audit", icon: Shield, roles: ["admin"] },
      { path: "/settings", label: "Paramètres", icon: Settings, roles: ["admin"] },
    ],
  },
]

interface SidebarProps {
  mobileOpen: boolean
  onMobileClose: () => void
}

export default function Sidebar({ mobileOpen, onMobileClose }: SidebarProps) {
  const { user, logout } = useAuth()
  const { sidebarCollapsed: collapsed, toggleSidebar, setTheme } = useUIStore()
  const navigate = useNavigate()

  const handleLogout = () => {
    logout()
    navigate("/login")
  }

  const filteredGroups = useMemo(
    () =>
      navGroups
        .map((group) => ({
          ...group,
          items: group.items.filter((item) => item.roles.includes(user?.role || "")),
        }))
        .filter((group) => group.items.length > 0),
    [user?.role]
  )

  return (
    <>
      {mobileOpen && (
        <div
          className="fixed inset-0 z-40 bg-black/50 lg:hidden"
          onClick={onMobileClose}
          aria-hidden="true"
        />
      )}

      <aside
        className={cn(
          "fixed lg:relative z-50 flex h-full flex-col transition-all duration-300 ease-in-out",
          "bg-sidebar-background text-sidebar-foreground border-r border-sidebar-border",
          collapsed ? "w-16" : "w-64",
          mobileOpen ? "translate-x-0" : "-translate-x-full lg:translate-x-0"
        )}
        aria-label="Navigation principale"
      >
        {/* Header */}
        <div className="flex h-16 items-center justify-between px-4 border-b border-sidebar-border">
          <div className={cn("flex items-center gap-2 overflow-hidden transition-all", collapsed ? "w-auto" : "w-full")}>
            <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-primary shrink-0">
              <ShoppingCart className="h-5 w-5 text-primary-foreground" />
            </div>
            {!collapsed && (
              <span className="font-bold text-lg text-sidebar-primary-foreground whitespace-nowrap">
                SuperCaisse
              </span>
            )}
          </div>
          <Button
            variant="ghost"
            size="icon"
            className="lg:hidden text-sidebar-foreground hover:text-sidebar-primary-foreground"
            onClick={onMobileClose}
            aria-label="Fermer le menu"
          >
            <X className="h-4 w-4" />
          </Button>
        </div>

        {/* User info */}
        {!collapsed && (
          <div className="px-4 py-3 border-b border-sidebar-border">
            <div className="flex items-center gap-3">
              <div className="flex h-8 w-8 items-center justify-center rounded-full bg-sidebar-primary shrink-0">
                <span className="text-sm font-medium text-sidebar-primary-foreground">
                  {user?.nom?.charAt(0).toUpperCase()}
                </span>
              </div>
              <div className="flex-1 min-w-0">
                <p className="text-sm font-medium text-sidebar-foreground truncate">{user?.nom}</p>
                <p className="text-xs text-sidebar-accent-foreground capitalize">{user?.role}</p>
              </div>
            </div>
          </div>
        )}

        {/* Navigation groups */}
        <nav className="flex-1 overflow-y-auto px-2 py-4 scrollbar-thin" aria-label="Menu principal">
          {filteredGroups.map((group) => (
            <div key={group.label} className="mb-4 last:mb-0">
              {!collapsed && (
                <p className="px-3 mb-1 text-xs font-semibold uppercase tracking-wider text-sidebar-accent-foreground/60">
                  {group.label}
                </p>
              )}
              <div className="space-y-0.5">
                {group.items.map((item) => {
                  const Icon = item.icon
                  return (
                    <NavLink
                      key={item.path}
                      to={item.path}
                      onClick={onMobileClose}
                      className={({ isActive }) =>
                        cn(
                          "relative flex items-center gap-3 rounded-lg px-3 py-2 text-sm font-medium transition-all duration-150",
                          "hover:bg-sidebar-accent hover:text-sidebar-accent-foreground",
                          isActive
                            ? "bg-sidebar-primary/10 text-sidebar-primary before:absolute before:left-0 before:top-1/2 before:-translate-y-1/2 before:h-5 before:w-0.5 before:rounded-full before:bg-sidebar-primary"
                            : "text-sidebar-foreground",
                          collapsed && "justify-center px-2"
                        )
                      }
                      title={collapsed ? item.label : undefined}
                      aria-label={collapsed ? item.label : undefined}
                    >
                      <Icon className="h-5 w-5 shrink-0" aria-hidden="true" />
                      {!collapsed && <span className="truncate">{item.label}</span>}
                    </NavLink>
                  )
                })}
              </div>
            </div>
          ))}
        </nav>

        {/* Footer */}
        {!collapsed && (
          <div className="p-4 border-t border-sidebar-border space-y-3">
            <div className="flex items-center justify-between">
              <span className="text-xs text-sidebar-accent-foreground">Apparence</span>
              <div className="flex gap-1">
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-7 w-7 text-sidebar-foreground hover:text-sidebar-primary-foreground"
                  onClick={() => setTheme("light")}
                  aria-label="Mode clair"
                >
                  <Sun className="h-3.5 w-3.5" />
                </Button>
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-7 w-7 text-sidebar-foreground hover:text-sidebar-primary-foreground"
                  onClick={() => setTheme("dark")}
                  aria-label="Mode sombre"
                >
                  <Moon className="h-3.5 w-3.5" />
                </Button>
              </div>
            </div>
            <Button
              variant="outline"
              className="w-full justify-start gap-3 text-sidebar-foreground border-sidebar-border hover:bg-sidebar-accent"
              onClick={handleLogout}
              size="sm"
            >
              <LogOut className="h-4 w-4" />
              <span>Déconnexion</span>
            </Button>
          </div>
        )}

        {/* Toggle */}
        <Button
          variant="ghost"
          size="icon"
          className="absolute -right-3 top-1/2 -translate-y-1/2 lg:flex hidden rounded-full bg-sidebar-accent border border-sidebar-border shadow-lg z-10"
          onClick={toggleSidebar}
          aria-label={collapsed ? "Étendre la sidebar" : "Réduire la sidebar"}
          aria-expanded={!collapsed}
        >
          {collapsed ? <ChevronRight className="h-4 w-4 text-sidebar-foreground" /> : <ChevronLeft className="h-4 w-4 text-sidebar-foreground" />}
        </Button>
      </aside>
    </>
  )
}
