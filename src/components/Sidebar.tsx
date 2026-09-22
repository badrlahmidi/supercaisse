import { useMemo } from "react"
import { NavLink, useNavigate } from "react-router-dom"
import { useAuth } from "@/context/AuthContext"
import { useUIStore } from "@/store/ui"
import { useI18nStore } from "@/store/i18n"
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
  TrendingUp,
  Scale,
  Monitor,
  FileCheck,
  ChefHat,
  CreditCard,
  Landmark,
  type LucideIcon,
} from "lucide-react"

interface NavItem {
  path: string
  labelKey: string
  icon: LucideIcon
  roles: string[]
}

interface NavGroup {
  labelKey: string
  items: NavItem[]
}

const navGroups: NavGroup[] = [
  {
    labelKey: "nav.shortcuts",
    items: [
      { path: "/pos", labelKey: "nav.pos", icon: ShoppingCart, roles: ["admin", "manager", "caissier"] },
      { path: "/dashboard", labelKey: "nav.dashboard", icon: LayoutDashboard, roles: ["admin", "manager", "caissier"] },
    ],
  },
  {
    labelKey: "nav.management",
    items: [
      { path: "/articles", labelKey: "nav.articles", icon: Package, roles: ["admin", "manager"] },
      { path: "/categories", labelKey: "nav.categories", icon: Archive, roles: ["admin", "manager"] },
      { path: "/clients", labelKey: "nav.clients", icon: Users, roles: ["admin", "manager"] },
      { path: "/paiements", labelKey: "nav.clientPayments", icon: Banknote, roles: ["admin", "manager"] },
      { path: "/fournisseurs", labelKey: "nav.suppliers", icon: Truck, roles: ["admin", "manager"] },
      { path: "/reappro", labelKey: "nav.restock", icon: TrendingUp, roles: ["admin", "manager"] },
    ],
  },
  {
    labelKey: "nav.salesAndPurchases",
    items: [
      { path: "/ventes", labelKey: "nav.sales", icon: ReceiptText, roles: ["admin", "manager"] },
      { path: "/achats", labelKey: "nav.purchases", icon: ShoppingBag, roles: ["admin", "manager"] },
      { path: "/comparaison-prix", labelKey: "nav.priceComparison", icon: Scale, roles: ["admin", "manager"] },
      { path: "/rapprochement", labelKey: "nav.reconciliation", icon: FileCheck, roles: ["admin", "manager"] },
      { path: "/stock", labelKey: "nav.stock", icon: Package, roles: ["admin", "manager"] },
      { path: "/inventaire", labelKey: "nav.inventory", icon: ClipboardCheck, roles: ["admin", "manager"] },
    ],
  },
  {
    labelKey: "nav.finance",
    items: [
      { path: "/journal", labelKey: "nav.cashJournal", icon: BarChart3, roles: ["admin", "manager"] },
      { path: "/cheques", labelKey: "nav.checkTracking", icon: Banknote, roles: ["admin", "manager"] },
      { path: "/rapports", labelKey: "nav.reports", icon: FileBarChart, roles: ["admin", "manager"] },
      { path: "/caisses", labelKey: "nav.multiCaisse", icon: Monitor, roles: ["admin", "manager"] },
    ],
  },
  {
    labelKey: "nav.restaurant",
    items: [
      { path: "/cuisine", labelKey: "nav.kitchen", icon: ChefHat, roles: ["admin", "manager", "caissier"] },
    ],
  },
  {
    labelKey: "nav.system",
    items: [
      { path: "/magasins", labelKey: "nav.shops", icon: Store, roles: ["admin"] },
      { path: "/boutiques", labelKey: "nav.multiShops", icon: Store, roles: ["admin"] },
      { path: "/veille-dgi", labelKey: "nav.dgiCompliance", icon: Landmark, roles: ["admin"] },
      { path: "/peripheriques", labelKey: "nav.peripherals", icon: CreditCard, roles: ["admin"] },
      { path: "/audit", labelKey: "nav.auditLog", icon: Shield, roles: ["admin"] },
      { path: "/settings", labelKey: "nav.settings", icon: Settings, roles: ["admin"] },
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
  const t = useI18nStore((s) => s.t)
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
          "bg-sidebar-background text-sidebar-foreground border-r border-sidebar-border rtl:border-r-0 rtl:border-l",
          collapsed ? "w-16" : "w-64",
          mobileOpen ? "translate-x-0" : "-translate-x-full lg:translate-x-0 rtl:translate-x-full rtl:lg:translate-x-0"
        )}
        aria-label={t("sidebar.closeMenu")}
      >
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
            aria-label={t("sidebar.closeMenu")}
          >
            <X className="h-4 w-4" />
          </Button>
        </div>

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

        <nav className="flex-1 overflow-y-auto px-2 py-4 scrollbar-thin" aria-label={t("nav.shortcuts")}>
          {filteredGroups.map((group) => (
            <div key={group.labelKey} className="mb-4 last:mb-0">
              {!collapsed && (
                <p className="px-3 mb-1 text-xs font-semibold uppercase tracking-wider text-sidebar-accent-foreground/60">
                  {t(group.labelKey)}
                </p>
              )}
              <div className="space-y-0.5">
                {group.items.map((item) => {
                  const Icon = item.icon
                  const label = t(item.labelKey)
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
                            ? "bg-sidebar-primary/10 text-sidebar-primary before:absolute before:left-0 rtl:before:left-auto rtl:before:right-0 before:top-1/2 before:-translate-y-1/2 before:h-5 before:w-0.5 before:rounded-full before:bg-sidebar-primary"
                            : "text-sidebar-foreground",
                          collapsed && "justify-center px-2"
                        )
                      }
                      title={collapsed ? label : undefined}
                      aria-label={collapsed ? label : undefined}
                    >
                      <Icon className="h-5 w-5 shrink-0" aria-hidden="true" />
                      {!collapsed && <span className="truncate">{label}</span>}
                    </NavLink>
                  )
                })}
              </div>
            </div>
          ))}
        </nav>

        {!collapsed && (
          <div className="p-4 border-t border-sidebar-border space-y-3">
            <div className="flex items-center justify-between">
              <span className="text-xs text-sidebar-accent-foreground">{t("sidebar.appearance")}</span>
              <div className="flex gap-1">
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-7 w-7 text-sidebar-foreground hover:text-sidebar-primary-foreground"
                  onClick={() => setTheme("light")}
                  aria-label={t("sidebar.lightMode")}
                >
                  <Sun className="h-3.5 w-3.5" />
                </Button>
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-7 w-7 text-sidebar-foreground hover:text-sidebar-primary-foreground"
                  onClick={() => setTheme("dark")}
                  aria-label={t("sidebar.darkMode")}
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
              <span>{t("sidebar.logout")}</span>
            </Button>
          </div>
        )}

        <Button
          variant="ghost"
          size="icon"
          className="absolute -right-3 rtl:right-auto rtl:-left-3 top-1/2 -translate-y-1/2 lg:flex hidden rounded-full bg-sidebar-accent border border-sidebar-border shadow-lg z-10"
          onClick={toggleSidebar}
          aria-label={collapsed ? t("sidebar.expand") : t("sidebar.collapse")}
          aria-expanded={!collapsed}
        >
          {collapsed ? <ChevronRight className="h-4 w-4 text-sidebar-foreground rtl:rotate-180" /> : <ChevronLeft className="h-4 w-4 text-sidebar-foreground rtl:rotate-180" />}
        </Button>
      </aside>
    </>
  )
}
