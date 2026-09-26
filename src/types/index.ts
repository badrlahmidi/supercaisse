import type { ArticleCatalogue } from "@/types/generated/ArticleCatalogue"
import type { ComposantArticle } from "@/types/generated/ComposantArticle"
import type { LotArticle } from "@/types/generated/LotArticle"
import type { Magasin } from "@/types/generated/Magasin"
import type { TableRestaurant } from "@/types/generated/TableRestaurant"
import type { VarianteArticle } from "@/types/generated/VarianteArticle"
export interface User {
  id: number
  login: string
  nom: string
  role: "admin" | "manager" | "caissier"
  must_change_password?: boolean
}

export type Article = ArticleCatalogue

export type ArticleVariante = VarianteArticle

export type ArticleComposant = ComposantArticle

export type ArticleLot = LotArticle

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

export type TableResto = TableRestaurant

export type { Magasin }

export type Saisie<T> = { [K in keyof T]?: T[K] | null }
