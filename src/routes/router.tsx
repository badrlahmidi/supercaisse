import { lazy, type ReactNode } from "react"
import { createBrowserRouter, Navigate, Outlet } from "react-router-dom"
import { AuthProvider, ProtectedRoute } from "@/context/AuthContext"
import Layout from "@/components/Layout"
import SuspensePage from "@/components/SuspensePage"

const Login = lazy(() => import("@/pages/Login"))
const Dashboard = lazy(() => import("@/pages/Dashboard"))
const POS = lazy(() => import("@/pages/POS"))
const Articles = lazy(() => import("@/pages/Articles"))
const Clients = lazy(() => import("@/pages/Clients"))
const Paiements = lazy(() => import("@/pages/Paiements"))
const Fournisseurs = lazy(() => import("@/pages/Fournisseurs"))
const Ventes = lazy(() => import("@/pages/Ventes"))
const Achats = lazy(() => import("@/pages/Achats"))
const Categories = lazy(() => import("@/pages/Categories"))
const Stock = lazy(() => import("@/pages/Stock"))
const MouvementsStock = lazy(() => import("@/pages/MouvementsStock"))
const PeremptionsStock = lazy(() => import("@/pages/PeremptionsStock"))
const JournalCaisse = lazy(() => import("@/pages/JournalCaisse"))
const Magasins = lazy(() => import("@/pages/Magasins"))
const Settings = lazy(() => import("@/pages/Settings"))
const Cheques = lazy(() => import("@/pages/Cheques"))
const AuditLog = lazy(() => import("@/pages/AuditLog"))
const Rapports = lazy(() => import("@/pages/Rapports"))
const Inventaire = lazy(() => import("@/pages/Inventaire"))

function AuthLayout({ children }: { children?: ReactNode }) {
  return (
    <AuthProvider>
      {children || <Outlet />}
    </AuthProvider>
  )
}

export const router = createBrowserRouter([
  {
    element: <AuthLayout />,
    children: [
      {
        path: "/login",
        element: <SuspensePage><Login /></SuspensePage>,
      },
      {
        element: <Layout />,
        children: [
          { index: true, element: <SuspensePage><ProtectedRoute><Dashboard /></ProtectedRoute></SuspensePage> },
          { path: "pos", element: <SuspensePage><ProtectedRoute><POS /></ProtectedRoute></SuspensePage> },
          { path: "dashboard", element: <SuspensePage><ProtectedRoute><Dashboard /></ProtectedRoute></SuspensePage> },
          { path: "articles", element: <SuspensePage><ProtectedRoute allowedRoles={["admin", "manager"]}><Articles /></ProtectedRoute></SuspensePage> },
          { path: "clients", element: <SuspensePage><ProtectedRoute allowedRoles={["admin", "manager"]}><Clients /></ProtectedRoute></SuspensePage> },
          { path: "paiements", element: <SuspensePage><ProtectedRoute allowedRoles={["admin", "manager"]}><Paiements /></ProtectedRoute></SuspensePage> },
          { path: "fournisseurs", element: <SuspensePage><ProtectedRoute allowedRoles={["admin", "manager"]}><Fournisseurs /></ProtectedRoute></SuspensePage> },
          { path: "ventes", element: <SuspensePage><ProtectedRoute allowedRoles={["admin", "manager"]}><Ventes /></ProtectedRoute></SuspensePage> },
          { path: "achats", element: <SuspensePage><ProtectedRoute allowedRoles={["admin", "manager"]}><Achats /></ProtectedRoute></SuspensePage> },
          { path: "categories", element: <SuspensePage><ProtectedRoute allowedRoles={["admin", "manager"]}><Categories /></ProtectedRoute></SuspensePage> },
          { path: "stock", element: <SuspensePage><ProtectedRoute allowedRoles={["admin", "manager"]}><Stock /></ProtectedRoute></SuspensePage> },
          { path: "stock/mouvements", element: <SuspensePage><ProtectedRoute allowedRoles={["admin", "manager"]}><MouvementsStock /></ProtectedRoute></SuspensePage> },
          { path: "stock/peremptions", element: <SuspensePage><ProtectedRoute allowedRoles={["admin", "manager"]}><PeremptionsStock /></ProtectedRoute></SuspensePage> },
          { path: "journal", element: <SuspensePage><ProtectedRoute allowedRoles={["admin", "manager"]}><JournalCaisse /></ProtectedRoute></SuspensePage> },
          { path: "cheques", element: <SuspensePage><ProtectedRoute allowedRoles={["admin", "manager"]}><Cheques /></ProtectedRoute></SuspensePage> },
          { path: "magasins", element: <SuspensePage><ProtectedRoute allowedRoles={["admin"]}><Magasins /></ProtectedRoute></SuspensePage> },
          { path: "rapports", element: <SuspensePage><ProtectedRoute allowedRoles={["admin", "manager"]}><Rapports /></ProtectedRoute></SuspensePage> },
          { path: "inventaire", element: <SuspensePage><ProtectedRoute allowedRoles={["admin", "manager"]}><Inventaire /></ProtectedRoute></SuspensePage> },
          { path: "audit", element: <SuspensePage><ProtectedRoute allowedRoles={["admin"]}><AuditLog /></ProtectedRoute></SuspensePage> },
          { path: "settings", element: <SuspensePage><ProtectedRoute allowedRoles={["admin"]}><Settings /></ProtectedRoute></SuspensePage> },
        ],
      },
      {
        path: "*",
        element: <Navigate to="/dashboard" replace />,
      },
    ],
  },
])
