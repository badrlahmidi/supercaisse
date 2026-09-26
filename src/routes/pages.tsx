import { lazy, type ReactNode } from "react"
import { Outlet } from "react-router-dom"
import { AuthProvider } from "@/context/AuthContext"

export const Login = lazy(() => import("@/pages/Login"))
export const Dashboard = lazy(() => import("@/pages/Dashboard"))
export const POS = lazy(() => import("@/pages/POS"))
export const Articles = lazy(() => import("@/pages/Articles"))
export const Clients = lazy(() => import("@/pages/Clients"))
export const Paiements = lazy(() => import("@/pages/Paiements"))
export const Fournisseurs = lazy(() => import("@/pages/Fournisseurs"))
export const Ventes = lazy(() => import("@/pages/Ventes"))
export const Achats = lazy(() => import("@/pages/Achats"))
export const Categories = lazy(() => import("@/pages/Categories"))
export const Stock = lazy(() => import("@/pages/Stock"))
export const MouvementsStock = lazy(() => import("@/pages/MouvementsStock"))
export const PeremptionsStock = lazy(() => import("@/pages/PeremptionsStock"))
export const JournalCaisse = lazy(() => import("@/pages/JournalCaisse"))
export const Magasins = lazy(() => import("@/pages/Magasins"))
export const Settings = lazy(() => import("@/pages/Settings"))
export const Cheques = lazy(() => import("@/pages/Cheques"))
export const AuditLog = lazy(() => import("@/pages/AuditLog"))
export const Rapports = lazy(() => import("@/pages/Rapports"))
export const Inventaire = lazy(() => import("@/pages/Inventaire"))
export const Reappro = lazy(() => import("@/pages/Reappro"))
export const Caisses = lazy(() => import("@/pages/Caisses"))
export const ComparaisonPrix = lazy(() => import("@/pages/ComparaisonPrix"))
export const Rapprochement = lazy(() => import("@/pages/Rapprochement"))
export const VeilleDGI = lazy(() => import("@/pages/VeilleDGI"))
export const Cuisine = lazy(() => import("@/pages/Cuisine"))
export const Boutiques = lazy(() => import("@/pages/Boutiques"))
export const Peripheriques = lazy(() => import("@/pages/Peripheriques"))

export function AuthLayout({ children }: { children?: ReactNode }) {
  return (
    <AuthProvider>
      {children || <Outlet />}
    </AuthProvider>
  )
}
