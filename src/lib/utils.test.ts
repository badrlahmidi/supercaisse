import { describe, it, expect } from "vitest"
import { cn, formatCurrency, formatDate, formatDateTime, getInitials, clamp, debounce } from "./utils"

describe("cn", () => {
  it("merges class names", () => {
    expect(cn("px-4", "py-2")).toBe("px-4 py-2")
  })

  it("handles conditional classes", () => {
    expect(cn("base", false && "hidden", "visible")).toBe("base visible")
  })

  it("merges tailwind classes correctly", () => {
    expect(cn("px-4", "px-6")).toBe("px-6")
  })
})

describe("formatCurrency", () => {
  it("formats number as MAD", () => {
    const result = formatCurrency(150.5, "MAD")
    expect(result).toContain("150")
  })

  it("handles zero", () => {
    const result = formatCurrency(0, "MAD")
    expect(result).toContain("0")
  })
})

describe("formatDate", () => {
  it("formats a date string", () => {
    const result = formatDate("2024-03-15")
    expect(result).toContain("03")
    expect(result).toContain("2024")
  })

  it("formats a Date object", () => {
    const result = formatDate(new Date(2024, 0, 1))
    expect(result).toContain("01")
    expect(result).toContain("2024")
  })
})

describe("formatDateTime", () => {
  it("includes time components", () => {
    const result = formatDateTime("2024-03-15T14:30:00")
    expect(result).toContain("14")
    expect(result).toContain("30")
  })
})

describe("getInitials", () => {
  it("returns initials from full name", () => {
    expect(getInitials("John Doe")).toBe("JD")
  })

  it("handles single name", () => {
    expect(getInitials("Alice")).toBe("A")
  })

  it("limits to 2 characters", () => {
    expect(getInitials("John Michael Doe")).toBe("JM")
  })
})

describe("clamp", () => {
  it("clamps within range", () => {
    expect(clamp(5, 0, 10)).toBe(5)
  })

  it("clamps below minimum", () => {
    expect(clamp(-5, 0, 10)).toBe(0)
  })

  it("clamps above maximum", () => {
    expect(clamp(15, 0, 10)).toBe(10)
  })
})

describe("debounce", () => {
  it("delays function execution", async () => {
    let callCount = 0
    const fn = debounce(() => { callCount++ }, 50)
    fn()
    fn()
    fn()
    expect(callCount).toBe(0)
    await new Promise((r) => setTimeout(r, 100))
    expect(callCount).toBe(1)
  })
})
