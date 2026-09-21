import { useState } from "react"
import { Outlet } from "react-router-dom"
import { useUIStore } from "@/store/ui"
import { Button } from "@/ui/Button"
import { Menu } from "lucide-react"
import { cn } from "@/lib/utils"
import Sidebar from "./Sidebar"
import IdleLock from "./IdleLock"

export default function Layout() {
  const { sidebarCollapsed } = useUIStore()
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false)

  return (
    <IdleLock>
    <div className="flex h-screen bg-background">
      <Sidebar mobileOpen={mobileMenuOpen} onMobileClose={() => setMobileMenuOpen(false)} />

      <main
        className={cn(
          "flex-1 overflow-y-auto transition-all duration-300",
          sidebarCollapsed ? "lg:ml-16" : "lg:ml-64"
        )}
      >
        {/* Mobile header */}
        <header className="lg:hidden sticky top-0 z-30 h-16 bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60 border-b border-border">
          <div className="flex h-full items-center justify-between px-4">
            <Button variant="ghost" size="icon" onClick={() => setMobileMenuOpen(true)} aria-label="Ouvrir le menu">
              <Menu className="h-5 w-5" />
            </Button>
            <h1 className="text-lg font-semibold text-foreground">SuperCaisse</h1>
            <div className="w-10" />
          </div>
        </header>

        {/* Page content */}
        <div className="p-4 lg:p-6 animate-fade-in">
          <Outlet />
        </div>
      </main>
    </div>
    </IdleLock>
  )
}
