import { describe, it, expect } from "vitest"
import { render, screen } from "@testing-library/react"
import { Button } from "./Button"

describe("Button", () => {
  it("renders with text", () => {
    render(<Button>Click me</Button>)
    expect(screen.getByRole("button", { name: /click me/i })).toBeInTheDocument()
  })

  it("renders with default variant classes", () => {
    render(<Button>Default</Button>)
    const btn = screen.getByRole("button")
    expect(btn.className).toContain("bg-primary")
  })

  it("renders with destructive variant", () => {
    render(<Button variant="destructive">Delete</Button>)
    const btn = screen.getByRole("button")
    expect(btn.className).toContain("bg-destructive")
  })

  it("renders with outline variant", () => {
    render(<Button variant="outline">Outline</Button>)
    const btn = screen.getByRole("button")
    expect(btn.className).toContain("border-input")
  })

  it("renders with different sizes", () => {
    const { rerender } = render(<Button size="sm">Small</Button>)
    expect(screen.getByRole("button").className).toContain("h-9")

    rerender(<Button size="lg">Large</Button>)
    expect(screen.getByRole("button").className).toContain("h-11")

    rerender(<Button size="icon">Icon</Button>)
    expect(screen.getByRole("button").className).toContain("w-10")
  })

  it("disables button when loading", () => {
    render(<Button loading>Loading</Button>)
    expect(screen.getByRole("button")).toBeDisabled()
  })

  it("shows spinner when loading", () => {
    const { container } = render(<Button loading>Loading</Button>)
    expect(container.querySelector("svg")).toBeInTheDocument()
  })

  it("forwards additional className", () => {
    render(<Button className="extra-class">Styled</Button>)
    expect(screen.getByRole("button").className).toContain("extra-class")
  })
})
