interface User {
  id: number
  login: string
  nom: string
  role: "admin" | "manager" | "caissier"
}

interface Article {
  id: number
  code_barre: string | null
  designation: string
  image_url?: string | null
  prix_achat: number
  prix_vente: number
  tva: number
  stock: number
  stock_alerte: number | null
  categorie_id: number | null
  fournisseur_id: number | null
  actif: boolean
  categorie_nom?: string
  fournisseur_nom?: string
}

interface Client {
  id: number
  code: string | null
  nom: string
  adresse: string | null
  telephone: string | null
  email: string | null
  credit_plafond: number
  credit_actuel: number
}

interface Fournisseur {
  id: number
  nom: string
  adresse: string | null
  telephone: string | null
  ice: string | null
  email: string | null
}

interface Category {
  id: number
  nom: string
  description: string | null
}

interface Settings {
  shop_name: string
  shop_address: string | null
  shop_phone: string | null
  shop_email: string | null
  tax_number: string | null
  default_tva: number
  receipt_footer: string | null
  currency: string
}

interface SessionCaisse {
  id: number
  caissier_id: number
  date_ouverture: string
  fond_initial: number
  statut: string
}

interface TableResto {
  id: number
  nom: string
  statut: "libre" | "occupee"
  ticket_id: string | null
}

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

const mockClients: Client[] = [
  { id: 1, code: "CL001", nom: "Client de passage", adresse: null, telephone: null, email: null, credit_plafond: 0, credit_actuel: 0 },
]

const mockFournisseurs: Fournisseur[] = [
  { id: 1, nom: "Fournisseur Test", adresse: null, telephone: null, ice: null, email: null },
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
  tax_number: null,
  default_tva: 20,
  receipt_footer: "Merci de votre visite",
  currency: "MAD",
}

let nextId = 100
let mockCurrentSession: SessionCaisse | null = null

const mockData: Record<string, (args: Record<string, unknown>) => unknown> = {
  login: ({ login, password }) => {
    const user = mockUsers.find(u => u.login === login && password === "admin")
    return user || null
  },

  // Categories
  get_categories: () => mockCategories,
  add_category: ({ nom, description }) => { nextId++; mockCategories.push({ id: nextId, nom: nom as string, description: description as string | null }); return nextId },
  update_category: ({ id, nom, description }) => { const c = mockCategories.find(c => c.id === id); if (c) { c.nom = nom as string; c.description = description as string | null } },
  delete_category: ({ id }) => { const idx = mockCategories.findIndex(c => c.id === id); if (idx >= 0) mockCategories.splice(idx, 1) },

  // Fournisseurs
  get_fournisseurs: () => mockFournisseurs,
  add_fournisseur: (args) => { nextId++; mockFournisseurs.push({ id: nextId, ...args as Omit<Fournisseur, 'id'> }); return nextId },
  update_fournisseur: ({ id, ...rest }) => { const f = mockFournisseurs.find(f => f.id === id); if (f) Object.assign(f, rest) },
  delete_fournisseur: ({ id }) => { const idx = mockFournisseurs.findIndex(f => f.id === id); if (idx >= 0) mockFournisseurs.splice(idx, 1) },

  // Clients
  get_clients: () => mockClients,
  add_client: (args) => { nextId++; mockClients.push({ id: nextId, ...args as Omit<Client, 'id'> }); return nextId },
  update_client: ({ id, ...rest }) => { const c = mockClients.find(c => c.id === id); if (c) Object.assign(c, rest) },
  delete_client: ({ id }) => { const idx = mockClients.findIndex(c => c.id === id); if (idx >= 0) mockClients.splice(idx, 1) },

  // Articles
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

  // Ventes
  create_vente: () => { nextId++; return nextId },
  get_ventes: () => [],
  get_vente_details: () => ({ vente: { id: 1, date: new Date().toISOString(), montant_total: 0, montant_remise: 0, mode_paiement: "especes", statut: "validee", numero_facture: "FA-2026-00001", client_nom: "Client", caissier_nom: "Admin" }, lignes: [] }),

  // Achats
  create_achat: () => { nextId++; return nextId },
  get_achats: () => [],

  // Paiements
  get_paiements: () => [],
  add_paiement: () => { nextId++; return nextId },

  // Sessions caisse
  get_current_session: ({ caissierId }) =>
    mockCurrentSession?.caissier_id === caissierId && mockCurrentSession.statut === "ouverte" ? mockCurrentSession : null,
  open_session: ({ caissierId, fondInitial }) => {
    nextId++
    mockCurrentSession = {
      id: nextId,
      caissier_id: caissierId as number,
      date_ouverture: new Date().toISOString(),
      fond_initial: fondInitial as number,
      statut: "ouverte",
    }
    return nextId
  },
  close_session: () => {
    if (mockCurrentSession) mockCurrentSession = { ...mockCurrentSession, statut: "fermee" }
    return true
  },

  // Stats
  get_stats: () => ({
    total_ventes_30j: 0,
    nb_articles: mockArticles.length,
    stock_alerte: 0,
    credit_total: 0,
    nb_clients: mockClients.length,
  }),

  // Stock alerts
  get_articles_stock_alerte: () => [],

  // Journal
  get_journal_caisse: () => [],
  add_journal_caisse: () => { nextId++; return nextId },

  // Utilisateurs
  get_utilisateurs: () => [...mockUsers],
  add_utilisateur: (args) => { nextId++; mockUsers.push({ id: nextId, ...args as { login: string; nom: string; role: "admin" | "manager" | "caissier" } }); return nextId },
  update_utilisateur: ({ id, ...rest }) => { const u = mockUsers.find(u => u.id === id); if (u) Object.assign(u, rest) },
  delete_utilisateur: ({ id }) => { const idx = mockUsers.findIndex(u => u.id === id); if (idx >= 0) mockUsers.splice(idx, 1) },

  // Settings
  get_settings: () => ({ ...mockSettings }),
  update_settings: (args) => { Object.assign(mockSettings, args) },

  // Stock movements
  get_mouvements_stock: () => [],

  // Restaurant tables
  get_tables: () => mockTables,
  update_table_status: ({ id, statut, ticket_id }) => {
    const table = mockTables.find((t) => t.id === id)
    if (table) {
      table.statut = statut as "libre" | "occupee"
      table.ticket_id = ticket_id as string | null
    }
  },

  // Import
  import_articles_csv: ({ csvContent }) => {
    const lines = (csvContent as string).split("\n").filter(l => l.trim())
    return `Import terminé. ${Math.max(0, lines.length - 1)} articles importés.`
  },

  // Print
  print_receipt: () => true,
  print_ticket: () => true,

  // Backup
  backup_database: () => "backups/supercaisse_20240101_120000.db",
  export_database: () => "exports/supercaisse_export_20240101_120000.db",
  import_database: () => true,
}

export async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    const { invoke: tauriInvoke } = await import("@tauri-apps/api/core")
    return await tauriInvoke<T>(cmd, args)
  } catch {
    const handler = mockData[cmd]
    if (handler) {
      await new Promise(r => setTimeout(r, 200))
      return handler(args || {}) as T
    }
    const methods = Object.keys(mockData).join(", ")
    throw new Error(`Commande "${cmd}" non disponible en dehors de Tauri. Commandes mockées: ${methods}`)
  }
}

export type { User, Article, Client, Fournisseur, Category, Settings }
