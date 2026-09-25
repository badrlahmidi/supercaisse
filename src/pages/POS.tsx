import { useState, useEffect, useRef, useCallback } from "react"
import { invoke } from "@/lib/tauri"
import { buildPaiements } from "@/lib/paiements"
import { calculerTotaux, round2, sommeDH } from "@/lib/totaux"
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query"
import { useAuth } from "@/context/AuthContext"
import { usePOSProducts } from "@/hooks/useProducts"
import { useCategoriesList } from "@/hooks/useCategories"
import { useClientsList } from "@/hooks/useClients"
import { useAppSettings } from "@/hooks/useSettings"
import { useCurrentSession, useOpenSession } from "@/hooks/useSessions"
import { Input } from "@/ui/Input"
import { Badge } from "@/ui/Badge"
import { Button } from "@/ui/Button"
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "@/ui/Dialog"
import { Toaster } from "@/ui/Toast"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/ui/Select"
import { Search, Package, Barcode, Loader2, Keyboard, PauseCircle, PlayCircle, Trash2, X, FileText, LockOpen, Coffee, Store, BarChart3 } from "lucide-react"
import { formatCurrency, cn } from "@/lib/utils"
import { printViaTauri, saveFacturePdf, type ReceiptData } from "@/lib/receipt"
import { useDebounce } from "@/hooks/useDebounce"
import { useCartStore } from "@/store/cart"
import { toast } from "sonner"
import CategoryPills from "@/components/pos/CategoryPills"
import ProductGrid from "@/components/pos/ProductGrid"
import CartPanel from "@/components/pos/CartPanel"
import { useTables, useUpdateTable, type TableResto } from "@/hooks/useTables"
import type { Article, ArticleVariante, Category } from "@/types"
import { estFiscal, mentionsVendeurManquantes } from "@/lib/fiscal"

interface RapportX {
  session_id: number
  date_ouverture: string
  fond_initial: number
  nb_ventes: number
  ca_total: number
  total_remises: number
  nb_annulations: number
  nb_articles_vendus: number
  par_mode: { mode: string; total: number; count: number }[]
}

const SHORTCUTS = [
  { key: "F1", label: "Recherche" },
  { key: "F2", label: "Nouveau" },
  { key: "F4", label: "Retirer" },
  { key: "F5", label: "Valider" },
  { key: "F6", label: "Paiement" },
  { key: "F7", label: "Mettre en attente" },
  { key: "F8", label: "Reprendre ticket" },
]

const PAYMENT_CYCLE = ["especes", "carte", "cheque", "credit", "virement"]

export default function POS() {
  const { user } = useAuth()
  const queryClient = useQueryClient()
  const searchRef = useRef<HTMLInputElement>(null)
  const [search, setSearch] = useState("")
  const { data: clients = [] } = useClientsList()
  const { data: settings = {} as any } = useAppSettings()
  const isRestaurant = settings.business_type === "restaurant"
  const { data: tables = [], isLoading: tablesLoading } = useTables()
  const updateTableMutation = useUpdateTable()

  const [debouncedSearch] = useDebounce(search, 300)
  const [activeCategory, setActiveCategory] = useState<number | "all">("all")
  const [processing, setProcessing] = useState(false)
  const [lastReceipt, setLastReceipt] = useState<ReceiptData | null>(null)
  const [showShortcuts, setShowShortcuts] = useState(false)
  const [showHeldPanel, setShowHeldPanel] = useState(false)
  const [, setLastSync] = useState<Date>(new Date())

  const [documentType, setDocumentType] = useState<string>("facture")
  const mentionsManquantes = mentionsVendeurManquantes(settings)
  const [variantPickerArticle, setVariantPickerArticle] = useState<Article | null>(null)
  const [showVariantPicker, setShowVariantPicker] = useState(false)
  const [showRapportX, setShowRapportX] = useState(false)

  const { data: currentSession, isLoading: isSessionLoading } = useCurrentSession(user?.id)
  const openSessionMutation = useOpenSession()
  const [fondInitial, setFondInitial] = useState("0")
  const [selectedMagasinId, setSelectedMagasinId] = useState<string>("")

  const { data: magasins = [] } = useQuery({
    queryKey: ["magasins"],
    queryFn: () => invoke<{ id: number; nom: string; adresse: string | null }[]>("get_magasins"),
  })

  const cart = useCartStore((s) => s.items)
  const selectedClient = useCartStore((s) => s.selectedClient)
  const paymentMode = useCartStore((s) => s.paymentMode)
  const discountPercent = useCartStore((s) => s.discountPercent)
  const cashGiven = useCartStore((s) => s.cashGiven)
  const addItem = useCartStore((s) => s.addItem)
  const updateQuantityStore = useCartStore((s) => s.updateQuantity)
  const removeItem = useCartStore((s) => s.removeItem)
  const clearCartStore = useCartStore((s) => s.clearCart)
  const setPaymentMode = useCartStore((s) => s.setPaymentMode)
  const paymentSplits = useCartStore((s) => s.paymentSplits)
  const holdCart = useCartStore((s) => s.holdCart)
  const resumeCart = useCartStore((s) => s.resumeCart)
  const deleteHeldCart = useCartStore((s) => s.deleteHeldCart)
  const useLoyaltyPoints = useCartStore((s) => s.useLoyaltyPoints)
  const setUseLoyaltyPoints = useCartStore((s) => s.setUseLoyaltyPoints)
  const activeTableId = useCartStore((s) => s.activeTableId)
  const activeTableNom = useCartStore((s) => s.activeTableNom)
  const setActiveTable = useCartStore((s) => s.setActiveTable)
  const heldCarts = useCartStore((s) => s.heldCarts)

  const { data: articles = [], isFetching: articlesFetching } = usePOSProducts(debouncedSearch, activeCategory)

  const { data: categories = [] } = useCategoriesList()

  const { data: pickerVariantes = [], isFetching: pickerVariantesFetching } = useQuery({
    queryKey: ["article_variantes_pos", variantPickerArticle?.id],
    queryFn: () => invoke<ArticleVariante[]>("get_article_variantes", { article_id: variantPickerArticle?.id }),
    enabled: showVariantPicker && !!variantPickerArticle,
  })

  const handleSelectTable = (table: TableResto) => {
    setActiveTable(table.id, table.nom)
    if (table.ticket_id) {
      // Reprendre le ticket si existant
      resumeCart(table.ticket_id)
    } else {
      clearCartStore() // Assure qu'on démarre sur un panier vide
    }
  }

  useEffect(() => {
    searchRef.current?.focus()
  }, [])

  const discount = parseFloat(discountPercent) || 0
  const totaux = calculerTotaux(cart, discount)
  const subtotal = totaux.sousTotalHT
  const totalTVA = totaux.totalTVABrut
  const discountAmount = totaux.montantRemise

  const activeClient = clients.find(c => c.id === selectedClient)
  const isLoyaltyActive = settings.fidelite_actif === "true"
  const ptsValueDH = parseFloat(settings.fidelite_valeur_1_point) || 1
  const ptsFor1DH = parseFloat(settings.fidelite_dh_pour_1_point) || 100

  const maxPtsUsable = Math.floor(totaux.netTTC / ptsValueDH)
  const ptsToUse = (useLoyaltyPoints && activeClient) ? Math.min(activeClient.points_fidelite, maxPtsUsable) : 0
  const loyaltyDiscount = round2(ptsToUse * ptsValueDH)

  const netAmount = Math.max(0, round2(totaux.netTTC - loyaltyDiscount))
  const ptsEarned = isLoyaltyActive && activeClient ? Math.floor(netAmount / ptsFor1DH) : 0
  const cashAmount = parseFloat(cashGiven) || 0
  const change = paymentMode === "especes" ? Math.max(0, cashAmount - netAmount) : 0
  const itemCount = cart.reduce((s, i) => s + i.quantite, 0)

  const filteredArticles = articles.filter((a) => {
    if (activeCategory !== "all" && a.categorie_id !== activeCategory) return false
    if (!debouncedSearch) return true
    const q = debouncedSearch.toLowerCase()
    return a.designation.toLowerCase().includes(q) || a.code_barre?.includes(debouncedSearch)
  })

  const addToCart = useCallback((article: Article, variante?: ArticleVariante) => {
    if (!variante && article.a_variantes) {
      setVariantPickerArticle(article)
      setShowVariantPicker(true)
      return
    }
    const stockDispo = variante ? variante.stock_dedie : article.stock
    if (stockDispo <= 0) {
      toast.error("Stock épuisé", { description: `${article.designation} n'est plus disponible` })
      return
    }
    const existing = cart.find((i) => i.article_id === article.id && (i.variante_id ?? null) === (variante?.id ?? null))
    if (existing && existing.quantite + 1 > stockDispo) {
      toast.error("Stock maximum atteint", { description: `Stock disponible: ${stockDispo}` })
      return
    }
    const label = variante ? [variante.taille, variante.couleur].filter(Boolean).join(" / ") : undefined
    addItem({
      article_id: article.id,
      variante_id: variante?.id,
      variante_label: label,
      designation: label ? `${article.designation} (${label})` : article.designation,
      quantite: 1,
      prix_unitaire: article.prix_vente,
      tva: article.tva,
      remise_ligne: 0,
      stock_max: stockDispo,
    })
    setShowVariantPicker(false)
    searchRef.current?.focus()
  }, [cart, addItem])

  const updateQuantity = useCallback((articleId: number, quantity: number, maxStock?: number, varianteId?: number | null) => {
    if (maxStock && quantity > maxStock) {
      toast.error("Stock maximum atteint", { description: `Stock disponible: ${maxStock}` })
      return
    }
    updateQuantityStore(articleId, quantity, varianteId)
  }, [updateQuantityStore])

  const removeFromCart = useCallback((articleId: number, varianteId?: number | null) => {
    removeItem(articleId, varianteId)
  }, [removeItem])

  const clearCart = useCallback(() => {
    clearCartStore()
    searchRef.current?.focus()
  }, [clearCartStore])

  const handleHoldCart = useCallback(() => {
    const items = useCartStore.getState().items
    if (items.length === 0) {
      toast.info("Panier vide", { description: "Aucun article à mettre en attente" })
      return
    }
    const label = `Ticket #${useCartStore.getState().heldCarts.length + 1}`
    holdCart(label)
    toast.success("Ticket mis en attente", { description: label })
    searchRef.current?.focus()
  }, [holdCart])

  const handleResumeCart = useCallback((id: string, label: string) => {
    const currentItems = useCartStore.getState().items
    if (currentItems.length > 0) {
      holdCart(`Ticket #${useCartStore.getState().heldCarts.length + 1}`)
    }
    resumeCart(id)
    setShowHeldPanel(false)
    toast.success("Ticket repris", { description: label })
    searchRef.current?.focus()
  }, [holdCart, resumeCart])

  const removeLastItem = useCallback(() => {
    const items = useCartStore.getState().items
    if (items.length > 0) {
      const last = items[items.length - 1]
      removeItem(last.article_id, last.variante_id)
    }
  }, [removeItem])

  const resolveAndAddByBarcode = useCallback(async (code: string): Promise<boolean> => {
    if (!code) return false
    const exact = articlesRef.current.find((a) => a.code_barre === code)
    if (exact) {
      addToCart(exact)
      return true
    }
    try {
      const found = await invoke<{
        variante_id: number; article_id: number; taille: string | null; couleur: string | null
        stock_dedie: number; designation: string; prix_vente: number; tva: number; actif: boolean
      } | null>("find_variante_by_barcode", { code_barre: code })
      if (found && found.actif) {
        const articleShim: Article = {
          id: found.article_id,
          code_barre: null,
          designation: found.designation,
          prix_achat: 0,
          fournisseur_id: null,
          actif: found.actif,
          prix_vente: found.prix_vente,
          tva: found.tva,
          stock: found.stock_dedie,
          stock_alerte: null,
          categorie_id: null,
        }
        const varianteShim: ArticleVariante = {
          id: found.variante_id,
          taille: found.taille,
          couleur: found.couleur,
          code_barre: code,
          stock_dedie: found.stock_dedie,
        }
        addToCart(articleShim, varianteShim)
        return true
      }
    } catch {
      // Pas de variante trouvée non plus : on ne fait rien, comme pour un code-barres article inconnu.
    }
    return false
  }, [addToCart])

  const handleValidateSale = () => {
    if (cart.length === 0) return
    const montantRecu = paymentSplits.length > 0
      ? sommeDH(paymentSplits.map((p) => p.amount))
      : paymentMode === "especes" ? cashAmount : netAmount
    if (montantRecu < netAmount) {
      toast.error("Montant insuffisant", { description: `Il manque ${formatCurrency(sommeDH([netAmount, -montantRecu]))}` })
      return
    }
    setProcessing(true)
    createSaleMutation.mutate()
  }

  const cartRef = useRef(cart)
  cartRef.current = cart
  const currentSearch = useRef(search)
  currentSearch.current = search
  const articlesRef = useRef(articles)
  const lastReceiptRef = useRef<ReceiptData | null>(null)
  lastReceiptRef.current = lastReceipt

  const printLastReceipt = useCallback(() => {
    if (lastReceiptRef.current) {
      printViaTauri(lastReceiptRef.current)
    }
  }, [])

  const [generatingPdf, setGeneratingPdf] = useState(false)
  const generateLastReceiptPdf = useCallback(async () => {
    if (!lastReceiptRef.current) return
    setGeneratingPdf(true)
    try {
      const path = await saveFacturePdf(lastReceiptRef.current)
      toast.success(`PDF généré : ${path}`)
    } catch (err) {
      toast.error(`Échec de la génération du PDF : ${String(err)}`)
    } finally {
      setGeneratingPdf(false)
    }
  }, [])

  articlesRef.current = articles
  const handlerRef = useRef({ handleValidateSale, clearCart, removeLastItem, addToCart, handleHoldCart, resolveAndAddByBarcode })

  useEffect(() => {
    handlerRef.current = { handleValidateSale, clearCart, removeLastItem, addToCart, handleHoldCart, resolveAndAddByBarcode }
  }, [handleValidateSale, clearCart, removeLastItem, addToCart, handleHoldCart, resolveAndAddByBarcode])

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement
      const isInput = target.tagName === "INPUT" || target.tagName === "TEXTAREA" || target.tagName === "SELECT"

      switch (e.key) {
        case "F1":
          e.preventDefault()
          searchRef.current?.focus()
          searchRef.current?.select()
          break
        case "F2":
          e.preventDefault()
          handlerRef.current.clearCart()
          break
        case "F4":
          e.preventDefault()
          if (cartRef.current.length > 0) {
            handlerRef.current.removeLastItem()
          }
          searchRef.current?.focus()
          break
        case "F5":
          e.preventDefault()
          if (cartRef.current.length > 0) {
            handlerRef.current.handleValidateSale()
          }
          queryClient.invalidateQueries({ queryKey: ["articles"] })
          break
        case "F6":
          e.preventDefault()
          const cur = useCartStore.getState().paymentMode
          const idx = PAYMENT_CYCLE.indexOf(cur)
          setPaymentMode(PAYMENT_CYCLE[(idx + 1) % PAYMENT_CYCLE.length])
          break
        case "F7":
          e.preventDefault()
          handlerRef.current.handleHoldCart()
          break
        case "F8":
          e.preventDefault()
          setShowHeldPanel((prev) => !prev)
          break
        case "Escape":
          if (isInput && (e.target as HTMLInputElement).value) {
            return
          }
          setShowHeldPanel(false)
          searchRef.current?.focus()
          break
        case "Enter":
          if (!isInput && currentSearch.current.trim()) {
            e.preventDefault()
            handlerRef.current.resolveAndAddByBarcode(currentSearch.current.trim())
          }
          break
      }
    }

    window.addEventListener("keydown", handleKeyDown)
    return () => window.removeEventListener("keydown", handleKeyDown)
  }, [queryClient, setPaymentMode])

  const createSaleMutation = useMutation({
    mutationFn: async () => {
      const items = cart.map((i) => ({
        article_id: i.article_id,
        variante_id: i.variante_id || null,
        quantite: i.quantite,
        prix_unitaire: i.prix_unitaire,
        tva: i.tva,
        remise_ligne: i.remise_ligne || 0,
        note: i.note || null,
        prix_type: i.prix_type || "public",
      }))
      const isSplit = paymentSplits.length > 0
      return invoke<{ id: number; numero_facture: string }>("create_vente", {
        clientId: selectedClient,
        articles: items,
        remiseGlobalePct: discount,
        modePaiement: isSplit ? paymentSplits.map((s) => s.mode).join("+") : paymentMode,
        splits: buildPaiements(paymentSplits, paymentMode, netAmount, loyaltyDiscount),
        dtype: documentType,
        pointsUtilises: ptsToUse,
        magasinId: currentSession?.magasin_id ?? null,
      })
    },
    onSuccess: ({ id: venteId, numero_facture: numeroFacture }) => {
      const clientObj = clients.find((c) => c.id === selectedClient)
      const clientName = clientObj?.nom || "Client de passage"
      const clientIce = clientObj?.ice || null
      const isSplit = paymentSplits.length > 0
      setLastReceipt({
        shopName: settings.shop_name || "SuperCaisse",
        shopAddress: settings.shop_address || "",
        shopPhone: settings.shop_phone || "",
        shopIce: settings.ice || null,
        shopIf: settings.if_number || null,
        shopRc: settings.rc_number || null,
        shopPatente: settings.patente || null,
        receiptFooter: settings.receipt_footer || "Merci de votre visite",
        logoBase64: settings.logo_base64 || null,
        docPrimaryColor: settings.doc_primary_color || null,
        receiptHeader: settings.receipt_header || null,
        venteId,
        docType: documentType,
        docNumero: numeroFacture,
        montantHT: totaux.montantHT,
        date: new Date().toISOString(),
        caissier: user?.nom || "",
        client: clientName,
        clientIce,
        items: cart.map((i, idx) => ({
          designation: i.designation,
          quantite: i.quantite,
          prix_unitaire: i.prix_unitaire,
          tva: i.tva,
          total_ligne: totaux.lignes[idx].total_ligne,
          montant_tva: totaux.lignes[idx].montant_tva,
          remise_ligne: i.remise_ligne || 0,
        })),
        montantTotal: totaux.montantTotal,
        montantRemise: totaux.montantRemise,
        netPaye: totaux.netTTC,
        modePaiement: buildPaiements(paymentSplits, paymentMode, netAmount, loyaltyDiscount)
          .map((p) => (isSplit || loyaltyDiscount > 0 ? `${p.mode} ${formatCurrency(p.montant)}` : p.mode))
          .join(" + "),
        monnaie: change,
      })

      // Libérer la table si on est en mode restaurant
      const curTable = useCartStore.getState().activeTableId
      if (isRestaurant && curTable) {
        updateTableMutation.mutate({ id: curTable, statut: "libre", ticket_id: null })
      }

      toast.success(documentType.charAt(0).toUpperCase() + documentType.slice(1) + " enregistré(e)", {
        description: `Ref #${venteId} - ${formatCurrency(netAmount)}`,
        action: { label: "Imprimer", onClick: () => printLastReceipt() },
      })
      clearCart()
      queryClient.invalidateQueries({ queryKey: ["articles"] })
      queryClient.invalidateQueries({ queryKey: ["stats"] })
      queryClient.invalidateQueries({ queryKey: ["ventes"] })
      setLastSync(new Date())
    },
    onError: (error) => {
      toast.error("Erreur", { description: String(error) })
    },
    onSettled: () => setProcessing(false),
  })

  const handleBarcodeScan = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && search.trim()) {
      const code = search.trim()
      resolveAndAddByBarcode(code).then((found) => {
        if (found) setSearch("")
      })
    }
  }

  // --- Session Blocker ---
  if (!isSessionLoading && !currentSession) {
    return (
      <div className="flex h-screen w-full items-center justify-center bg-muted/30">
        <div className="bg-card p-8 rounded-xl shadow-xl max-w-sm w-full text-center space-y-6">
          <div className="bg-primary/10 w-16 h-16 rounded-full flex items-center justify-center mx-auto mb-4">
            <PlayCircle className="h-8 w-8 text-primary" />
          </div>
          <h2 className="text-2xl font-bold tracking-tight">Ouvrir la caisse</h2>
          <p className="text-muted-foreground text-sm">
            Déclarez votre fond de caisse et choisissez votre boutique pour commencer.
          </p>
          {magasins.length > 1 && (
            <div className="space-y-2 text-left">
              <label className="text-sm font-medium flex items-center gap-2">
                <Store className="h-4 w-4" />
                Boutique
              </label>
              <Select value={selectedMagasinId} onValueChange={setSelectedMagasinId}>
                <SelectTrigger className="h-12">
                  <SelectValue placeholder="Sélectionner une boutique" />
                </SelectTrigger>
                <SelectContent>
                  {magasins.map((m) => (
                    <SelectItem key={m.id} value={String(m.id)}>{m.nom}</SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          )}
          <div className="space-y-2 text-left">
            <label className="text-sm font-medium">Fond de caisse initial (DH)</label>
            <Input
              type="number"
              value={fondInitial}
              onChange={(e) => setFondInitial(e.target.value)}
              className="h-12 text-lg text-center"
              min="0"
              step="0.01"
              autoFocus
            />
          </div>
          <Button
            className="w-full h-12 text-lg"
            disabled={openSessionMutation.isPending || (magasins.length > 1 && !selectedMagasinId)}
            onClick={() => {
              if (user?.id) {
                const magasinId = selectedMagasinId ? parseInt(selectedMagasinId) : undefined
                openSessionMutation.mutate({ caissierId: user.id, fondInitial: parseFloat(fondInitial) || 0, magasinId })
              }
            }}
          >
            {openSessionMutation.isPending ? <Loader2 className="h-5 w-5 animate-spin mr-2" /> : null}
            Ouvrir la session
          </Button>
        </div>
      </div>
    )
  }

  return (
    <div className="flex h-screen w-full bg-background overflow-hidden">
      {/* Left Panel - Products */}
      <div className="flex flex-col flex-1 min-w-0 overflow-hidden">
        {estFiscal(documentType) && mentionsManquantes.length > 0 && (
          <div role="alert" className="flex-shrink-0 border-b border-amber-300 bg-amber-50 px-4 py-2 text-sm text-amber-900 dark:bg-amber-950 dark:text-amber-200">
            Mentions légales manquantes ({mentionsManquantes.join(", ")}) : aucune facture ne peut être émise. Renseignez-les dans Paramètres &gt; Général.
          </div>
        )}
        {/* Top Bar */}
        <div className="flex-shrink-0 border-b border-border bg-card">
          <div className="flex items-center gap-4 p-3 lg:px-6">
            <div className="relative flex-1 max-w-2xl">
              <Search className="absolute left-4 top-1/2 -translate-y-1/2 h-5 w-5 text-muted-foreground" />
              <Input
                ref={searchRef}
                type="text"
                placeholder="Scanner code-barres ou rechercher un produit..."
                value={search}
                onChange={(e) => setSearch(e.target.value)}
                onKeyDown={handleBarcodeScan}
                className="pl-12 pr-12 h-12 text-base bg-muted/50 border-none ring-1 ring-border focus:ring-2 focus:ring-primary/30 transition-all"
                autoComplete="off"
              />
              <Barcode className="absolute right-4 top-1/2 -translate-y-1/2 h-5 w-5 text-muted-foreground/40" />
            </div>
            <div className="hidden sm:flex items-center gap-2">
              {articlesFetching && (
                <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" />
              )}
              <Badge variant="outline" className="gap-1.5 text-xs">
                <Package className="h-3 w-3" />
                {articles.length} articles
              </Badge>
            </div>
            <Button
              variant="ghost"
              size="icon"
              className="relative"
              onClick={handleHoldCart}
              title="Mettre en attente (F7)"
            >
              <PauseCircle className="h-5 w-5" />
            </Button>
            <Button
              variant="ghost"
              size="icon"
              className="relative"
              onClick={() => setShowHeldPanel((p) => !p)}
              title="Tickets en attente (F8)"
            >
              <PlayCircle className="h-5 w-5" />
              {heldCarts.length > 0 && (
                <span className="absolute -top-1 -right-1 flex h-4 w-4 items-center justify-center rounded-full bg-primary text-[10px] font-bold text-primary-foreground">
                  {heldCarts.length}
                </span>
              )}
            </Button>
            <Button
              variant="ghost"
              size="icon"
              className="relative"
              onClick={() => setShowShortcuts(!showShortcuts)}
              title="Raccourcis clavier"
            >
              <Keyboard className="h-5 w-5" />
            </Button>
            {currentSession && (
              <Button
                variant="ghost"
                size="icon"
                onClick={() => setShowRapportX(true)}
                title="Rapport X (résumé session)"
              >
                <BarChart3 className="h-5 w-5" />
              </Button>
            )}
          </div>

          {/* Category Pills & Table Badge */}
          <div className="px-3 lg:px-6 pb-3 overflow-x-auto scrollbar-thin flex items-center gap-2">
            {isRestaurant && activeTableNom && (
              <Badge variant="outline" className="text-sm px-3 py-1.5 flex items-center gap-2 border-primary/50 shrink-0">
                <Coffee className="h-4 w-4 text-primary" />
                {activeTableNom}
              </Badge>
            )}
            <CategoryPills
              categories={categories}
              articles={articles}
              activeCategory={activeCategory}
              onCategoryChange={setActiveCategory}
            />
          </div>

          {/* Keyboard Shortcuts Bar */}
          {showShortcuts && (
            <div className="px-3 lg:px-6 pb-3 animate-fade-in">
              <div className="flex flex-wrap gap-2 p-3 rounded-lg bg-muted/50 border border-border">
                {SHORTCUTS.map((s) => (
                  <kbd key={s.key} className="flex items-center gap-1.5 px-2.5 py-1 text-xs font-medium rounded-md bg-background border shadow-sm">
                    <span className="text-primary font-semibold">{s.key}</span>
                    <span className="text-muted-foreground">{s.label}</span>
                  </kbd>
                ))}
              </div>
            </div>
          )}
        </div>

        {/* Products Grid */}
        <div className="flex-1 overflow-y-auto scrollbar-thin p-3 lg:p-6">
          <ProductGrid
            articles={filteredArticles}
            onAddToCart={addToCart}
          />
        </div>
      </div>

      {/* Right Panel - Cart */}
      <CartPanel
        articles={articles}
        clients={clients}
        processing={processing}
        lastReceipt={lastReceipt}
        subtotal={subtotal}
        totalTVA={totalTVA}
        netAmount={netAmount}
        discount={discount}
        discountAmount={discountAmount}
        cashAmount={cashAmount}
        change={change}
        itemCount={itemCount}
        onUpdateQuantity={updateQuantity}
        onRemoveItem={removeFromCart}
        onClearCart={clearCart}
        onValidateSale={handleValidateSale}
        onPrintLastReceipt={printLastReceipt}
        onGeneratePdf={generateLastReceiptPdf}
        generatingPdf={generatingPdf}
        documentType={documentType}
        setDocumentType={setDocumentType}
        isLoyaltyActive={isLoyaltyActive}
        ptsValueDH={ptsValueDH}
        ptsEarned={ptsEarned}
        loyaltyDiscount={loyaltyDiscount}
        ptsToUse={ptsToUse}
        isRestaurant={isRestaurant}
      />

      {/* Held Tickets Panel */}
      {showHeldPanel && (
        <div className="absolute inset-0 z-50 flex justify-end" onClick={() => setShowHeldPanel(false)}>
          <div
            className="relative w-80 h-full bg-card border-l border-border shadow-2xl flex flex-col animate-slide-in-right"
            onClick={(e) => e.stopPropagation()}
          >
            <div className="flex items-center justify-between p-4 border-b border-border">
              <div className="flex items-center gap-2">
                <PauseCircle className="h-5 w-5 text-primary" />
                <h2 className="font-semibold text-sm">Tickets en attente</h2>
                <Badge variant="secondary">{heldCarts.length}</Badge>
              </div>
              <Button variant="ghost" size="icon" onClick={() => setShowHeldPanel(false)}>
                <X className="h-4 w-4" />
              </Button>
            </div>

            <div className="flex gap-2 p-4">
              <Button variant="outline" size="sm" onClick={() => invoke("open_cash_drawer").catch(e => toast.error("Erreur tiroir", { description: String(e) }))}>
                <LockOpen className="h-4 w-4 mr-2" />
                Ouvrir Tiroir
              </Button>
            </div>

            <div className="flex-1 overflow-y-auto p-3 space-y-2">
              {heldCarts.length === 0 ? (
                <div className="text-center py-12 text-muted-foreground text-sm">
                  <PauseCircle className="h-8 w-8 mx-auto mb-3 opacity-30" />
                  Aucun ticket en attente
                </div>
              ) : (
                heldCarts.map((held) => (
                  <div
                    key={held.id}
                    className="rounded-lg border border-border bg-muted/30 p-3 space-y-2"
                  >
                    <div className="flex items-start justify-between gap-2">
                      <div className="min-w-0">
                        <p className="font-medium text-sm truncate">{held.label}</p>
                        <p className="text-xs text-muted-foreground">
                          {held.state.items.length} article{held.state.items.length > 1 ? "s" : ""} •{" "}
                          {formatCurrency(
                            held.state.items.reduce((s, i) => s + i.quantite * i.prix_unitaire, 0)
                          )}
                        </p>
                        <p className="text-xs text-muted-foreground/60">
                          {new Date(held.date).toLocaleTimeString("fr-MA", { hour: "2-digit", minute: "2-digit" })}
                        </p>
                      </div>
                      <Button
                        variant="ghost"
                        size="icon"
                        className="h-7 w-7 shrink-0 text-destructive hover:text-destructive"
                        onClick={() => deleteHeldCart(held.id)}
                        title="Supprimer"
                      >
                        <Trash2 className="h-3.5 w-3.5" />
                      </Button>
                    </div>
                    <div className="pt-1">
                      <p className="text-xs text-muted-foreground mb-1.5 font-medium">Articles :</p>
                      <div className="space-y-0.5 max-h-20 overflow-y-auto">
                        {held.state.items.map((item) => (
                          <div key={`${item.article_id}-${item.variante_id ?? "x"}`} className="flex justify-between text-xs text-muted-foreground">
                            <span className="truncate max-w-[140px]">{item.designation}</span>
                            <span className="shrink-0 ml-1">×{item.quantite}</span>
                          </div>
                        ))}
                      </div>
                    </div>
                    <Button
                      className="w-full h-8 text-xs gap-1.5"
                      onClick={() => handleResumeCart(held.id, held.label)}
                    >
                      <PlayCircle className="h-3.5 w-3.5" />
                      Reprendre ce ticket
                    </Button>
                  </div>
                ))
              )}
            </div>

            <div className="p-3 border-t border-border">
              <p className="text-xs text-muted-foreground text-center">
                F7 — Mettre en attente · F8 — Ouvrir ce panneau
              </p>
            </div>
          </div>
        </div>
      )}

      {/* Sélecteur de table (Restaurant) */}
      <Dialog open={isRestaurant && activeTableId === null && !isSessionLoading && !!currentSession} onOpenChange={() => {}}>
        <DialogContent className="max-w-4xl bg-muted/30">
          <DialogHeader>
            <DialogTitle className="text-2xl font-bold flex items-center justify-center gap-2">
              <Coffee className="h-6 w-6" />
              Plan de salle
            </DialogTitle>
          </DialogHeader>
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4 p-4 max-h-[70vh] overflow-y-auto">
            {tablesLoading ? (
              <div className="col-span-full flex justify-center py-10"><Loader2 className="h-8 w-8 animate-spin" /></div>
            ) : (
              tables.map(table => (
                <button
                  key={table.id}
                  onClick={() => handleSelectTable(table)}
                  className={`p-6 rounded-2xl shadow-sm border-2 text-center transition-all ${
                    table.statut === "occupee" 
                      ? "bg-destructive/10 border-destructive text-destructive hover:bg-destructive/20" 
                      : "bg-card border-border hover:border-primary/50 hover:shadow-md"
                  }`}
                >
                  <p className="font-bold text-lg mb-2">{table.nom}</p>
                  <Badge variant={table.statut === "occupee" ? "destructive" : "secondary"}>
                    {table.statut === "occupee" ? "Occupée" : "Libre"}
                  </Badge>
                </button>
              ))
            )}
          </div>
        </DialogContent>
      </Dialog>

      {/* Sélecteur de déclinaison (taille/couleur) */}
      <Dialog open={showVariantPicker} onOpenChange={setShowVariantPicker}>
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle>{variantPickerArticle?.designation} — choisir une déclinaison</DialogTitle>
          </DialogHeader>
          <div className="grid grid-cols-2 gap-2 max-h-[60vh] overflow-y-auto py-2">
            {pickerVariantesFetching ? (
              <div className="col-span-2 flex justify-center py-8">
                <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
              </div>
            ) : pickerVariantes.length === 0 ? (
              <p className="col-span-2 text-center text-sm text-muted-foreground py-8">
                Aucune déclinaison définie pour cet article. Ajoutez-en depuis la page Articles.
              </p>
            ) : (
              pickerVariantes.map((v) => {
                const label = [v.taille, v.couleur].filter(Boolean).join(" / ") || `#${v.id}`
                const epuise = v.stock_dedie <= 0
                return (
                  <button
                    key={v.id}
                    type="button"
                    disabled={epuise}
                    onClick={() => variantPickerArticle && addToCart(variantPickerArticle, v)}
                    className={cn(
                      "flex flex-col items-center justify-center gap-1 rounded-xl border p-4 transition-colors",
                      epuise
                        ? "opacity-50 cursor-not-allowed border-muted"
                        : "border-border hover:border-primary hover:bg-primary/5"
                    )}
                  >
                    <span className="font-semibold">{label}</span>
                    <span className={cn("text-xs", epuise ? "text-destructive" : "text-muted-foreground")}>
                      {epuise ? "Rupture" : `Stock: ${v.stock_dedie}`}
                    </span>
                  </button>
                )
              })
            )}
          </div>
        </DialogContent>
      </Dialog>

      <RapportXDialog
        open={showRapportX}
        onOpenChange={setShowRapportX}
        sessionId={currentSession?.id}
      />

      <Toaster />
    </div>
  )
}

function RapportXDialog({ open, onOpenChange, sessionId }: { open: boolean; onOpenChange: (v: boolean) => void; sessionId?: number }) {
  const { data: rapport, isLoading } = useQuery({
    queryKey: ["rapport_x", sessionId],
    queryFn: () => invoke<RapportX>("get_rapport_x", { sessionId }),
    enabled: open && !!sessionId,
  })

  const MODE_LABELS: Record<string, string> = {
    especes: "Espèces",
    carte: "Carte bancaire",
    cheque: "Chèque",
    mixte: "Mixte",
    credit: "Crédit",
    virement: "Virement",
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-md">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <BarChart3 className="h-5 w-5" />
            Rapport X — Résumé de session
          </DialogTitle>
        </DialogHeader>
        {isLoading ? (
          <div className="flex justify-center py-8">
            <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
          </div>
        ) : rapport ? (
          <div className="space-y-4">
            <div className="text-sm text-muted-foreground">
              Ouverture : {new Date(rapport.date_ouverture).toLocaleString("fr-MA")}
            </div>
            <div className="grid grid-cols-2 gap-3">
              <div className="rounded-lg border p-3 text-center">
                <p className="text-2xl font-bold text-primary">{formatCurrency(rapport.ca_total)}</p>
                <p className="text-xs text-muted-foreground">Chiffre d'affaires</p>
              </div>
              <div className="rounded-lg border p-3 text-center">
                <p className="text-2xl font-bold">{rapport.nb_ventes}</p>
                <p className="text-xs text-muted-foreground">Ventes</p>
              </div>
              <div className="rounded-lg border p-3 text-center">
                <p className="text-2xl font-bold">{rapport.nb_articles_vendus}</p>
                <p className="text-xs text-muted-foreground">Articles vendus</p>
              </div>
              <div className="rounded-lg border p-3 text-center">
                <p className="text-2xl font-bold">{formatCurrency(rapport.fond_initial)}</p>
                <p className="text-xs text-muted-foreground">Fond de caisse</p>
              </div>
            </div>
            {rapport.par_mode.length > 0 && (
              <div className="space-y-2">
                <p className="text-sm font-medium">Par mode de paiement</p>
                <div className="space-y-1">
                  {rapport.par_mode.map((m) => (
                    <div key={m.mode} className="flex items-center justify-between text-sm py-1 px-2 rounded bg-muted/50">
                      <span>{MODE_LABELS[m.mode] || m.mode}</span>
                      <span className="font-medium">{formatCurrency(m.total)} ({m.count})</span>
                    </div>
                  ))}
                </div>
              </div>
            )}
            <div className="flex items-center justify-between text-sm border-t pt-3">
              <span className="text-muted-foreground">Remises</span>
              <span className="font-medium text-amber-600">{formatCurrency(rapport.total_remises)}</span>
            </div>
            <div className="flex items-center justify-between text-sm">
              <span className="text-muted-foreground">Annulations</span>
              <span className="font-medium text-destructive">{rapport.nb_annulations}</span>
            </div>
          </div>
        ) : (
          <p className="text-center py-8 text-muted-foreground">Aucune donnée de session</p>
        )}
      </DialogContent>
    </Dialog>
  )
}
