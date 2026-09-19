import { create } from "zustand"
import { persist } from "zustand/middleware"

interface PaymentSplit {
  mode: string
  amount: number
}

interface CartItem {
  article_id: number
  designation: string
  quantite: number
  prix_unitaire: number
  tva: number
  remise_ligne: number
  note: string
}

interface HeldCart {
  id: string
  label: string
  date: string
  state: CartState
}

interface CartState {
  items: CartItem[]
  selectedClient: number | null
  paymentMode: string
  discountPercent: string
  cashGiven: string
  paymentSplits: PaymentSplit[]
  heldCarts: HeldCart[]
  useLoyaltyPoints: boolean
  activeTableId: number | null
  activeTableNom: string | null
  addItem: (item: CartItem) => void
  updateQuantity: (articleId: number, quantity: number) => void
  removeItem: (articleId: number) => void
  clearCart: () => void
  setSelectedClient: (clientId: number | null) => void
  setPaymentMode: (mode: string) => void
  setDiscountPercent: (percent: string) => void
  setCashGiven: (cash: string) => void
  setLineDiscount: (articleId: number, percent: number) => void
  setLineNote: (articleId: number, note: string) => void
  addSplit: (mode: string) => void
  removeSplit: (index: number) => void
  updateSplitAmount: (index: number, amount: number) => void
  updateSplitMode: (index: number, mode: string) => void
  holdCart: (label: string, customId?: string) => string
  resumeCart: (id: string) => void
  deleteHeldCart: (id: string) => void
  setUseLoyaltyPoints: (use: boolean) => void
  activeTableId: number | null
  activeTableNom: string | null
  setActiveTable: (id: number | null, nom: string | null) => void
}

export const useCartStore = create<CartState>()(
  persist(
    (set) => ({
      items: [],
      selectedClient: null,
      paymentMode: "especes",
      discountPercent: "0",
      cashGiven: "",
      paymentSplits: [],
      heldCarts: [],
      useLoyaltyPoints: false,
      activeTableId: null,
      activeTableNom: null,
      setActiveTable: (id, nom) => set({ activeTableId: id, activeTableNom: nom }),
      addItem: (item) =>
        set((state) => {
          const existing = state.items.find((i) => i.article_id === item.article_id)
          if (existing) {
            return {
              items: state.items.map((i) =>
                i.article_id === item.article_id ? { ...i, quantite: i.quantite + item.quantite } : i
              ),
            }
          }
          return { items: [...state.items, { ...item, remise_ligne: item.remise_ligne ?? 0, note: item.note ?? "" }] }
        }),
      updateQuantity: (articleId, quantity) =>
        set((state) => {
          if (quantity <= 0) {
            return { items: state.items.filter((i) => i.article_id !== articleId) }
          }
          return { items: state.items.map((i) => (i.article_id === articleId ? { ...i, quantite: quantity } : i)) }
        }),
      removeItem: (articleId) =>
        set((state) => ({ items: state.items.filter((i) => i.article_id !== articleId) })),
      clearCart: () =>
        set({ items: [], selectedClient: null, discountPercent: "0", cashGiven: "", paymentMode: "especes", paymentSplits: [], useLoyaltyPoints: false, activeTableId: null, activeTableNom: null }),
      setSelectedClient: (clientId) => set({ selectedClient: clientId }),
      setPaymentMode: (mode) => set({ paymentMode: mode }),
      setDiscountPercent: (percent) => set({ discountPercent: percent }),
      setCashGiven: (cash) => set({ cashGiven: cash }),
      setLineDiscount: (articleId, percent) =>
        set((state) => ({
          items: state.items.map((i) =>
            i.article_id === articleId ? { ...i, remise_ligne: Math.max(0, Math.min(100, percent)) } : i
          ),
        })),
      setLineNote: (articleId, note) =>
        set((state) => ({
          items: state.items.map((i) =>
            i.article_id === articleId ? { ...i, note } : i
          ),
        })),
      addSplit: (mode) =>
        set((state) => ({
          paymentSplits: [...state.paymentSplits, { mode, amount: 0 }],
        })),
      removeSplit: (index) =>
        set((state) => ({
          paymentSplits: state.paymentSplits.filter((_, i) => i !== index),
        })),
      updateSplitAmount: (index, amount) =>
        set((state) => ({
          paymentSplits: state.paymentSplits.map((s, i) =>
            i === index ? { ...s, amount: Math.max(0, amount) } : s
          ),
        })),
      updateSplitMode: (index, mode) =>
        set((state) => ({
          paymentSplits: state.paymentSplits.map((s, i) =>
            i === index ? { ...s, mode } : s
          ),
        })),
      holdCart: (label, customId) => {
        const id = customId || crypto.randomUUID()
        set((state) => {
          const newHeld: HeldCart = {
            id,
            label: label || `Ticket #${state.heldCarts.length + 1}`,
            date: new Date().toISOString(),
            state: { ...state, heldCarts: [] },
          }
          return {
            heldCarts: [...state.heldCarts, newHeld],
            items: [],
            selectedClient: null,
            discountPercent: "0",
            cashGiven: "",
            paymentMode: "especes",
            paymentSplits: [],
            useLoyaltyPoints: false,
            activeTableId: null,
            activeTableNom: null,
          }
        })
        return id
      },
      resumeCart: (id) =>
        set((state) => {
          const held = state.heldCarts.find((h) => h.id === id)
          if (!held) return state
          return {
            ...held.state,
            heldCarts: state.heldCarts.filter((h) => h.id !== id),
          }
        }),
      deleteHeldCart: (id) =>
        set((state) => ({
          heldCarts: state.heldCarts.filter((c) => c.id !== id),
        })),
      setUseLoyaltyPoints: (use) => set({ useLoyaltyPoints: use }),
    }),
    {
      name: "supercaisse-cart",
      version: 2,
    }
  )
)
