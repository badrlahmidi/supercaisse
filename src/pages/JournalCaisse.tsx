import { useState } from "react"
import { useJournalCaisse } from "@/hooks/useJournalCaisse"
import { Card, CardContent } from "@/ui/Card"
import { Button } from "@/ui/Button"
import { Input } from "@/ui/Input"
import { Badge } from "@/ui/Badge"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/ui/Table"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/ui/Select"
import { Label } from "@/ui/Label"
import { formatCurrency, formatDateTime } from "@/lib/utils"
import { Search, Loader2, Wallet, CreditCard, ArrowUp, ArrowDown, Download, SearchX, Lock } from "lucide-react"
import PageHeader from "@/components/PageHeader"
import EmptyState from "@/components/EmptyState"
import { format, subDays } from "date-fns"
import { useAuth } from "@/context/AuthContext"
import { useCurrentSession, useCloseSession } from "@/hooks/useSessions"
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter, DialogDescription } from "@/ui/Dialog"

interface JournalEntry {
  id: number
  date: string
  utilisateur_id: number | null
  jtype: string
  montant: number
  description: string | null
  user_nom: string | null
}

export default function JournalCaisse() {
  const [search, setSearch] = useState("")
  const [dateDebut, setDateDebut] = useState<string>(format(subDays(new Date(), 30), "yyyy-MM-dd"))
  const [dateFin, setDateFin] = useState<string>(format(new Date(), "yyyy-MM-dd"))
  const [filterType, setFilterType] = useState<"all" | "entree" | "sortie">("all")

  const { user } = useAuth()
  const { data: session } = useCurrentSession(user?.id)
  const closeSession = useCloseSession()
  const [showCloseDialog, setShowCloseDialog] = useState(false)
  const [declaredCash, setDeclaredCash] = useState("")

  const { data: entries, isLoading } = useJournalCaisse(dateDebut, dateFin)

  const filtered = entries?.filter((e) => {
    if (filterType === "entree" && e.montant < 0) return false
    if (filterType === "sortie" && e.montant > 0) return false
    if (search) {
      const q = search.toLowerCase()
      if (!e.description?.toLowerCase().includes(q) &&
          !e.user_nom?.toLowerCase().includes(q) &&
          !e.jtype.toLowerCase().includes(q) &&
          !e.id.toString().includes(q)) return false
    }
    return true
  }) || []

  const totalEntrees = filtered.filter(e => e.montant > 0).reduce((s, e) => s + e.montant, 0)
  const totalSorties = filtered.filter(e => e.montant < 0).reduce((s, e) => s + Math.abs(e.montant), 0)
  const solde = totalEntrees - totalSorties

  return (
    <div className="space-y-6">
      <PageHeader title="Journal de caisse" description="Suivi des mouvements de caisse">
        <div className="flex gap-2">
          {session && (
            <Button variant="destructive" onClick={() => setShowCloseDialog(true)}>
              <Lock className="h-4 w-4 mr-2" />
              Clôturer caisse (Z)
            </Button>
          )}
          <Button variant="outline">
            <Download className="h-4 w-4 mr-2" />
            Exporter
          </Button>
        </div>
      </PageHeader>

      <div className="grid gap-4 md:grid-cols-4">
        <Card>
          <CardContent className="pt-6">
            <div className="flex items-center justify-between">
              <div>
                <p className="text-sm text-muted-foreground">Entrées</p>
                <p className="text-2xl font-bold text-success">{formatCurrency(totalEntrees)}</p>
              </div>
              <div className="p-3 bg-success/10 rounded-xl">
                <ArrowUp className="h-6 w-6 text-success" />
              </div>
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-6">
            <div className="flex items-center justify-between">
              <div>
                <p className="text-sm text-muted-foreground">Sorties</p>
                <p className="text-2xl font-bold text-destructive">{formatCurrency(totalSorties)}</p>
              </div>
              <div className="p-3 bg-destructive/10 rounded-xl">
                <ArrowDown className="h-6 w-6 text-destructive" />
              </div>
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-6">
            <div className="flex items-center justify-between">
              <div>
                <p className="text-sm text-muted-foreground">Solde</p>
                <p className={`text-2xl font-bold ${solde >= 0 ? "text-success" : "text-destructive"}`}>
                  {formatCurrency(solde)}
                </p>
              </div>
              <div className="p-3 bg-primary/10 rounded-xl">
                <Wallet className="h-6 w-6 text-primary" />
              </div>
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-6">
            <div className="flex items-center justify-between">
              <div>
                <p className="text-sm text-muted-foreground">Nombre opérations</p>
                <p className="text-2xl font-bold">{filtered.length}</p>
              </div>
              <div className="p-3 bg-muted rounded-xl">
                <CreditCard className="h-6 w-6 text-muted-foreground" />
              </div>
            </div>
          </CardContent>
        </Card>
      </div>

      <Card>
        <CardContent className="pt-6">
          <div className="flex flex-col sm:flex-row gap-4 mb-4">
            <div className="relative flex-1 max-w-md">
              <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
              <Input
                placeholder="Rechercher (description, utilisateur, type, ID)..."
                value={search}
                onChange={(e) => setSearch(e.target.value)}
                className="pl-10"
              />
            </div>
            <div className="flex gap-2">
              <Select value={filterType} onValueChange={(v) => setFilterType(v as "all" | "entree" | "sortie")}>
                <SelectTrigger className="w-[160px]">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="all">Tous</SelectItem>
                  <SelectItem value="entree">Entrées seulement</SelectItem>
                  <SelectItem value="sortie">Sorties seulement</SelectItem>
                </SelectContent>
              </Select>
              <div className="flex gap-2">
                <div className="space-y-1">
                  <Label htmlFor="dateDebut" className="text-xs text-muted-foreground">Du</Label>
                  <Input id="dateDebut" type="date" value={dateDebut} onChange={(e) => setDateDebut(e.target.value)} className="w-[140px] h-9" />
                </div>
                <div className="space-y-1">
                  <Label htmlFor="dateFin" className="text-xs text-muted-foreground">Au</Label>
                  <Input id="dateFin" type="date" value={dateFin} onChange={(e) => setDateFin(e.target.value)} className="w-[140px] h-9" />
                </div>
              </div>
            </div>
          </div>

          <div className="overflow-x-auto">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead className="w-[60px]">#</TableHead>
                  <TableHead>Date</TableHead>
                  <TableHead>Type</TableHead>
                  <TableHead>Utilisateur</TableHead>
                  <TableHead className="text-right">Montant</TableHead>
                  <TableHead>Description</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {isLoading ? (
                  <TableRow>
                    <TableCell colSpan={6} className="text-center py-8">
                      <Loader2 className="h-8 w-8 animate-spin mx-auto" />
                    </TableCell>
                  </TableRow>
                ) : filtered.map((entry) => (
                  <TableRow key={entry.id}>
                    <TableCell className="font-mono text-sm">#{entry.id}</TableCell>
                    <TableCell className="text-sm">{formatDateTime(entry.date)}</TableCell>
                    <TableCell>
                      <Badge variant={entry.montant > 0 ? "success" : "destructive"} className="capitalize">
                        {entry.jtype}
                      </Badge>
                    </TableCell>
                    <TableCell>{entry.user_nom || "Système"}</TableCell>
                    <TableCell className="text-right font-medium">
                      <span className={entry.montant > 0 ? "text-success" : "text-destructive"}>
                        {entry.montant > 0 ? "+" : ""}{formatCurrency(entry.montant)}
                      </span>
                    </TableCell>
                    <TableCell>{entry.description || "—"}</TableCell>
                  </TableRow>
                ))}
                {!isLoading && !filtered.length && (
                  <TableRow>
                    <TableCell colSpan={6}>
                      <EmptyState icon={<SearchX className="h-12 w-12" />} title="Aucune opération trouvée" description="Aucune opération dans cette période" />
                    </TableCell>
                  </TableRow>
                )}
              </TableBody>
            </Table>
          </div>
        </CardContent>
      </Card>

      <Dialog open={showCloseDialog} onOpenChange={setShowCloseDialog}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Clôture de Caisse (Z)</DialogTitle>
            <DialogDescription>
              Veuillez compter le tiroir-caisse et saisir le montant exact en espèces trouvé.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-4">
            <div className="space-y-2">
              <Label>Total Espèces Déclaré (DH)</Label>
              <Input
                type="number"
                step="0.01"
                value={declaredCash}
                onChange={(e) => setDeclaredCash(e.target.value)}
                autoFocus
                className="text-lg text-center h-12 font-bold"
              />
            </div>
            {session && (
              <div className="text-sm text-muted-foreground bg-muted p-4 rounded-lg">
                <p>Fond initial: {formatCurrency(session.fond_initial)}</p>
                <p>Ouverture: {formatDateTime(session.date_ouverture)}</p>
              </div>
            )}
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setShowCloseDialog(false)}>Annuler</Button>
            <Button 
              variant="destructive"
              disabled={closeSession.isPending || !declaredCash}
              onClick={() => {
                if (session?.id) {
                  closeSession.mutate({ 
                    sessionId: session.id, 
                    totalEspecesDeclare: parseFloat(declaredCash) || 0 
                  }, {
                    onSuccess: () => {
                      setShowCloseDialog(false)
                      setDeclaredCash("")
                    }
                  })
                }
              }}
            >
              {closeSession.isPending ? <Loader2 className="h-4 w-4 animate-spin mr-2" /> : <Lock className="h-4 w-4 mr-2" />}
              Valider la clôture
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  )
}