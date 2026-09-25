import * as React from "react"
import { cn } from "@/lib/utils"

const TabsContext = React.createContext<{ value: string; onChange: (v: string) => void } | null>(null)

export function Tabs({ children, value, defaultValue, onValueChange, className }: {
  children: React.ReactNode
  value?: string
  defaultValue?: string
  onValueChange?: (v: string) => void
  className?: string
}) {
  const [interne, setInterne] = React.useState(defaultValue ?? "")
  const courant = value ?? interne
  const onChange = (v: string) => {
    if (value === undefined) setInterne(v)
    onValueChange?.(v)
  }
  return (
    <TabsContext.Provider value={{ value: courant, onChange }}>
      <div className={cn(className)}>
        {children}
      </div>
    </TabsContext.Provider>
  )
}

export function TabsList({ children, className }: { children: React.ReactNode; className?: string }) {
  return (
    <div className={cn("inline-flex h-10 items-center justify-center rounded-md bg-muted p-1 text-muted-foreground", className)}>
      {children}
    </div>
  )
}

export function TabsTrigger({ children, value, className, disabled }: { children: React.ReactNode; value: string; className?: string; disabled?: boolean }) {
  const ctx = React.useContext(TabsContext)
  const isActive = ctx?.value === value
  return (
    <button
      type="button"
      role="tab"
      className={cn(
        "inline-flex items-center justify-center whitespace-nowrap rounded-sm px-3 py-1.5 text-sm font-medium ring-offset-background transition-all focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50",
        isActive && "bg-background text-foreground shadow-sm",
        className
      )}
      onClick={() => ctx?.onChange(value)}
      disabled={disabled}
      aria-selected={isActive}
    >
      {children}
    </button>
  )
}

export function TabsContent({ children, value, className }: { children: React.ReactNode; value: string; className?: string }) {
  const ctx = React.useContext(TabsContext)
  if (ctx?.value !== value) return null
  return (
    <div className={cn("mt-2 ring-offset-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2", className)}>
      {children}
    </div>
  )
}
