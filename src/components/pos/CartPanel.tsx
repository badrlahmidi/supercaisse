import { useState } from "react"
import { Button } from "@/ui/Button"
import { Input } from "@/ui/Input"
import { Badge } from "@/ui/Badge"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/ui/Select"
import { useCartStore } from "@/store/cart"
import { cn, formatCurrency } from "@/lib/utils"
import { Plus, Minus, Trash2, Check, X, RotateCcw, ShoppingCart, Printer, Banknote, CreditCard, Users, Receipt, Loader2, ChevronUp, Clock, PauseCircle, PlayCircle, Percent, MessageSquare, ChefHat, Send, FileText } from "lucide-react"
import type { Article, Client } from "@/types"
import { calculerLigne, round2 } from "@/lib/totaux"

interface ReceiptData {
  shopName: string
  shopAddress: string
  shopPhone: string
  receiptFooter: string
  venteId: number
  date: string
  caissier: string
  client: string
  items: Array<{ designation: string; quantite: number; prix_unitaire: number; tva: number; total_ligne: number }>
  montantTotal: number
  montantRemise: number
  netPaye: number
  modePaiement: string
  monnaie: number
}

interface CartPanelProps {
  articles: Article[]
  clients: Client[]
  processing: boolean
  lastReceipt: ReceiptData | null
  subtotal: number
  totalTVA: number
  netAmount: number
  discount: number
  discountAmount: number
  cashAmount: number
  change: number
  itemCount: number
  onUpdateQuantity: (articleId: number, quantity: number, maxStock?: number, varianteId?: number | null) => void
  onRemoveItem: (articleId: number, varianteId?: number | null) => void
  onClearCart: () => void
  onValidateSale: () => void
  onPrintLastReceipt: () => void
  onGeneratePdf: () => void
  generatingPdf?: boolean
  documentType: string
  setDocumentType: (type: string) => void
  isLoyaltyActive: boolean
  ptsValueDH: number
  ptsEarned: number
  loyaltyDiscount: number
  ptsToUse: number
  isRestaurant?: boolean
}

const PAYMENT_MODES = [
  { value: "especes", label: "Espèces", icon: Banknote, color: "text-emerald-500", bg: "bg-emerald-500/10" },
  { value: "carte", label: "Carte", icon: CreditCard, color: "text-blue-500", bg: "bg-blue-500/10" },
  { value: "cheque", label: "Chèque", icon: Receipt, color: "text-amber-500", bg: "bg-amber-500/10" },
  { value: "credit", label: "Crédit", icon: Users, color: "text-purple-500", bg: "bg-purple-500/10" },
  { value: "virement", label: "Virement", icon: CreditCard, color: "text-cyan-500", bg: "bg-cyan-500/10" },
] as const

const QUICK_AMOUNTS = [10, 20, 50, 100, 200, 500]

export default function CartPanel({
  articles, clients, processing, lastReceipt,
  subtotal, totalTVA, netAmount, discount, discountAmount, cashAmount, change, itemCount,
  onUpdateQuantity, onRemoveItem, onClearCart, onValidateSale, onPrintLastReceipt, onGeneratePdf, generatingPdf,
  documentType, setDocumentType, isLoyaltyActive, ptsValueDH, ptsEarned, loyaltyDiscount, ptsToUse, isRestaurant
}: CartPanelProps) {
  const cart = useCartStore((s) => s.items)
  const selectedClient = useCartStore((s) => s.selectedClient)
  const discountPercent = useCartStore((s) => s.discountPercent)
  const paymentMode = useCartStore((s) => s.paymentMode)
  const cashGiven = useCartStore((s) => s.cashGiven)
  const paymentSplits = useCartStore((s) => s.paymentSplits)
  const heldCarts = useCartStore((s) => s.heldCarts)
  const setSelectedClient = useCartStore((s) => s.setSelectedClient)
  const setPaymentMode = useCartStore((s) => s.setPaymentMode)
  const setDiscountPercent = useCartStore((s) => s.setDiscountPercent)
  const setCashGiven = useCartStore((s) => s.setCashGiven)
  const setLineDiscount = useCartStore((s) => s.setLineDiscount)
  const setLineNote = useCartStore((s) => s.setLineNote)
  const setLinePrice = useCartStore((s) => s.setLinePrice)
  const holdCart = useCartStore((s) => s.holdCart)
  const resumeCart = useCartStore((s) => s.resumeCart)
  const deleteHeldCart = useCartStore((s) => s.deleteHeldCart)
  const addSplit = useCartStore((s) => s.addSplit)
  const removeSplit = useCartStore((s) => s.removeSplit)
  const updateSplitAmount = useCartStore((s) => s.updateSplitAmount)
  const updateSplitMode = useCartStore((s) => s.updateSplitMode)
  const useLoyaltyPoints = useCartStore((s) => s.useLoyaltyPoints)
  const setUseLoyaltyPoints = useCartStore((s) => s.setUseLoyaltyPoints)
  const [holdLabel, setHoldLabel] = useState("")
  const [showHoldDialog, setShowHoldDialog] = useState(false)
  
  const activeTableId = useCartStore((s) => s.activeTableId)
  const activeTableNom = useCartStore((s) => s.activeTableNom)

  const activeClient = clients.find(c => c.id === selectedClient)

  const isSplit = paymentSplits.length > 0
  const totalRemisesLignes = round2(cart.reduce(
    (s, i) => s + calculerLigne({ ...i, remise_ligne: 0 }).total_ligne - calculerLigne(i).total_ligne,
    0,
  ))
  const splitsTotal = paymentSplits.reduce((s, p) => s + p.amount, 0)
  const splitRemaining = Math.max(0, netAmount - splitsTotal)
  const cashSplit = paymentSplits.find((p) => p.mode === "especes")
  const cashTotal = cashSplit?.amount || 0
  const splitChange = cashTotal > 0 ? Math.max(0, cashTotal - (netAmount - splitsTotal + cashTotal)) : 0

  const handleHoldCart = () => {
    if (cart.length === 0) return
    if (isRestaurant && activeTableId) {
      // Envoyer en cuisine
      const ticketId = holdCart(activeTableNom || "Table")
      
      // Mettre à jour le backend
      import("@/lib/tauri").then(({ invoke }) => {
        invoke("update_table_status", { id: activeTableId, statut: "occupee", ticket_id: ticketId }).catch(console.error)
      })
      
      // Optionnel : Générer un ticket ESC/POS pour la cuisine ici
      // import("@/lib/escpos").then(...)
      
      import("sonner").then(({ toast }) => toast.success("Envoyé en cuisine"))
    } else {
      setShowHoldDialog(true)
    }
  }

  return (
    <div className="hidden lg:flex lg:w-[420px] xl:w-[460px] bg-card border-l border-border flex-col overflow-hidden">
      {/* Cart Header */}
      <div className="flex-shrink-0 flex items-center justify-between p-4 border-b border-border">
        <div className="flex items-center gap-3">
          <div className="p-2 rounded-lg bg-primary/10">
            <ShoppingCart className="h-5 w-5 text-primary" />
          </div>
          <div>
            <h2 className="font-semibold text-lg leading-none">Panier</h2>
            <p className="text-sm text-muted-foreground mt-0.5">
              {itemCount} article{itemCount > 1 ? "s" : ""}
              {itemCount > 0 && ` — ${formatCurrency(netAmount)}`}
            </p>
          </div>
        </div>
        <div className="flex items-center gap-1">
          {lastReceipt && (
            <>
              <Button variant="ghost" size="sm" onClick={onPrintLastReceipt} title="Imprimer">
                <Printer className="h-4 w-4" />
              </Button>
              <Button variant="ghost" size="sm" onClick={onGeneratePdf} disabled={generatingPdf} title="Générer PDF">
                {generatingPdf ? <Loader2 className="h-4 w-4 animate-spin" /> : <FileText className="h-4 w-4" />}
              </Button>
            </>
          )}
          {cart.length > 0 && (
            <>
              <Button variant="ghost" size="sm" onClick={handleHoldCart} title={isRestaurant ? "Envoyer en cuisine" : "Mettre en attente"}>
                {isRestaurant ? <ChefHat className="h-4 w-4" /> : <Clock className="h-4 w-4" />}
              </Button>
              <Button variant="ghost" size="sm" onClick={onClearCart} className="text-destructive hover:text-destructive">
                <RotateCcw className="h-4 w-4" />
              </Button>
            </>
          )}
        </div>
      </div>

      {/* Cart Items */}
      <div className="flex-1 overflow-y-auto scrollbar-thin p-3 space-y-2">
        {cart.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-full text-muted-foreground">
            <div className="p-4 rounded-full bg-muted mb-4">
              <ShoppingCart className="h-10 w-10" />
            </div>
            <p className="font-medium">Panier vide</p>
            <p className="text-sm text-center mt-1">Scannez ou sélectionnez des produits</p>
          </div>
        ) : (
          cart.map((item) => {
            const article = articles.find((a) => a.id === item.article_id)
            const maxStock = item.variante_id ? item.stock_max : article?.stock
            const lineTotalBase = calculerLigne({ ...item, remise_ligne: 0 }).total_ligne
            const lineTotal = calculerLigne(item).total_ligne
            const lineDiscountAmount = round2(lineTotalBase - lineTotal)
            return (
              <div
                key={`${item.article_id}-${item.variante_id ?? "x"}`}
                className="group flex items-start gap-3 p-3 rounded-xl bg-muted/30 border border-border/50 hover:border-border transition-colors animate-fade-in"
              >
                <div className="flex-1 min-w-0">
                  <p className="font-medium text-sm truncate">{item.designation}</p>
                  {item.variante_label && (
                    <Badge variant="outline" className="mt-1 text-[10px] px-1.5 py-0">{item.variante_label}</Badge>
                  )}
                  <div className="flex items-center gap-2 mt-2">
                    <div className="flex items-center border border-border rounded-md overflow-hidden">
                      <button
                        type="button"
                        className="h-8 w-8 flex items-center justify-center text-muted-foreground hover:bg-muted transition-colors"
                        onClick={() => onUpdateQuantity(item.article_id, item.quantite - 1, maxStock, item.variante_id)}
                      >
                        <Minus className="h-3.5 w-3.5" />
                      </button>
                      <input
                        type="number"
                        value={item.quantite}
                        onChange={(e) => onUpdateQuantity(item.article_id, parseInt(e.target.value) || 0, maxStock, item.variante_id)}
                        className="h-8 w-12 text-center text-sm bg-background border-x border-border outline-none [appearance:textfield] [&::-webkit-outer-spin-button]:appearance-none [&::-webkit-inner-spin-button]:appearance-none"
                        min="1"
                        max={maxStock || 999}
                      />
                      <button
                        type="button"
                        className="h-8 w-8 flex items-center justify-center text-muted-foreground hover:bg-muted transition-colors"
                        onClick={() => onUpdateQuantity(item.article_id, item.quantite + 1, maxStock, item.variante_id)}
                      >
                        <Plus className="h-3.5 w-3.5" />
                      </button>
                    </div>
                    <div className="flex items-center gap-1 ml-auto">
                      <input
                        type="number"
                        value={item.remise_ligne || 0}
                        onChange={(e) => setLineDiscount(item.article_id, parseFloat(e.target.value) || 0, item.variante_id)}
                        className="h-8 w-14 text-center text-xs bg-background border border-border rounded-md outline-none [appearance:textfield] [&::-webkit-outer-spin-button]:appearance-none [&::-webkit-inner-spin-button]:appearance-none"
                        min="0"
                        max="100"
                        placeholder="%"
                      />
                      <Percent className="h-3 w-3 text-muted-foreground" />
                    </div>
                    <span className={cn("text-sm font-semibold whitespace-nowrap", item.remise_ligne > 0 ? "text-success" : "text-primary")}>
                      {formatCurrency(lineTotal)}
                    </span>
                  </div>
                  {item.remise_ligne > 0 && (
                    <p className="text-[11px] text-success mt-1">Remise ligne: {item.remise_ligne}% (−{formatCurrency(lineDiscountAmount)})</p>
                  )}
                  <div className="mt-1.5">
                    <input
                      type="text"
                      value={item.note || ""}
                      onChange={(e) => setLineNote(item.article_id, e.target.value, item.variante_id)}
                      placeholder="Note (optionnel)"
                      className="w-full h-7 px-2 text-xs bg-background border border-border/50 rounded-md outline-none focus:ring-1 focus:ring-primary/30 placeholder:text-muted-foreground/50 text-muted-foreground"
                    />
                  </div>
                  {!item.variante_id && article?.prix_grossiste != null && (
                    <button
                      type="button"
                      onClick={() => {
                        const grossiste = item.prix_type === "grossiste"
                        setLinePrice(
                          item.article_id,
                          grossiste ? (article.prix_vente ?? item.prix_unitaire) : article.prix_grossiste!,
                          grossiste ? "public" : "grossiste"
                        )
                      }}
                      className={cn(
                        "mt-1.5 text-[11px] px-2 py-0.5 rounded-full border transition-colors",
                        item.prix_type === "grossiste"
                          ? "bg-primary/10 border-primary text-primary"
                          : "border-border text-muted-foreground hover:border-primary/50"
                      )}
                    >
                      {item.prix_type === "grossiste" ? "Prix grossiste ✓" : "Passer en prix grossiste"}
                    </button>
                  )}
                </div>
                <button
                  type="button"
                  className="p-2 text-muted-foreground hover:text-destructive hover:bg-destructive/10 rounded-lg transition-colors opacity-0 group-hover:opacity-100"
                  onClick={() => onRemoveItem(item.article_id, item.variante_id)}
                >
                  <Trash2 className="h-4 w-4" />
                </button>
              </div>
            )
          })
        )}
      </div>

      {/* Cart Bottom */}
      <div className="flex-shrink-0 border-t border-border">
        {/* Summary */}
        <div className="p-4 space-y-1.5 bg-muted/20">
          {totalRemisesLignes > 0 && (
            <div className="flex justify-between text-sm text-success">
              <span>Remises lignes (incluses)</span>
              <span>-{formatCurrency(totalRemisesLignes)}</span>
            </div>
          )}
          <div className="flex justify-between text-sm">
            <span className="text-muted-foreground">Sous-total HT</span>
            <span>{formatCurrency(subtotal)}</span>
          </div>
          <div className="flex justify-between text-sm">
            <span className="text-muted-foreground">TVA</span>
            <span>{formatCurrency(totalTVA)}</span>
          </div>
          {discount > 0 && (
            <div className="flex justify-between text-sm text-success">
              <span>Remise doc. ({discount}%)</span>
              <span>-{formatCurrency(discountAmount)}</span>
            </div>
          )}
          {loyaltyDiscount > 0 && (
            <div className="flex justify-between text-sm text-purple-600 font-medium">
              <span>Fidélité (-{ptsToUse} pts)</span>
              <span>-{formatCurrency(loyaltyDiscount)}</span>
            </div>
          )}
          <div className="flex justify-between text-lg font-bold pt-2 border-t border-border mt-2">
            <span>Net à payer</span>
            <span className="text-primary">{formatCurrency(netAmount)}</span>
          </div>
          {isLoyaltyActive && ptsEarned > 0 && (
            <div className="text-right text-xs text-muted-foreground mt-1 font-medium">
              +{ptsEarned} points de fidélité gagnés
            </div>
          )}
        </div>

        {/* Payment Section */}
        <div className="p-4 space-y-3">
          {/* Client */}
          <Select value={selectedClient ? String(selectedClient) : "none"} onValueChange={(v) => setSelectedClient(v !== "none" ? parseInt(v) : null)}>
            <SelectTrigger className="w-full h-10">
              <SelectValue placeholder="Client de passage">
                {selectedClient ? (
                  <div className="flex items-center gap-2">
                    <Users className="h-4 w-4 text-muted-foreground" />
                    <span>{clients.find((c) => c.id === selectedClient)?.nom}</span>
                    {(() => {
                      const c = clients.find((c) => c.id === selectedClient)
                      return c?.credit_actuel ? (
                        <Badge variant="warning" className="ml-1">Crédit: {formatCurrency(c.credit_actuel)}</Badge>
                      ) : null
                    })()}
                  </div>
                ) : (
                  <div className="flex items-center gap-2">
                    <Users className="h-4 w-4 text-muted-foreground" />
                    <span>Client de passage</span>
                  </div>
                )}
              </SelectValue>
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="none">
                <div className="flex items-center gap-2">
                  <Users className="h-4 w-4" />
                  Client de passage
                </div>
              </SelectItem>
              {clients.map((c) => (
                <SelectItem key={c.id} value={String(c.id)}>
                  <div className="flex items-center justify-between w-full gap-4">
                    <span>{c.nom}</span>
                    <div className="flex gap-2">
                      {c.points_fidelite > 0 && (
                        <Badge variant="default" className="text-[10px] bg-purple-600 hover:bg-purple-700">{c.points_fidelite} pts</Badge>
                      )}
                      {c.credit_actuel > 0 && (
                        <Badge variant="warning" className="text-[10px]">{formatCurrency(c.credit_actuel)}</Badge>
                      )}
                    </div>
                  </div>
                </SelectItem>
              ))}
            </SelectContent>
          </Select>

          {isLoyaltyActive && activeClient && activeClient.points_fidelite > 0 && (
            <div className="flex items-center justify-between p-2.5 rounded-lg border border-purple-200 bg-purple-50/50 dark:bg-purple-900/10 dark:border-purple-800/30">
              <div>
                <p className="text-sm font-medium text-purple-700 dark:text-purple-400">Solde fidélité : {activeClient.points_fidelite} pts</p>
                <p className="text-[10px] text-purple-600/70 dark:text-purple-400/70">Valeur : {formatCurrency(activeClient.points_fidelite * ptsValueDH)}</p>
              </div>
              <Button
                size="sm"
                variant={useLoyaltyPoints ? "default" : "outline"}
                className={useLoyaltyPoints ? "bg-purple-600 hover:bg-purple-700 text-white h-7 text-xs" : "h-7 text-xs border-purple-200 text-purple-700 dark:text-purple-400"}
                onClick={() => setUseLoyaltyPoints(!useLoyaltyPoints)}
              >
                {useLoyaltyPoints ? "Appliqué" : "Utiliser"}
              </Button>
            </div>
          )}

          <div className="grid grid-cols-2 gap-2">
            <Select value={documentType} onValueChange={setDocumentType}>
              <SelectTrigger className="w-full h-10">
                <SelectValue placeholder="Type de document" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="facture">Facture</SelectItem>
                <SelectItem value="bl">Bon de Livraison</SelectItem>
                <SelectItem value="devis">Devis</SelectItem>
                <SelectItem value="commande">Bon de commande</SelectItem>
              </SelectContent>
            </Select>

            {/* Discount */}
            <Input
              type="number"
              step="0.01"
              placeholder="Remise (%)"
              value={discountPercent}
              onChange={(e) => setDiscountPercent(e.target.value)}
              className="w-full h-10"
              inputMode="decimal"
            />
          </div>

          {/* Payment Split */}
          <div className="space-y-2">
            <div className="flex items-center justify-between">
              <span className="text-xs font-medium text-muted-foreground uppercase tracking-wider">Paiement</span>
              {!isSplit && (
                <Button variant="ghost" size="sm" className="h-7 text-xs" onClick={() => addSplit("especes")}>
                  <Plus className="h-3 w-3 mr-1" />
                  Split
                </Button>
              )}
            </div>

            {!isSplit ? (
              <>
                <div className="grid grid-cols-5 gap-1.5">
                  {PAYMENT_MODES.map((mode) => {
                    const Icon = mode.icon
                    return (
                      <button
                        key={mode.value}
                        type="button"
                        className="flex flex-col items-center gap-1 py-2.5 px-1 rounded-lg text-xs font-medium transition-all duration-150 text-muted-foreground hover:bg-muted/50"
                        onClick={() => {
                          setPaymentMode(mode.value)
                          if (mode.value === "especes") {
                            addSplit(mode.value)
                          }
                        }}
                      >
                        <Icon className={cn("h-5 w-5 text-muted-foreground")} />
                        <span className="leading-tight text-center">{mode.label}</span>
                      </button>
                    )
                  })}
                </div>

                {/* Simple cash input when not split */}
                {paymentMode === "especes" && (
                  <div className="space-y-2 animate-fade-in">
                    <Input
                      type="number"
                      step="0.01"
                      placeholder="Montant donné"
                      value={cashGiven}
                      onChange={(e) => setCashGiven(e.target.value)}
                      className="w-full h-10 text-base font-medium"
                      inputMode="decimal"
                    />
                    <div className="flex flex-wrap gap-1.5">
                      {QUICK_AMOUNTS.map((amt) => (
                        <Button key={amt} variant="secondary" size="sm" className="h-8 text-xs" onClick={() => setCashGiven(String(amt))}>
                          {formatCurrency(amt)}
                        </Button>
                      ))}
                      <Button variant="outline" size="sm" className="h-8 text-xs" onClick={() => setCashGiven(netAmount.toFixed(2))}>
                        Exact
                      </Button>
                    </div>
                    {parseFloat(cashGiven) > 0 && (
                      <div className={cn(
                        "p-3 rounded-lg text-center font-semibold text-sm",
                        parseFloat(cashGiven) >= netAmount
                          ? "bg-success/10 text-success border border-success/20"
                          : "bg-destructive/10 text-destructive border border-destructive/20"
                      )}>
                        {parseFloat(cashGiven) >= netAmount
                          ? `Monnaie à rendre : ${formatCurrency(parseFloat(cashGiven) - netAmount)}`
                          : `Il manque ${formatCurrency(netAmount - parseFloat(cashGiven))}`
                        }
                      </div>
                    )}
                  </div>
                )}
              </>
            ) : (
              <div className="space-y-2 animate-fade-in">
                {paymentSplits.map((split, idx) => (
                  <div key={idx} className="flex items-center gap-2">
                    <select
                      value={split.mode}
                      onChange={(e) => updateSplitMode(idx, e.target.value)}
                      className="h-10 px-2 rounded-lg border border-border bg-background text-sm font-medium outline-none focus:ring-2 focus:ring-primary/30"
                    >
                      {PAYMENT_MODES.map((m) => (
                        <option key={m.value} value={m.value}>{m.label}</option>
                      ))}
                    </select>
                    <Input
                      type="number"
                      step="0.01"
                      min="0"
                      placeholder="Montant"
                      value={split.amount || ""}
                      onChange={(e) => updateSplitAmount(idx, parseFloat(e.target.value) || 0)}
                      className="flex-1 h-10"
                      inputMode="decimal"
                    />
                    {paymentSplits.length > 1 && (
                      <button
                        type="button"
                        className="p-2 text-muted-foreground hover:text-destructive rounded-lg transition-colors"
                        onClick={() => removeSplit(idx)}
                      >
                        <X className="h-4 w-4" />
                      </button>
                    )}
                  </div>
                ))}
                {splitsTotal < netAmount && (
                  <Button variant="outline" size="sm" className="w-full h-9 text-xs" onClick={() => addSplit("especes")}>
                    <Plus className="h-3 w-3 mr-1" />
                    Ajouter un mode ({formatCurrency(splitRemaining)} restant)
                  </Button>
                )}
                <div className="flex justify-between text-xs text-muted-foreground pt-1">
                  <span>Total alloué</span>
                  <span className={cn("font-medium", splitsTotal === netAmount ? "text-success" : "text-destructive")}>
                    {formatCurrency(splitsTotal)} / {formatCurrency(netAmount)}
                  </span>
                </div>
                {/* Cash split change */}
                {cashTotal > 0 && splitsTotal >= netAmount && (
                  <div className="p-2 rounded-lg text-center text-sm font-semibold bg-success/10 text-success border border-success/20">
                    Monnaie à rendre : {formatCurrency(splitChange)}
                  </div>
                )}
              </div>
            )}
          </div>

          {/* Action Buttons */}
          <div className="flex gap-2 pt-1">
            <Button variant="outline" className="flex-1 h-11" onClick={onClearCart} disabled={cart.length === 0}>
              <X className="h-4 w-4 mr-2" />
              Annuler
            </Button>
            <Button
              className="flex-1 h-11 bg-success hover:bg-success-hover text-success-foreground font-semibold"
              onClick={onValidateSale}
              disabled={cart.length === 0 || processing || (isSplit ? splitsTotal < netAmount : paymentMode === "especes" && (parseFloat(cashGiven) || 0) < netAmount)}
            >
              {processing ? (
                <>
                  <Loader2 className="h-4 w-4 animate-spin mr-2" />
                  Encaissement...
                </>
              ) : (
                <>
                  <Check className="h-4 w-4 mr-2" />
                  Encaisser {formatCurrency(netAmount)}
                </>
              )}
            </Button>
          </div>
        </div>
      </div>

      {/* Held Carts */}
      {heldCarts.length > 0 && (
        <div className="border-t border-border p-3 space-y-1.5">
          <p className="text-xs font-semibold text-muted-foreground uppercase tracking-wider flex items-center gap-1.5">
            <PauseCircle className="h-3.5 w-3.5" />
            Tickets en attente ({heldCarts.length})
          </p>
          {heldCarts.map((hc) => (
            <div key={hc.id} className="flex items-center justify-between p-2 rounded-lg bg-muted/30 border border-border/50 text-sm">
              <div className="min-w-0 flex-1">
                <p className="font-medium truncate">{hc.label}</p>
                <p className="text-xs text-muted-foreground">{new Date(hc.date).toLocaleString("fr-FR")}</p>
              </div>
              <div className="flex items-center gap-1 ml-2">
                <Button variant="ghost" size="sm" className="h-7 w-7 p-0" onClick={() => resumeCart(hc.id)} title="Reprendre">
                  <PlayCircle className="h-4 w-4 text-primary" />
                </Button>
                <Button variant="ghost" size="sm" className="h-7 w-7 p-0 text-destructive" onClick={() => deleteHeldCart(hc.id)} title="Supprimer">
                  <X className="h-4 w-4" />
                </Button>
              </div>
            </div>
          ))}
        </div>
      )}

      {/* Hold Dialog */}
      {showHoldDialog && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
          <div className="bg-card rounded-xl shadow-2xl border border-border p-6 w-full max-w-sm mx-4">
            <h3 className="font-semibold text-lg mb-1">Mettre en attente</h3>
            <p className="text-sm text-muted-foreground mb-4">Le panier sera sauvegardé pour reprise ultérieure.</p>
            <Input
              placeholder="Nom du ticket (ex: Client salle 4)"
              value={holdLabel}
              onChange={(e) => setHoldLabel(e.target.value)}
              autoFocus
              className="mb-4"
            />
            <div className="flex gap-2 justify-end">
              <Button variant="outline" onClick={() => { setShowHoldDialog(false); setHoldLabel("") }}>
                Annuler
              </Button>
              <Button onClick={() => { holdCart(holdLabel); setShowHoldDialog(false); setHoldLabel("") }}>
                <Clock className="h-4 w-4 mr-2" />
                Mettre en attente
              </Button>
            </div>
          </div>
        </div>
      )}

      {/* Mobile Cart Button */}
      <div className="lg:hidden fixed bottom-4 left-4 right-4 z-50">
        <Button
          className="w-full h-14 shadow-xl shadow-primary/20 text-base font-semibold"
          disabled={cart.length === 0}
        >
          <ShoppingCart className="h-5 w-5 mr-3" />
          Voir le panier ({itemCount}) — {formatCurrency(netAmount)}
          <ChevronUp className="h-5 w-5 ml-3" />
        </Button>
      </div>
    </div>
  )
}
