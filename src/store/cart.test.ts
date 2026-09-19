import { describe, it, expect, beforeEach } from "vitest"
import { useCartStore } from "./cart"

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

  it("clears cart completely", () => {
    useCartStore.getState().addItem({ article_id: 1, designation: "Test", quantite: 1, prix_unitaire: 10, tva: 20 })
    useCartStore.getState().setSelectedClient(5)
    useCartStore.getState().setDiscountPercent("10")
    useCartStore.getState().clearCart()
    expect(useCartStore.getState().items).toHaveLength(0)
    expect(useCartStore.getState().selectedClient).toBeNull()
    expect(useCartStore.getState().discountPercent).toBe("0")
  })
})
