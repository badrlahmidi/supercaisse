import type { User, Article, ArticleComposant, ArticleLot, ArticleVariante, Client, Fournisseur, Category, Settings, TableResto, Magasin } from "@/types"
import type { SessionCaisse } from "@/types/generated/SessionCaisse"

const mockUsers: User[] = [
  { id: 1, login: "admin", nom: "Administrateur", role: "admin" },
  { id: 2, login: "manager", nom: "Gestionnaire", role: "manager" },
  { id: 3, login: "caissier", nom: "Caissier", role: "caissier" },
]

const mockCategories: Category[] = [
  { id: 1, nom: "Alimentation", description: null },
  { id: 2, nom: "Boissons", description: null },
  { id: 3, nom: "Entretien", description: null },
]

const mockArticles: Article[] = [
  { id: 1, code_barre: "123456789", designation: "Produit Test", prix_achat: 5, prix_vente: 10, tva: 20, stock: 100, stock_alerte: 10, categorie_id: 1, fournisseur_id: null, actif: true, categorie_nom: "Alimentation" },
]

const mockLots: Record<number, ArticleLot[]> = {}

const mockVariantes: Record<number, ArticleVariante[]> = {}
const mockComposants: Record<number, ArticleComposant[]> = {}

const mockClients: Client[] = [
  { id: 1, code: "CL001", nom: "Client de passage", adresse: null, telephone: null, email: null, ice: null, credit_plafond: 0, credit_actuel: 0, points_fidelite: 0, segment: null },
]

const mockFournisseurs: Fournisseur[] = [
  { id: 1, nom: "Fournisseur Test", adresse: null, telephone: null, ice: null, email: null },
]

const mockMagasins: Magasin[] = [
  { id: 1, nom: "Magasin Principal", adresse: "123 Rue Mohammed V, Casablanca" },
]

const mockTables: TableResto[] = [
  { id: 1, nom: "Table 1", statut: "libre", ticket_id: null },
  { id: 2, nom: "Table 2", statut: "libre", ticket_id: null },
]

const mockSettings: Settings = {
  shop_name: "SuperCaisse",
  shop_address: null,
  shop_phone: null,
  shop_email: null,
  ice: null,
  if_number: null,
  rc_number: null,
  patente: null,
  default_tva: 20,
  receipt_footer: "Merci de votre visite",
  currency: "MAD",
  printer_name: "POS-80",
  business_type: "standard",
  fidelite_actif: "true",
  autoriser_stock_negatif: "false",
  remise_max_caissier: "10",
  remise_max_manager: "100",
  fidelite_dh_pour_1_point: "100",
  fidelite_valeur_1_point: "1",
  idle_timeout: "300",
  logo_base64: null,
  receipt_header: null,
  doc_primary_color: null,
}

let nextId = 100
let mockCurrentSession: SessionCaisse | null = null

const mockData: Record<string, (args: Record<string, unknown>) => unknown> = {
  login: ({ login, password }) => {
    const user = mockUsers.find(u => u.login === login && password === "admin")
    return user || null
  },
  change_password: () => undefined,

  get_categories: () => mockCategories,
  add_category: ({ nom, description }) => { nextId++; mockCategories.push({ id: nextId, nom: nom as string, description: description as string | null }); return nextId },
  update_category: ({ id, nom, description }) => { const c = mockCategories.find(c => c.id === id); if (c) { c.nom = nom as string; c.description = description as string | null } },
  delete_category: ({ id }) => { const idx = mockCategories.findIndex(c => c.id === id); if (idx >= 0) mockCategories.splice(idx, 1) },

  get_fournisseurs: () => mockFournisseurs,
  add_fournisseur: (args) => { nextId++; mockFournisseurs.push({ id: nextId, ...args as Omit<Fournisseur, 'id'> }); return nextId },
  update_fournisseur: ({ id, ...rest }) => { const f = mockFournisseurs.find(f => f.id === id); if (f) Object.assign(f, rest) },
  delete_fournisseur: ({ id }) => { const idx = mockFournisseurs.findIndex(f => f.id === id); if (idx >= 0) mockFournisseurs.splice(idx, 1) },

  get_clients: () => mockClients,
  add_client: (args) => { nextId++; mockClients.push({ id: nextId, ...args as Omit<Client, 'id'> }); return nextId },
  update_client: ({ id, ...rest }) => { const c = mockClients.find(c => c.id === id); if (c) Object.assign(c, rest) },
  delete_client: ({ id }) => { const idx = mockClients.findIndex(c => c.id === id); if (idx >= 0) mockClients.splice(idx, 1) },

  get_articles: ({ recherche }) => {
    if (recherche) {
      const q = String(recherche).toLowerCase()
      return mockArticles.filter(a => a.designation.toLowerCase().includes(q) || a.code_barre?.includes(q))
    }
    return mockArticles
  },
  add_article: (args) => { nextId++; mockArticles.push({ id: nextId, ...args as Omit<Article, 'id'> }); return nextId },
  update_article: ({ id, ...rest }) => { const a = mockArticles.find(a => a.id === id); if (a) Object.assign(a, rest) },
  delete_article: ({ id }) => { const idx = mockArticles.findIndex(a => a.id === id); if (idx >= 0) mockArticles.splice(idx, 1) },
  update_article_stock: ({ article_id, quantite }) => { const a = mockArticles.find(a => a.id === article_id); if (a) a.stock += quantite as number },

  add_article_lot: ({ article_id, numero_lot, date_peremption, quantite }) => {
    nextId++
    const lot: ArticleLot = { id: nextId, numero_lot: (numero_lot as string) || null, date_peremption: (date_peremption as string) || null, quantite: quantite as number, date_reception: new Date().toISOString() }
    mockLots[article_id as number] = [...(mockLots[article_id as number] || []), lot]
    const a = mockArticles.find(a => a.id === article_id)
    if (a) a.stock += quantite as number
    return nextId
  },
  get_article_lots: ({ article_id }) => mockLots[article_id as number] || [],
  get_lots_peremption_proche: () => [],
  discard_article_lot: ({ lot_id, quantite }) => {
    for (const lots of Object.values(mockLots)) {
      const lot = lots.find((l) => l.id === lot_id)
      if (lot) lot.quantite -= quantite as number
    }
  },

  add_article_variante: ({ article_id, taille, couleur, code_barre, stock_initial }) => {
    nextId++
    const v: ArticleVariante = { id: nextId, taille: (taille as string) || null, couleur: (couleur as string) || null, code_barre: (code_barre as string) || null, stock_dedie: stock_initial as number }
    mockVariantes[article_id as number] = [...(mockVariantes[article_id as number] || []), v]
    return nextId
  },
  get_article_variantes: ({ article_id }) => mockVariantes[article_id as number] || [],
  update_article_variante: ({ id, taille, couleur, code_barre }) => {
    for (const list of Object.values(mockVariantes)) {
      const v = list.find((v) => v.id === id)
      if (v) { v.taille = (taille as string) || null; v.couleur = (couleur as string) || null; v.code_barre = (code_barre as string) || null }
    }
  },
  adjust_article_variante_stock: ({ id, quantite }) => {
    for (const list of Object.values(mockVariantes)) {
      const v = list.find((v) => v.id === id)
      if (v) v.stock_dedie += quantite as number
    }
  },
  delete_article_variante: ({ id }) => {
    for (const key of Object.keys(mockVariantes)) {
      mockVariantes[Number(key)] = mockVariantes[Number(key)].filter((v) => v.id !== id)
    }
  },
  find_variante_by_barcode: () => null,

  add_article_composant: ({ article_id, composant_id, quantite }) => {
    nextId++
    const composant = mockArticles.find((a) => a.id === composant_id)
    const c: ArticleComposant = { id: nextId, composant_id: composant_id as number, designation: composant?.designation || "Article", stock: composant?.stock || 0, quantite: quantite as number }
    mockComposants[article_id as number] = [...(mockComposants[article_id as number] || []), c]
    return nextId
  },
  get_article_composants: ({ article_id }) => mockComposants[article_id as number] || [],
  update_article_composant_quantite: ({ id, quantite }) => {
    for (const list of Object.values(mockComposants)) {
      const c = list.find((c) => c.id === id)
      if (c) c.quantite = quantite as number
    }
  },
  delete_article_composant: ({ id }) => {
    for (const key of Object.keys(mockComposants)) {
      mockComposants[Number(key)] = mockComposants[Number(key)].filter((c) => c.id !== id)
    }
  },

  create_vente: () => { nextId++; return { id: nextId, numero_facture: `FA-2026-${String(nextId).padStart(5, "0")}` } },
  get_ventes: () => ({ lignes: [], total: 0, page: 0, par_page: 50, chiffre_affaires: 0 }),
  get_vente_details: () => ({ vente: { id: 1, date: new Date().toISOString(), montant_total: 0, montant_remise: 0, mode_paiement: "especes", statut: "validee", numero_facture: "FA-2026-00001", client_nom: "Client", caissier_nom: "Admin", dtype: "facture", source_vente_id: null, source_dtype: null, source_numero: null }, lignes: [] }),
  convert_document: () => { nextId++; return nextId },

  create_achat: () => { nextId++; return nextId },
  get_achats: () => [],

  get_paiements: () => [],
  add_paiement: () => { nextId++; return nextId },

  get_current_session: ({ caissierId }) =>
    mockCurrentSession && mockCurrentSession.caissier_id === caissierId && mockCurrentSession.statut === "ouverte" ? mockCurrentSession : null,
  open_session: ({ caissierId, fondInitial }) => {
    nextId++
    mockCurrentSession = {
      id: nextId,
      caissier_id: caissierId as number,
      date_ouverture: new Date().toISOString(),
      fond_initial: fondInitial as number,
      magasin_id: null,
      statut: "ouverte",
    }
    return nextId
  },
  close_session: () => {
    if (mockCurrentSession) mockCurrentSession = { ...mockCurrentSession, statut: "fermee" }
    return true
  },

  get_stats: () => ({
    total_ventes_30j: 0,
    nb_articles: mockArticles.length,
    stock_alerte: 0,
    credit_total: 0,
    nb_clients: mockClients.length,
    ca_7_jours: [],
  }),

  get_articles_stock_alerte: () => [] as { id: number; designation: string; stock: number; stock_alerte: number; categorie_nom: string | null; fournisseur_nom: string | null; fournisseur_id: number | null; prix_achat: number; suggestion_qte: number }[],

  get_journal_caisse: () => ({ lignes: [], total: 0, page: 0, par_page: 100, total_entrees: 0, total_sorties: 0 }),
  add_journal_caisse: () => { nextId++; return nextId },

  get_utilisateurs: () => [...mockUsers],
  add_utilisateur: (args) => { nextId++; mockUsers.push({ id: nextId, ...args as { login: string; nom: string; role: "admin" | "manager" | "caissier" } }); return nextId },
  update_utilisateur: ({ id, ...rest }) => { const u = mockUsers.find(u => u.id === id); if (u) Object.assign(u, rest) },
  delete_utilisateur: ({ id }) => { const idx = mockUsers.findIndex(u => u.id === id); if (idx >= 0) mockUsers.splice(idx, 1) },

  get_settings: () => ({ ...mockSettings }),
  update_settings: (args) => { Object.assign(mockSettings, args) },

  get_mouvements_fidelite: () => [],
  get_rapport_x: () => ({ session_id: 1, date_ouverture: new Date().toISOString(), fond_initial: 0, nb_ventes: 0, ca_total: 0, total_remises: 0, nb_annulations: 0, nb_articles_vendus: 0, par_mode: [] }),
  get_releve_client: () => ({ client_id: 1, nom: "Client", credit_actuel: 0, credit_plafond: 0, ventes: [], paiements: [] }),
  get_audit_log: () => [],
  login_pin: () => null,
  get_comptes_pin: () => [],
  set_user_pin: () => {},
  get_rapport_detaille: () => ({ ca_total: 0, total_remises: 0, nb_ventes: 0, marge_brute: 0, tva_collectee: 0, top_articles: [], rotation_stock: [], ventes_par_jour: [], par_mode: [] }),
  create_inventaire: () => ({ id: 1, nb_articles: 0 }),
  get_inventaire: () => ({ id: 1, date_debut: new Date().toISOString(), statut: "en_cours", magasin_id: 1, lignes: [] }),
  get_inventaires: () => [],
  update_inventaire_ligne: () => {},
  valider_inventaire: () => {},

  get_permissions: () => [],
  update_permission: () => {},

  get_caisses: () => [],
  get_tresorerie: () => ({
    jour: { especes: 0, cb: 0, cheque: 0, virement: 0, total: 0 },
    semaine: { especes: 0, cb: 0, cheque: 0, virement: 0, total: 0 },
    mois: { especes: 0, cb: 0, cheque: 0, virement: 0, total: 0 },
  }),

  get_mouvements_stock: () => ({ lignes: [], total: 0, page: 0, par_page: 100 }),

  compare_fournisseur_prices: () => [],

  get_magasins: () => mockMagasins,
  add_magasin: ({ nom, adresse }) => { nextId++; mockMagasins.push({ id: nextId, nom: nom as string, adresse: (adresse as string) || null }); return nextId },
  update_magasin: ({ id, nom, adresse }) => { const m = mockMagasins.find(m => m.id === id); if (m) { m.nom = nom as string; m.adresse = (adresse as string) || null } },
  delete_magasin: ({ id }) => { if (mockMagasins.length <= 1) throw new Error("Impossible de supprimer le dernier magasin"); const idx = mockMagasins.findIndex(m => m.id === id); if (idx >= 0) mockMagasins.splice(idx, 1) },
  get_transferts: () => [],
  get_stock_par_magasin: () => [],

  get_tables: () => mockTables,
  update_table_status: ({ id, statut, ticket_id }) => {
    const table = mockTables.find((t) => t.id === id)
    if (table) {
      table.statut = statut as "libre" | "occupee"
      table.ticket_id = ticket_id as string | null
    }
  },

  import_articles_csv: ({ csvContent }) => {
    const lines = (csvContent as string).split("\n").filter(l => l.trim())
    return `Import terminé. ${Math.max(0, lines.length - 1)} articles importés.`
  },

  print_receipt: () => true,
  print_ticket: () => true,

  save_document_pdf: ({ filename }) => `documents/${filename || "document"}.pdf`,

  backup_database: () => "backups/supercaisse_20240101_120000.db",
  export_database: () => "exports/supercaisse_export_20240101_120000.db",
  import_database: () => "backups/avant_restauration_supercaisse.db",
  list_backups: () => [],
  verifier_mise_a_jour: () => ({ version_actuelle: __APP_VERSION__, configuree: false, disponible: null }),
  installer_mise_a_jour: () => null,
}

export async function mockInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const handler = mockData[cmd]
  if (!handler) {
    throw new Error(`Commande "${cmd}" non disponible en dehors de Tauri (aucun mock de développement)`)
  }
  await new Promise(r => setTimeout(r, 200))
  return handler(args || {}) as T
}
