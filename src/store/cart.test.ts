import { describe, it, expect, beforeEach } from "vitest"
import { useCartStore, basculerPanier, clePanier } from "./cart"

beforeEach(() => {
  useCartStore.setState({ items: [], selectedClient: null, paymentMode: "especes", discountPercent: "0", cashGiven: "" })
})

describe("useCartStore", () => {
  it("adds item to empty cart", () => {
    useCartStore.getState().addItem({ article_id: 1, designation: "Test", quantite: 2, prix_unitaire: 10, tva: 20 })
    expect(useCartStore.getState().items).toHaveLength(1)
    expect(useCartStore.getState().items[0].quantite).toBe(2)
  })

  it("increments quantity for existing item", () => {
    useCartStore.getState().addItem({ article_id: 1, designation: "Test", quantite: 2, prix_unitaire: 10, tva: 20 })
    useCartStore.getState().addItem({ article_id: 1, designation: "Test", quantite: 3, prix_unitaire: 10, tva: 20 })
    expect(useCartStore.getState().items).toHaveLength(1)
    expect(useCartStore.getState().items[0].quantite).toBe(5)
  })

  it("removes item when quantity set to 0", () => {
    useCartStore.getState().addItem({ article_id: 1, designation: "Test", quantite: 2, prix_unitaire: 10, tva: 20 })
    useCartStore.getState().updateQuantity(1, 0)
    expect(useCartStore.getState().items).toHaveLength(0)
  })

  it("removes item by id", () => {
    useCartStore.getState().addItem({ article_id: 1, designation: "A", quantite: 1, prix_unitaire: 10, tva: 20 })
    useCartStore.getState().addItem({ article_id: 2, designation: "B", quantite: 1, prix_unitaire: 10, tva: 20 })
    useCartStore.getState().removeItem(1)
    expect(useCartStore.getState().items).toHaveLength(1)
    expect(useCartStore.getState().items[0].article_id).toBe(2)
  })

  it("keeps two variants of the same article as separate lines", () => {
    useCartStore.getState().addItem({ article_id: 1, variante_id: 10, designation: "T-shirt (M)", quantite: 1, prix_unitaire: 100, tva: 20 })
    useCartStore.getState().addItem({ article_id: 1, variante_id: 11, designation: "T-shirt (L)", quantite: 1, prix_unitaire: 100, tva: 20 })
    expect(useCartStore.getState().items).toHaveLength(2)
    expect(useCartStore.getState().items[0].variante_id).toBe(10)
    expect(useCartStore.getState().items[1].variante_id).toBe(11)
  })

  it("merges quantity only for the same variant, not other variants of the same article", () => {
    useCartStore.getState().addItem({ article_id: 1, variante_id: 10, designation: "T-shirt (M)", quantite: 1, prix_unitaire: 100, tva: 20 })
    useCartStore.getState().addItem({ article_id: 1, variante_id: 11, designation: "T-shirt (L)", quantite: 1, prix_unitaire: 100, tva: 20 })
    useCartStore.getState().addItem({ article_id: 1, variante_id: 10, designation: "T-shirt (M)", quantite: 2, prix_unitaire: 100, tva: 20 })
    expect(useCartStore.getState().items).toHaveLength(2)
    const m = useCartStore.getState().items.find((i) => i.variante_id === 10)
    const l = useCartStore.getState().items.find((i) => i.variante_id === 11)
    expect(m?.quantite).toBe(3)
    expect(l?.quantite).toBe(1)
  })

  it("updateQuantity/removeItem target the right variant only", () => {
    useCartStore.getState().addItem({ article_id: 1, variante_id: 10, designation: "T-shirt (M)", quantite: 1, prix_unitaire: 100, tva: 20 })
    useCartStore.getState().addItem({ article_id: 1, variante_id: 11, designation: "T-shirt (L)", quantite: 1, prix_unitaire: 100, tva: 20 })
    useCartStore.getState().updateQuantity(1, 5, 10)
    expect(useCartStore.getState().items.find((i) => i.variante_id === 10)?.quantite).toBe(5)
    expect(useCartStore.getState().items.find((i) => i.variante_id === 11)?.quantite).toBe(1)
    useCartStore.getState().removeItem(1, 11)
    expect(useCartStore.getState().items).toHaveLength(1)
    expect(useCartStore.getState().items[0].variante_id).toBe(10)
  })

  it("does not confuse a variant line with the plain article line", () => {
    useCartStore.getState().addItem({ article_id: 1, designation: "T-shirt", quantite: 1, prix_unitaire: 100, tva: 20 })
    useCartStore.getState().addItem({ article_id: 1, variante_id: 10, designation: "T-shirt (M)", quantite: 1, prix_unitaire: 100, tva: 20 })
    expect(useCartStore.getState().items).toHaveLength(2)
  })

  it("clears cart completely", () => {
    useCartStore.getState().addItem({ article_id: 1, designation: "Test", quantite: 1, prix_unitaire: 10, tva: 20 })
    useCartStore.getState().setSelectedClient(5)
    useCartStore.getState().setDiscountPercent("10")
    useCartStore.getState().clearCart()
    expect(useCartStore.getState().items).toHaveLength(0)
    expect(useCartStore.getState().selectedClient).toBeNull()
    expect(useCartStore.getState().discountPercent).toBe("0")
  })

  it("garde un panier et des tickets en attente propres à chaque utilisateur", async () => {
    localStorage.setItem("supercaisse-cart", JSON.stringify({ state: { items: [{ article_id: 9 }] }, version: 2 }))
    await basculerPanier(1)
    expect(localStorage.getItem("supercaisse-cart")).toBeNull()
    useCartStore.getState().addItem({ article_id: 1, designation: "Pain", quantite: 1, prix_unitaire: 2, tva: 0 })
    useCartStore.getState().holdCart("Table 4")
    useCartStore.getState().addItem({ article_id: 2, designation: "Lait", quantite: 1, prix_unitaire: 7, tva: 0 })

    await basculerPanier(2)
    expect(useCartStore.getState().items).toHaveLength(0)
    expect(useCartStore.getState().heldCarts).toHaveLength(0)
    useCartStore.getState().addItem({ article_id: 3, designation: "Eau", quantite: 1, prix_unitaire: 5, tva: 0 })

    await basculerPanier(1)
    expect(useCartStore.getState().items.map((i) => i.article_id)).toEqual([2])
    expect(useCartStore.getState().heldCarts.map((h) => h.label)).toEqual(["Table 4"])

    await basculerPanier(null)
    expect(useCartStore.getState().items).toHaveLength(0)
    expect(JSON.parse(localStorage.getItem(clePanier(2)) ?? "{}").state.items[0].article_id).toBe(3)
  })
})
