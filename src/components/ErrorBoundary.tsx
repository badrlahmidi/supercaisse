import { Component, type ReactNode, type ErrorInfo } from "react"
import { AlertTriangle, RefreshCw } from "lucide-react"
import { Button } from "@/ui/Button"
import { formaterErreur, journaliser } from "@/lib/journal"

interface Props {
  children: ReactNode
  fallback?: ReactNode
}

interface State {
  error: Error | null
}

export default class ErrorBoundary extends Component<Props, State> {
  constructor(props: Props) {
    super(props)
    this.state = { error: null }
  }

  static getDerivedStateFromError(error: Error): State {
    return { error }
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    journaliser("error", `Erreur d'affichage : ${formaterErreur(error)}${errorInfo.componentStack ?? ""}`)
  }

  handleRetry = () => {
    this.setState({ error: null })
  }

  render() {
    if (this.state.error) {
      if (this.props.fallback) return this.props.fallback

      return (
        <div className="flex flex-col items-center justify-center h-64 text-center px-4">
          <AlertTriangle className="h-12 w-12 text-destructive mb-4" />
          <h2 className="text-lg font-semibold text-foreground mb-1">
            Une erreur est survenue
          </h2>
          <p className="text-sm text-muted-foreground mb-4 max-w-md">
            {this.state.error.message || "Le chargement de cette page a échoué."}
          </p>
          <Button variant="outline" onClick={this.handleRetry}>
            <RefreshCw className="h-4 w-4 mr-2" />
            Réessayer
          </Button>
        </div>
      )
    }

    return this.props.children
  }
}
