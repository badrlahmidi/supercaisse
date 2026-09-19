import { describe, it, expect, beforeEach } from "vitest"
import { useUIStore } from "./ui"

beforeEach(() => {
  useUIStore.setState({ sidebarCollapsed: false, theme: "system" })
})

describe("useUIStore", () => {
  it("toggles sidebar", () => {
    expect(useUIStore.getState().sidebarCollapsed).toBe(false)
    useUIStore.getState().toggleSidebar()
    expect(useUIStore.getState().sidebarCollapsed).toBe(true)
    useUIStore.getState().toggleSidebar()
    expect(useUIStore.getState().sidebarCollapsed).toBe(false)
  })
})
