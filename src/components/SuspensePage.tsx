import { type ReactNode, Suspense } from "react"
import { Loader2 } from "lucide-react"
import ErrorBoundary from "@/components/ErrorBoundary"

export default function SuspensePage({ children }: { children: ReactNode }) {
  return (
    <ErrorBoundary>
      <Suspense
        fallback={
          <div className="flex items-center justify-center h-64">
            <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
          </div>
        }
      >
        {children}
      </Suspense>
    </ErrorBoundary>
  )
}
