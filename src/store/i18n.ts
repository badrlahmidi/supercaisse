import { create } from "zustand"
import { persist } from "zustand/middleware"
import { translations, type Locale } from "@/lib/translations"

interface I18nState {
  locale: Locale
  setLocale: (locale: Locale) => void
  t: (key: string) => string
}

function applyLocale(locale: Locale) {
  const root = document.documentElement
  if (locale === "ar") {
    root.dir = "rtl"
    root.lang = "ar"
    root.classList.add("rtl")
  } else {
    root.dir = "ltr"
    root.lang = "fr"
    root.classList.remove("rtl")
  }
}

export const useI18nStore = create<I18nState>()(
  persist(
    (set, get) => ({
      locale: "fr",
      setLocale: (locale) => {
        set({ locale })
        applyLocale(locale)
      },
      t: (key: string) => {
        const { locale } = get()
        return translations[locale][key] ?? key
      },
    }),
    { name: "i18n-preferences", partialize: (state) => ({ locale: state.locale }) }
  )
)

export function initLocale() {
  const stored = localStorage.getItem("i18n-preferences")
  if (stored) {
    try {
      const { state } = JSON.parse(stored)
      applyLocale(state.locale || "fr")
    } catch {
      applyLocale("fr")
    }
  } else {
    applyLocale("fr")
  }
}
