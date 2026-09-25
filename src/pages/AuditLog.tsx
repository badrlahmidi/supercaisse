import { useState } from "react"
import { useQuery } from "@tanstack/react-query"
import { invoke } from "@/lib/tauri"
import { Input } from "@/ui/Input"
import { Button } from "@/ui/Button"
import { Badge } from "@/ui/Badge"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/ui/Select"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/ui/Table"
import { Card, CardContent, CardHeader, CardTitle } from "@/ui/Card"
import { Shield, Download } from "lucide-react"
import { formatDate } from "@/lib/utils"

interface AuditEntry {
  id: number
  date: string
  utilisateur_id: number | null
  action: string
  detail: string | null
  reference_type: string | null
  reference_id: number | null
  user_nom: string | null
}

const ACTION_LABELS: Record<string, string> = {
  annuler_vente: "Annulation vente",
  modifier_article: "Modification article",
  supprimer_article: "Suppression article",
  modifier_parametres: "Modification paramètres",
}

const ACTION_VARIANTS: Record<string, "destructive" | "default" | "secondary" | "outline"> = {
  annuler_vente: "destructive",
  supprimer_article: "destructive",
  modifier_article: "default",
  modifier_parametres: "secondary",
}

export default function AuditLog() {
  const [debut, setDebut] = useState("")
  const [fin, setFin] = useState("")
  const [actionFilter, setActionFilter] = useState("all")

  const { data: entries = [], isLoading } = useQuery({
    queryKey: ["audit_log", debut, fin, actionFilter],
    queryFn: () => invoke<AuditEntry[]>("get_audit_log", {
      debut: debut || null,
      fin: fin || null,
      actionFilter: actionFilter === "all" ? null : actionFilter,
    }),
  })

  const exportCSV = () => {
    const header = "Date,Utilisateur,Action,Détail,Type référence,ID référence"
    const rows = entries.map(e =>
      `"${e.date}","${e.user_nom || "Système"}","${e.action}","${(e.detail || "").replace(/"/g, '""')}","${e.reference_type || ""}","${e.reference_id || ""}"`
    )
    const csv = [header, ...rows].join("\n")
    const blob = new Blob(["﻿" + csv], { type: "text/csv;charset=utf-8" })
    const url = URL.createObjectURL(blob)
    const a = document.createElement("a")
    a.href = url
    a.download = `audit_${new Date().toISOString().slice(0, 10)}.csv`
    a.click()
    URL.revokeObjectURL(url)
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <Shield className="h-8 w-8 text-primary" />
          <div>
            <h1 className="text-2xl font-bold">Journal d'audit</h1>
            <p className="text-sm text-muted-foreground">Traçabilité des actions sensibles</p>
          </div>
        </div>
        <Button variant="outline" onClick={exportCSV} disabled={entries.length === 0}>
          <Download className="h-4 w-4 mr-2" />
          Export CSV
        </Button>
      </div>

      <Card>
        <CardHeader>
          <CardTitle className="text-base">Filtres</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="flex flex-wrap gap-4">
            <div className="space-y-1">
              <label htmlFor="audit-debut" className="text-xs font-medium text-muted-foreground">Du</label>
              <Input id="audit-debut" type="date" value={debut} onChange={(e) => setDebut(e.target.value)} className="w-40" />
            </div>
            <div className="space-y-1">
              <label htmlFor="audit-fin" className="text-xs font-medium text-muted-foreground">Au</label>
              <Input id="audit-fin" type="date" value={fin} onChange={(e) => setFin(e.target.value)} className="w-40" />
            </div>
            <div className="space-y-1">
              <label htmlFor="audit-action" className="text-xs font-medium text-muted-foreground">Action</label>
              <Select value={actionFilter} onValueChange={setActionFilter}>
                <SelectTrigger id="audit-action" className="w-48">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="all">Toutes les actions</SelectItem>
                  <SelectItem value="annuler_vente">Annulation vente</SelectItem>
                  <SelectItem value="modifier_article">Modification article</SelectItem>
                  <SelectItem value="supprimer_article">Suppression article</SelectItem>
                  <SelectItem value="modifier_parametres">Modification paramètres</SelectItem>
                </SelectContent>
              </Select>
            </div>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardContent className="p-0">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead className="w-44">Date</TableHead>
                <TableHead className="w-36">Utilisateur</TableHead>
                <TableHead className="w-44">Action</TableHead>
                <TableHead>Détail</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {isLoading ? (
                <TableRow>
                  <TableCell colSpan={4} className="text-center py-8 text-muted-foreground">Chargement...</TableCell>
                </TableRow>
              ) : entries.length === 0 ? (
                <TableRow>
                  <TableCell colSpan={4} className="text-center py-8 text-muted-foreground">Aucune entrée d'audit</TableCell>
                </TableRow>
              ) : (
                entries.map((entry) => (
                  <TableRow key={entry.id}>
                    <TableCell className="text-sm">{formatDate(entry.date)}</TableCell>
                    <TableCell className="text-sm">{entry.user_nom || "Système"}</TableCell>
                    <TableCell>
                      <Badge variant={ACTION_VARIANTS[entry.action] || "outline"}>
                        {ACTION_LABELS[entry.action] || entry.action}
                      </Badge>
                    </TableCell>
                    <TableCell className="text-sm text-muted-foreground max-w-md truncate">{entry.detail}</TableCell>
                  </TableRow>
                ))
              )}
            </TableBody>
          </Table>
        </CardContent>
      </Card>
    </div>
  )
}
