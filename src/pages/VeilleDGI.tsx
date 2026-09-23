import { Card, CardContent } from "@/ui/Card"
import { Badge } from "@/ui/Badge"
import { AlertTriangle, CheckCircle, Clock, FileText, ExternalLink } from "lucide-react"
import { Button } from "@/ui/Button"
import PageHeader from "@/components/PageHeader"

const reglementations = [
  {
    titre: "Facturation électronique obligatoire",
    description: "Le Maroc prévoit la mise en place progressive de la facturation électronique. La DGI (Direction Générale des Impôts) a annoncé un calendrier de déploiement pour les entreprises soumises à la TVA.",
    statut: "en_preparation" as const,
    echeance: "2026-2027 (prévisionnel)",
    impact: "Toutes les factures devront être émises, transmises et conservées sous format électronique normalisé.",
  },
  {
    titre: "Format de facture normalisé",
    description: "Les factures devront respecter un format structuré (XML/JSON) conforme aux spécifications de la DGI, incluant : identifiants fiscaux (ICE, IF, RC), TVA détaillée par taux, signature numérique.",
    statut: "en_preparation" as const,
    echeance: "À confirmer par la DGI",
    impact: "SuperCaisse devra générer des fichiers XML conformes et les transmettre via une plateforme agréée.",
  },
  {
    titre: "Obligation de conservation numérique",
    description: "Les factures électroniques devront être conservées pendant 10 ans sous format numérique avec garantie d'intégrité (horodatage, signature).",
    statut: "actif" as const,
    echeance: "En vigueur",
    impact: "Les documents générés par SuperCaisse (factures PDF) sont déjà archivés localement. Un export cloud sera nécessaire.",
  },
  {
    titre: "Mentions obligatoires sur factures",
    description: "Chaque facture doit comporter : ICE du vendeur et de l'acheteur (B2B), numéro IF, RC, patente, taux de TVA appliqués, montant HT/TTC.",
    statut: "actif" as const,
    echeance: "En vigueur",
    impact: "SuperCaisse inclut déjà ces champs dans les paramètres et les documents générés.",
  },
  {
    titre: "Télédéclaration TVA",
    description: "Les entreprises dont le CA dépasse 2M MAD doivent télédéclarer leur TVA via le portail SIMPL de la DGI.",
    statut: "actif" as const,
    echeance: "En vigueur",
    impact: "SuperCaisse peut exporter les données de TVA (collectée/déductible) pour faciliter la déclaration.",
  },
]

const preparationChecklist = [
  { label: "ICE configuré dans les paramètres", done: true },
  { label: "IF et RC configurés", done: true },
  { label: "TVA par taux sur chaque facture", done: true },
  { label: "Numérotation séquentielle des factures", done: true },
  { label: "Export PDF des factures", done: true },
  { label: "Archivage local des documents", done: true },
  { label: "Signature numérique des factures", done: false },
  { label: "Format XML/JSON normalisé DGI", done: false },
  { label: "Transmission automatique à la plateforme DGI", done: false },
  { label: "Archivage cloud certifié", done: false },
]

export default function VeilleDGI() {
  const doneCount = preparationChecklist.filter((c) => c.done).length

  return (
    <div className="space-y-6">
      <PageHeader title="Veille réglementaire" description="Facturation électronique — Direction Générale des Impôts (DGI)" />

      <div className="grid gap-4 md:grid-cols-3">
        <Card>
          <CardContent className="pt-6 text-center">
            <div className="text-3xl font-bold text-green-600">{doneCount}</div>
            <p className="text-sm text-muted-foreground">Exigences conformes</p>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-6 text-center">
            <div className="text-3xl font-bold text-amber-600">{preparationChecklist.length - doneCount}</div>
            <p className="text-sm text-muted-foreground">À préparer</p>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-6 text-center">
            <div className="text-3xl font-bold">{Math.round((doneCount / preparationChecklist.length) * 100)}%</div>
            <p className="text-sm text-muted-foreground">Niveau de conformité</p>
          </CardContent>
        </Card>
      </div>

      <Card>
        <CardContent className="pt-6">
          <h3 className="text-lg font-semibold mb-4">Checklist de préparation</h3>
          <div className="grid gap-2 md:grid-cols-2">
            {preparationChecklist.map((item) => (
              <div key={item.label} className="flex items-center gap-3 p-2 rounded-lg hover:bg-muted/50">
                {item.done ? (
                  <CheckCircle className="h-5 w-5 text-green-500 shrink-0" />
                ) : (
                  <Clock className="h-5 w-5 text-amber-500 shrink-0" />
                )}
                <span className={item.done ? "text-muted-foreground" : "font-medium"}>{item.label}</span>
              </div>
            ))}
          </div>
        </CardContent>
      </Card>

      <div className="space-y-4">
        <h3 className="text-lg font-semibold">Réglementations en vigueur et à venir</h3>
        {reglementations.map((reg) => (
          <Card key={reg.titre}>
            <CardContent className="pt-6">
              <div className="flex items-start justify-between gap-4">
                <div className="space-y-2 flex-1">
                  <div className="flex items-center gap-2">
                    <h4 className="font-semibold">{reg.titre}</h4>
                    {reg.statut === "actif" ? (
                      <Badge className="bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200">En vigueur</Badge>
                    ) : (
                      <Badge className="bg-amber-100 text-amber-800 dark:bg-amber-900 dark:text-amber-200">En préparation</Badge>
                    )}
                  </div>
                  <p className="text-sm text-muted-foreground">{reg.description}</p>
                  <div className="flex gap-4 text-sm">
                    <span><strong>Échéance :</strong> {reg.echeance}</span>
                  </div>
                  <div className="rounded-lg bg-muted p-3 text-sm">
                    <strong>Impact SuperCaisse :</strong> {reg.impact}
                  </div>
                </div>
              </div>
            </CardContent>
          </Card>
        ))}
      </div>

      <Card>
        <CardContent className="pt-6">
          <div className="flex items-center gap-2 mb-3">
            <AlertTriangle className="h-5 w-5 text-amber-500" />
            <h3 className="font-semibold">Ressources utiles</h3>
          </div>
          <div className="grid gap-2 md:grid-cols-2">
            <Button variant="outline" className="justify-start gap-2" onClick={() => window.open("https://www.tax.gov.ma", "_blank")}>
              <ExternalLink className="h-4 w-4" />
              Portail DGI (tax.gov.ma)
            </Button>
            <Button variant="outline" className="justify-start gap-2" onClick={() => window.open("https://simpl.tax.gov.ma", "_blank")}>
              <FileText className="h-4 w-4" />
              Portail SIMPL — Télédéclarations
            </Button>
          </div>
        </CardContent>
      </Card>
    </div>
  )
}
