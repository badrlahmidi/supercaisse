import { create } from "zustand"
import { persist } from "zustand/middleware"

interface UIState {
  sidebarCollapsed: boolean
  theme: "light" | "dark" | "system"
  toggleSidebar: () => void
  setTheme: (theme: "light" | "dark" | "system") => void
}

export const useUIStore = create<UIState>()(
  persist(
    (set) => ({
      sidebarCollapsed: false,
      theme: "system",
      toggleSidebar: () => set((state) => ({ sidebarCollapsed: !state.sidebarCollapsed })),
      setTheme: (theme) => {
        set({ theme })
        applyTheme(theme)
      },
    }),
    { name: "ui-preferences" }
  )
)

export function applyTheme(theme: "light" | "dark" | "system") {
  const root = document.documentElement
  root.classList.remove("light", "dark")
  if (theme === "system") {
    const prefersDark = window.matchMedia("(prefers-color-scheme: dark)").matches
    root.classList.add(prefersDark ? "dark" : "light")
  } else {
    root.classList.add(theme)
  }
}

export function initTheme() {
  const stored = localStorage.getItem("ui-preferences")
  if (stored) {
    try {
      const { state } = JSON.parse(stored)
      applyTheme(state.theme || "system")
    } catch {
      applyTheme("system")
    }
  } else {
    applyTheme("system")
  }
}
