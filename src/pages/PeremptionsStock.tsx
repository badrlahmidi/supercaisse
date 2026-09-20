import { useState } from "react"
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query"
import { invoke } from "@/lib/tauri"
import { Card, CardContent } from "@/ui/Card"
import { Button } from "@/ui/Button"
import { Badge } from "@/ui/Badge"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/ui/Table"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/ui/Select"
import { Loader2, SearchX, CalendarClock, Trash2, AlertTriangle } from "lucide-react"
import { toast } from "sonner"
import PageHeader from "@/components/PageHeader"
import EmptyState from "@/components/EmptyState"
import { formatDate, cn } from "@/lib/utils"

interface LotPeremption {
  id: number
  article_id: number
  designation: string
  numero_lot: string | null
  date_peremption: string
  quantite: number
}

export default function PeremptionsStock() {
  const [horizon, setHorizon] = useState("30")
  const queryClient = useQueryClient()

  const { data: lots, isLoading } = useQuery({
    queryKey: ["lots_peremption_proche", horizon],
    queryFn: () => invoke<LotPeremption[]>("get_lots_peremption_proche", { jours: parseInt(horizon) }),
  })

  const discardMutation = useMutation({
    mutationFn: ({ lot_id, quantite }: { lot_id: number; quantite: number }) =>
      invoke("discard_article_lot", { lot_id, quantite, motif: "peremption" }),
    onSuccess: () => {
      toast.success("Lot retiré du stock")
      queryClient.invalidateQueries({ queryKey: ["lots_peremption_proche"] })
      queryClient.invalidateQueries({ queryKey: ["articles"] })
    },
    onError: (e) => toast.error("Erreur", { description: String(e) }),
  })

  const isExpired = (date: string) => new Date(date).getTime() < Date.now()

  const expiredCount = lots?.filter((l) => isExpired(l.date_peremption)).length || 0

  return (
    <div className="space-y-6">
      <PageHeader title="Péremptions" description="Lots à date de péremption proche ou dépassée (DLC/DLUO)" />

      <div className="grid gap-4 md:grid-cols-3">
        <Card>
          <CardContent className="pt-6 flex items-center justify-between">
            <div>
              <p className="text-sm text-muted-foreground">Lots concernés</p>
              <p className="text-2xl font-bold">{lots?.length || 0}</p>
            </div>
            <div className="p-3 bg-warning/10 rounded-xl">
              <CalendarClock className="h-6 w-6 text-warning" />
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-6 flex items-center justify-between">
            <div>
              <p className="text-sm text-muted-foreground">Déjà périmés</p>
              <p className="text-2xl font-bold text-destructive">{expiredCount}</p>
            </div>
            <div className="p-3 bg-destructive/10 rounded-xl">
              <AlertTriangle className="h-6 w-6 text-destructive" />
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-6">
            <p className="text-sm text-muted-foreground mb-2">Horizon d'alerte</p>
            <Select value={horizon} onValueChange={setHorizon}>
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="7">7 jours</SelectItem>
                <SelectItem value="15">15 jours</SelectItem>
                <SelectItem value="30">30 jours</SelectItem>
                <SelectItem value="90">90 jours</SelectItem>
              </SelectContent>
            </Select>
          </CardContent>
        </Card>
      </div>

      <Card>
        <CardContent className="pt-6">
          <div className="overflow-x-auto">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Article</TableHead>
                  <TableHead>N° lot</TableHead>
                  <TableHead>Péremption</TableHead>
                  <TableHead className="text-right">Quantité</TableHead>
                  <TableHead className="w-[60px]">Actions</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {isLoading ? (
                  <TableRow>
                    <TableCell colSpan={5} className="text-center py-8">
                      <Loader2 className="h-6 w-6 animate-spin mx-auto text-muted-foreground" />
                    </TableCell>
                  </TableRow>
                ) : !lots?.length ? (
                  <TableRow>
                    <TableCell colSpan={5}>
                      <EmptyState
                        icon={<SearchX className="h-12 w-12" />}
                        title="Aucune péremption à signaler"
                        description="Aucun lot ne périme dans l'horizon sélectionné"
                      />
                    </TableCell>
                  </TableRow>
                ) : (
                  lots.map((lot) => {
                    const expired = isExpired(lot.date_peremption)
                    return (
                      <TableRow key={lot.id} className={cn(expired ? "bg-destructive/5" : "bg-warning/5")}>
                        <TableCell className="font-medium">{lot.designation}</TableCell>
                        <TableCell className="font-mono text-sm">{lot.numero_lot || "—"}</TableCell>
                        <TableCell>
                          <Badge variant={expired ? "destructive" : "warning"}>
                            {formatDate(lot.date_peremption)}
                          </Badge>
                        </TableCell>
                        <TableCell className="text-right">{lot.quantite}</TableCell>
                        <TableCell>
                          <Button
                            variant="ghost"
                            size="icon"
                            title="Retirer du stock (péremption/casse)"
                            onClick={() => discardMutation.mutate({ lot_id: lot.id, quantite: lot.quantite })}
                            disabled={discardMutation.isPending}
                          >
                            <Trash2 className="h-4 w-4 text-destructive" />
                          </Button>
                        </TableCell>
                      </TableRow>
                    )
                  })
                )}
              </TableBody>
            </Table>
          </div>
        </CardContent>
      </Card>
    </div>
  )
}
