export interface User {
  id: number
  login: string
  nom: string
  role: "admin" | "manager" | "caissier"
  must_change_password?: boolean
}

export interface Article {
  id: number
  code_barre: string | null
  designation: string
  description?: string | null
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
  suivi_lot?: boolean
  prix_grossiste?: number | null
  est_kit?: boolean
  a_variantes?: boolean
}

export interface ArticleVariante {
  id: number
  taille: string | null
  couleur: string | null
  code_barre: string | null
  stock_dedie: number
}

export interface ArticleComposant {
  id: number
  composant_id: number
  designation: string
  stock: number
  quantite: number
}

export interface ArticleLot {
  id: number
  numero_lot: string | null
  date_peremption: string | null
  quantite: number
  date_reception: string
}

export interface Client {
  id: number
  code: string | null
  nom: string
  adresse: string | null
  telephone: string | null
  email: string | null
  ice: string | null
  credit_plafond: number
  credit_actuel: number
  points_fidelite: number
  segment: string | null
}

export interface Fournisseur {
  id: number
  nom: string
  adresse: string | null
  telephone: string | null
  ice: string | null
  email: string | null
}

export interface Category {
  id: number
  nom: string
  description: string | null
}

export interface Settings {
  shop_name: string
  shop_address: string | null
  shop_phone: string | null
  shop_email: string | null
  ice: string | null
  if_number: string | null
  rc_number: string | null
  patente: string | null
  default_tva: number
  receipt_footer: string | null
  currency: string
  printer_name: string | null
  business_type: string
  fidelite_actif: string
  autoriser_stock_negatif?: string
  remise_max_caissier?: string
  remise_max_manager?: string
  fidelite_dh_pour_1_point: string
  fidelite_valeur_1_point: string
  idle_timeout: string
  logo_base64: string | null
  receipt_header: string | null
  doc_primary_color: string | null
}

export interface TableResto {
  id: number
  nom: string
  statut: "libre" | "occupee"
  ticket_id: string | null
}

export interface Magasin {
  id: number
  nom: string
  adresse: string | null
}

export type Saisie<T> = { [K in keyof T]?: T[K] | null }
