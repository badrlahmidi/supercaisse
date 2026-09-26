import { useState } from "react"
import { useMutation } from "@tanstack/react-query"
import { toast } from "sonner"
import { Download, Loader2, RefreshCw } from "lucide-react"
import { invoke } from "@/lib/tauri"
import { Card, CardContent, CardHeader, CardTitle } from "@/ui/Card"
import { Button } from "@/ui/Button"

export interface EtatMiseAJour {
  version_actuelle: string
  configuree: boolean
  disponible: { version: string; date: string | null; notes: string | null } | null
}

export default function MisesAJour() {
  const [etat, setEtat] = useState<EtatMiseAJour | null>(null)

  const verifier = useMutation({
    mutationFn: () => invoke<EtatMiseAJour>("verifier_mise_a_jour"),
    onSuccess: setEtat,
    onError: (err) => toast.error("Recherche de mise à jour impossible", { description: String(err) }),
  })

  const installer = useMutation({
    mutationFn: () => invoke<void>("installer_mise_a_jour"),
    onError: (err) => toast.error("Installation impossible", { description: String(err) }),
  })

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <RefreshCw className="h-5 w-5" />
          À propos et mises à jour
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="flex items-center justify-between gap-4">
          <div>
            <p className="font-medium">SuperCaisse {etat?.version_actuelle ?? __APP_VERSION__}</p>
            <p className="text-sm text-muted-foreground">Les mises à jour sont signées et vérifiées avant installation.</p>
          </div>
          <Button variant="outline" onClick={() => verifier.mutate()} disabled={verifier.isPending || installer.isPending}>
            {verifier.isPending ? <Loader2 className="h-4 w-4 mr-2 animate-spin" /> : <RefreshCw className="h-4 w-4 mr-2" />}
            Rechercher une mise à jour
          </Button>
        </div>
        {etat && !etat.configuree && (
          <p className="text-sm text-amber-700 dark:text-amber-300">
            Mises à jour automatiques non configurées : la clé publique de signature n'est pas renseignée (voir docs/RELEASE.md).
          </p>
        )}
        {etat?.configuree && !etat.disponible && (
          <p className="text-sm text-muted-foreground">SuperCaisse est à jour.</p>
        )}
        {etat?.disponible && (
          <div className="space-y-3 rounded-md border p-3">
            <p className="font-medium">Version {etat.disponible.version} disponible</p>
            {etat.disponible.notes && (
              <p className="whitespace-pre-line text-sm text-muted-foreground">{etat.disponible.notes}</p>
            )}
            <p className="text-sm text-muted-foreground">
              Une sauvegarde de la base est faite avant l'installation, puis l'application redémarre.
            </p>
            <Button onClick={() => installer.mutate()} disabled={installer.isPending}>
              {installer.isPending ? <Loader2 className="h-4 w-4 mr-2 animate-spin" /> : <Download className="h-4 w-4 mr-2" />}
              Installer et redémarrer
            </Button>
          </div>
        )}
      </CardContent>
    </Card>
  )
}
