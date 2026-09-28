use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum ModePaiement {
    Especes,
    Carte,
    Cb,
    Cheque,
    Virement,
    Credit,
    Fidelite,
    Mixte,
}

impl ModePaiement {
    pub fn code(self) -> &'static str {
        match self {
            ModePaiement::Especes => "especes",
            ModePaiement::Carte => "carte",
            ModePaiement::Cb => "cb",
            ModePaiement::Cheque => "cheque",
            ModePaiement::Virement => "virement",
            ModePaiement::Credit => "credit",
            ModePaiement::Fidelite => "fidelite",
            ModePaiement::Mixte => "mixte",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum TypeDocument {
    Facture,
    Bl,
    Devis,
    Commande,
    Avoir,
}

impl TypeDocument {
    pub fn code(self) -> &'static str {
        match self {
            TypeDocument::Facture => "facture",
            TypeDocument::Bl => "bl",
            TypeDocument::Devis => "devis",
            TypeDocument::Commande => "commande",
            TypeDocument::Avoir => "avoir",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum StatutVente {
    Validee,
    Annulee,
    Convertie,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum PrixType {
    #[default]
    Public,
    Grossiste,
}

impl PrixType {
    pub fn code(self) -> &'static str {
        match self {
            PrixType::Public => "public",
            PrixType::Grossiste => "grossiste",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export, optional_fields = nullable)]
pub struct LigneVenteSaisie {
    #[ts(as = "f64")]
    pub article_id: i64,
    #[ts(as = "Option<f64>")]
    pub variante_id: Option<i64>,
    pub quantite: f64,
    pub remise_ligne: Option<f64>,
    pub note: Option<String>,
    pub prix_type: Option<PrixType>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct PaiementSaisi {
    pub mode: ModePaiement,
    pub montant: f64,
}

#[derive(Debug, Clone, PartialEq, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct LigneAchatSaisie {
    #[ts(as = "f64")]
    pub article_id: i64,
    pub quantite: f64,
    pub prix_unitaire: f64,
}

#[derive(Debug, Clone, PartialEq, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct LigneTransfertSaisie {
    #[ts(as = "f64")]
    pub article_id: i64,
    pub quantite: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct VenteCreee {
    #[ts(as = "f64")]
    pub id: i64,
    pub numero_facture: String,
    pub montant_total: f64,
    pub montant_remise: f64,
    pub montant_ht: f64,
    pub montant_tva: f64,
    pub net_ttc: f64,
    pub points_gagnes: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct VenteResume {
    #[ts(as = "f64")]
    pub id: i64,
    pub date: String,
    #[ts(as = "Option<f64>")]
    pub client_id: Option<i64>,
    #[ts(as = "Option<f64>")]
    pub caissier_id: Option<i64>,
    pub montant_total: f64,
    pub montant_remise: f64,
    #[ts(as = "ModePaiement")]
    pub mode_paiement: String,
    #[ts(as = "StatutVente")]
    pub statut: String,
    pub numero_facture: Option<String>,
    pub client_nom: Option<String>,
    pub caissier_nom: Option<String>,
    #[ts(as = "TypeDocument")]
    pub dtype: String,
    pub client_telephone: Option<String>,
    pub client_email: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct VenteEntete {
    #[ts(as = "f64")]
    pub id: i64,
    pub date: String,
    pub montant_total: f64,
    pub montant_remise: f64,
    #[ts(as = "ModePaiement")]
    pub mode_paiement: String,
    #[ts(as = "StatutVente")]
    pub statut: String,
    pub numero_facture: Option<String>,
    pub client_nom: Option<String>,
    pub client_tel: Option<String>,
    pub caissier_nom: Option<String>,
    #[ts(as = "TypeDocument")]
    pub dtype: String,
    pub client_ice: Option<String>,
    #[ts(as = "Option<f64>")]
    pub source_vente_id: Option<i64>,
    #[ts(as = "Option<TypeDocument>")]
    pub source_dtype: Option<String>,
    pub source_numero: Option<String>,
    pub montant_ht: Option<f64>,
    pub montant_tva: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct LigneVenteDetail {
    #[ts(as = "f64")]
    pub id: i64,
    #[ts(as = "f64")]
    pub article_id: i64,
    pub designation: String,
    pub quantite: f64,
    pub prix_unitaire: f64,
    pub tva: f64,
    pub total_ligne: f64,
    pub remise_ligne: Option<f64>,
    pub montant_ht: Option<f64>,
    pub montant_tva: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct VenteDetail {
    pub vente: VenteEntete,
    pub lignes: Vec<LigneVenteDetail>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct Page<T> {
    pub lignes: Vec<T>,
    #[ts(as = "f64")]
    pub total: i64,
    #[ts(as = "f64")]
    pub page: i64,
    #[ts(as = "f64")]
    pub par_page: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct ListeVentes {
    #[serde(flatten)]
    #[ts(flatten)]
    pub page: Page<VenteResume>,
    pub chiffre_affaires: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct EcritureJournal {
    #[ts(as = "f64")]
    pub id: i64,
    pub date: String,
    #[ts(as = "Option<f64>")]
    pub utilisateur_id: Option<i64>,
    pub jtype: String,
    pub montant: f64,
    pub description: Option<String>,
    pub user_nom: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct ListeJournal {
    #[serde(flatten)]
    #[ts(flatten)]
    pub page: Page<EcritureJournal>,
    pub total_entrees: f64,
    pub total_sorties: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct MouvementStockLigne {
    #[ts(as = "f64")]
    pub id: i64,
    pub date: String,
    #[ts(as = "f64")]
    pub article_id: i64,
    pub designation: String,
    pub quantite: f64,
    pub mtype: String,
    #[ts(as = "Option<f64>")]
    pub reference_id: Option<i64>,
    pub reference_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct SessionSupervision {
    #[ts(as = "f64")]
    pub id: i64,
    #[ts(as = "f64")]
    pub caissier_id: i64,
    pub caissier_nom: Option<String>,
    pub magasin_nom: Option<String>,
    pub statut: String,
    pub date_ouverture: String,
    pub date_cloture: Option<String>,
    pub fond_initial: f64,
    pub recettes_especes: f64,
    pub recettes_cb: f64,
    pub recettes_cheque: f64,
    pub recettes_virement: f64,
    pub sorties: f64,
    pub entrees: f64,
    pub especes_attendu: Option<f64>,
    pub especes_declare: Option<f64>,
    pub ecart: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct SessionCaisse {
    #[ts(as = "f64")]
    pub id: i64,
    #[ts(as = "f64")]
    pub caissier_id: i64,
    pub date_ouverture: String,
    pub fond_initial: f64,
    pub statut: String,
    #[ts(as = "Option<f64>")]
    pub magasin_id: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct AchatResume {
    #[ts(as = "f64")]
    pub id: i64,
    pub date: String,
    #[ts(as = "Option<f64>")]
    pub fournisseur_id: Option<i64>,
    pub reference: Option<String>,
    pub montant_total: f64,
    pub statut: String,
    pub fournisseur_nom: Option<String>,
    pub statut_livraison: String,
    pub statut_paiement: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct EntreeAudit {
    #[ts(as = "f64")]
    pub id: i64,
    pub date: String,
    #[ts(as = "Option<f64>")]
    pub utilisateur_id: Option<i64>,
    pub action: String,
    pub detail: Option<String>,
    pub reference_type: Option<String>,
    #[ts(as = "Option<f64>")]
    pub reference_id: Option<i64>,
    pub user_nom: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct Cheque {
    #[ts(as = "f64")]
    pub id: i64,
    pub numero: String,
    pub banque: String,
    pub tireur: Option<String>,
    pub montant: f64,
    pub date_emission: String,
    pub date_echeance: String,
    pub statut: String,
    pub ctype: String,
    #[ts(as = "Option<f64>")]
    pub client_id: Option<i64>,
    #[ts(as = "Option<f64>")]
    pub fournisseur_id: Option<i64>,
    pub client_nom: Option<String>,
    pub fournisseur_nom: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct MouvementFidelite {
    #[ts(as = "f64")]
    pub id: i64,
    #[ts(as = "f64")]
    pub client_id: i64,
    #[ts(as = "Option<f64>")]
    pub vente_id: Option<i64>,
    pub points: f64,
    pub mtype: String,
    pub date: String,
    pub numero_facture: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct ComposantArticle {
    #[ts(as = "f64")]
    pub id: i64,
    #[ts(as = "f64")]
    pub composant_id: i64,
    pub designation: String,
    pub stock: f64,
    pub quantite: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct InventaireResume {
    #[ts(as = "f64")]
    pub id: i64,
    pub date_debut: String,
    pub date_fin: Option<String>,
    pub statut: String,
    #[ts(as = "f64")]
    pub magasin_id: i64,
    pub magasin_nom: String,
    pub utilisateur_nom: Option<String>,
    #[ts(as = "f64")]
    pub nb_articles: i64,
    #[ts(as = "f64")]
    pub nb_comptes: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct LotArticle {
    #[ts(as = "f64")]
    pub id: i64,
    pub numero_lot: Option<String>,
    pub date_peremption: Option<String>,
    pub quantite: f64,
    pub date_reception: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct LotPeremption {
    #[ts(as = "f64")]
    pub id: i64,
    #[ts(as = "f64")]
    pub article_id: i64,
    pub designation: String,
    pub numero_lot: Option<String>,
    pub date_peremption: String,
    pub quantite: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct Magasin {
    #[ts(as = "f64")]
    pub id: i64,
    pub nom: String,
    pub adresse: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct StatsMagasin {
    #[ts(as = "f64")]
    pub id: i64,
    pub nom: String,
    pub adresse: Option<String>,
    pub ca_mois: f64,
    #[ts(as = "f64")]
    pub nb_ventes: i64,
    #[ts(as = "f64")]
    pub nb_clients_actifs: i64,
    pub valeur_stock: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct TransfertResume {
    #[ts(as = "f64")]
    pub id: i64,
    pub date: String,
    pub statut: String,
    pub source_nom: String,
    pub dest_nom: String,
    pub utilisateur_nom: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct StockMagasin {
    #[ts(as = "f64")]
    pub id: i64,
    pub designation: String,
    pub code_barre: Option<String>,
    pub stock: f64,
    pub stock_alerte: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct PaiementClient {
    #[ts(as = "f64")]
    pub id: i64,
    #[ts(as = "f64")]
    pub client_id: i64,
    pub date: String,
    pub montant: f64,
    pub r#type: String,
    pub reference: Option<String>,
    pub client_nom: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct ArticleAlerte {
    #[ts(as = "f64")]
    pub id: i64,
    pub designation: String,
    pub stock: f64,
    pub stock_alerte: f64,
    pub categorie_nom: Option<String>,
    pub fournisseur_nom: Option<String>,
    #[ts(as = "Option<f64>")]
    pub fournisseur_id: Option<i64>,
    pub prix_achat: f64,
    #[ts(as = "f64")]
    pub suggestion_qte: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct TableRestaurant {
    #[ts(as = "f64")]
    pub id: i64,
    pub nom: String,
    pub statut: String,
    pub ticket_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct VarianteArticle {
    #[ts(as = "f64")]
    pub id: i64,
    pub taille: Option<String>,
    pub couleur: Option<String>,
    pub code_barre: Option<String>,
    pub stock_dedie: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct ArticleCatalogue {
    #[ts(as = "f64")]
    pub id: i64,
    pub code_barre: Option<String>,
    pub designation: String,
    pub prix_achat: f64,
    pub prix_vente: f64,
    pub tva: f64,
    pub stock: f64,
    pub stock_alerte: Option<f64>,
    #[ts(as = "Option<f64>")]
    pub categorie_id: Option<i64>,
    #[ts(as = "Option<f64>")]
    pub fournisseur_id: Option<i64>,
    pub actif: bool,
    pub image_url: Option<String>,
    pub categorie_nom: Option<String>,
    pub fournisseur_nom: Option<String>,
    pub suivi_lot: bool,
    pub prix_grossiste: Option<f64>,
    pub est_kit: bool,
    pub a_variantes: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct PrixFournisseur {
    #[ts(as = "f64")]
    pub fournisseur_id: i64,
    pub fournisseur_nom: String,
    pub prix_unitaire: f64,
    pub date: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct ComparaisonArticle {
    #[ts(as = "f64")]
    pub article_id: i64,
    pub designation: String,
    pub code_barre: Option<String>,
    pub fournisseurs: Vec<PrixFournisseur>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct PermissionRole {
    pub role: String,
    pub module: String,
    pub action: String,
    pub allowed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct VarianteScannee {
    #[ts(as = "f64")]
    pub variante_id: i64,
    #[ts(as = "f64")]
    pub article_id: i64,
    pub taille: Option<String>,
    pub couleur: Option<String>,
    pub stock_dedie: f64,
    pub designation: String,
    pub prix_vente: f64,
    pub tva: f64,
    pub actif: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct FichierSauvegarde {
    pub chemin: String,
    pub nom: String,
    #[ts(as = "f64")]
    pub taille: u64,
    pub date: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct RecettesPeriode {
    pub especes: f64,
    pub cb: f64,
    pub cheque: f64,
    pub virement: f64,
    pub total: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct Tresorerie {
    pub jour: RecettesPeriode,
    pub semaine: RecettesPeriode,
    pub mois: RecettesPeriode,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct VenteReleve {
    #[ts(as = "f64")]
    pub id: i64,
    pub date: String,
    pub numero_facture: Option<String>,
    pub montant_total: f64,
    pub montant_remise: f64,
    pub mode_paiement: String,
    pub statut: String,
    pub dtype: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct PaiementReleve {
    #[ts(as = "f64")]
    pub id: i64,
    pub date: String,
    pub montant: f64,
    pub r#type: String,
    pub reference: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct ReleveClient {
    #[ts(as = "f64")]
    pub client_id: i64,
    pub nom: String,
    pub credit_actuel: f64,
    pub credit_plafond: f64,
    pub ventes: Vec<VenteReleve>,
    pub paiements: Vec<PaiementReleve>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct LigneInventaire {
    #[ts(as = "f64")]
    pub id: i64,
    #[ts(as = "f64")]
    pub article_id: i64,
    pub designation: String,
    pub code_barre: Option<String>,
    pub stock_theorique: f64,
    pub stock_compte: Option<f64>,
    pub ecart: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct InventaireDetail {
    #[ts(as = "f64")]
    pub id: i64,
    pub date_debut: String,
    pub statut: String,
    #[ts(as = "f64")]
    pub magasin_id: i64,
    pub lignes: Vec<LigneInventaire>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct InventaireCree {
    #[ts(as = "f64")]
    pub id: i64,
    #[ts(as = "f64")]
    pub nb_articles: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct TotalParMode {
    pub mode: String,
    pub total: f64,
    #[ts(as = "f64")]
    pub count: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct RapportX {
    #[ts(as = "f64")]
    pub session_id: i64,
    pub date_ouverture: String,
    pub fond_initial: f64,
    #[ts(as = "f64")]
    pub nb_ventes: i64,
    pub ca_total: f64,
    pub total_remises: f64,
    #[ts(as = "f64")]
    pub nb_annulations: i64,
    pub nb_articles_vendus: f64,
    pub par_mode: Vec<TotalParMode>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct TopArticle {
    pub designation: String,
    pub quantite: f64,
    pub total: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct RotationArticle {
    #[ts(as = "f64")]
    pub id: i64,
    pub designation: String,
    pub stock_actuel: f64,
    pub quantite_vendue: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct VentesJour {
    pub jour: String,
    pub total: f64,
    #[ts(as = "f64")]
    pub nb: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct VentesParMode {
    pub mode: String,
    pub total: f64,
    #[ts(as = "f64")]
    pub nb: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct RapportDetaille {
    pub ca_total: f64,
    pub total_remises: f64,
    #[ts(as = "f64")]
    pub nb_ventes: i64,
    pub marge_brute: f64,
    pub tva_collectee: f64,
    pub top_articles: Vec<TopArticle>,
    pub rotation_stock: Vec<RotationArticle>,
    pub ventes_par_jour: Vec<VentesJour>,
    pub par_mode: Vec<VentesParMode>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct ArticleVendu {
    pub designation: String,
    pub quantite: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct MeilleurClient {
    pub nom: String,
    pub depense: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct MontantJour {
    pub jour: String,
    pub montant: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct StatsTableauDeBord {
    pub ca_7_jours: Vec<MontantJour>,
    pub total_ventes_30j: f64,
    #[ts(as = "f64")]
    pub nb_articles: i64,
    #[ts(as = "f64")]
    pub stock_alerte: i64,
    pub credit_total: f64,
    #[ts(as = "f64")]
    pub nb_clients: i64,
    pub ca_jour: f64,
    pub ca_mois: f64,
    pub benefice_mois: f64,
    pub top_articles: Vec<ArticleVendu>,
    pub top_clients: Vec<MeilleurClient>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct MagasinAppairageCloud {
    pub cloud_magasin_id: String,
    pub nom: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct MagasinLocalNonAssocie {
    #[ts(as = "f64")]
    pub id: i64,
    pub nom: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct ChoixAppairageCloud {
    pub magasins_cloud: Vec<MagasinAppairageCloud>,
    pub magasins_locaux: Vec<MagasinLocalNonAssocie>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct AssociationMagasinCloud {
    #[ts(as = "f64")]
    pub magasin_local_id: i64,
    pub cloud_magasin_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct EtatSynchroCloud {
    pub connecte: bool,
    pub tenant_id: Option<String>,
    pub cloud_magasin_id: Option<String>,
    #[ts(as = "f64")]
    pub en_attente: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_ligne_de_vente_stricte() {
        let ligne: LigneVenteSaisie =
            serde_json::from_value(json!({ "article_id": 1, "quantite": 2 })).unwrap();
        assert_eq!(ligne.prix_type, None);
        assert!(serde_json::from_value::<LigneVenteSaisie>(
            json!({ "article_id": 1, "quantite": 2, "remise_lign": 50 })
        )
        .is_err());
        assert!(serde_json::from_value::<LigneVenteSaisie>(json!({ "article_id": 1 })).is_err());
        assert!(serde_json::from_value::<LigneVenteSaisie>(
            json!({ "article_id": "1", "quantite": 2 })
        )
        .is_err());
        assert!(serde_json::from_value::<LigneVenteSaisie>(
            json!({ "article_id": 1, "quantite": 2, "prix_type": "vip" })
        )
        .is_err());
        assert!(serde_json::from_value::<LigneVenteSaisie>(
            json!({ "article_id": 1, "quantite": 2, "prix_unitaire": 0.01 })
        )
        .is_err());
    }

    #[test]
    fn test_paiements_et_lignes_stricts() {
        assert!(serde_json::from_value::<PaiementSaisi>(
            json!({ "mode": "Especes ", "montant": 1 })
        )
        .is_err());
        assert!(serde_json::from_value::<PaiementSaisi>(json!({ "mode": "especes" })).is_err());
        assert_eq!(
            serde_json::from_value::<PaiementSaisi>(json!({ "mode": "cb", "montant": 5 }))
                .unwrap()
                .mode,
            ModePaiement::Cb
        );
        assert!(serde_json::from_value::<LigneAchatSaisie>(
            json!({ "article_id": 1, "quantite": 2 })
        )
        .is_err());
        assert!(serde_json::from_value::<LigneTransfertSaisie>(
            json!({ "article_id": 1, "qte": 2 })
        )
        .is_err());
    }

    #[test]
    fn test_aucune_commande_non_typee() {
        let dossier = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/commands");
        let mut non_typees = Vec::new();
        for entree in std::fs::read_dir(dossier).unwrap() {
            let source = std::fs::read_to_string(entree.unwrap().path()).unwrap();
            let code = source.split("#[cfg(test)]").next().unwrap();
            for bloc in code.split("#[tauri::command").skip(1) {
                let signature = bloc.split('{').next().unwrap();
                if signature.contains("serde_json::Value") {
                    let nom = signature
                        .split("fn ")
                        .nth(1)
                        .and_then(|s| s.split('(').next())
                        .unwrap_or("?")
                        .to_string();
                    non_typees.push(nom);
                }
            }
        }
        assert!(
            non_typees.is_empty(),
            "Commandes utilisant serde_json::Value : typez-les dans contrats.rs. {:?}",
            non_typees
        );
    }

    #[test]
    fn test_codes_identiques_a_la_serialisation() {
        for mode in [
            ModePaiement::Especes,
            ModePaiement::Carte,
            ModePaiement::Cb,
            ModePaiement::Cheque,
            ModePaiement::Virement,
            ModePaiement::Credit,
            ModePaiement::Fidelite,
            ModePaiement::Mixte,
        ] {
            assert_eq!(serde_json::to_value(mode).unwrap(), json!(mode.code()));
            assert!(crate::db::MODES_PAIEMENT_VENTE.contains(&mode.code()));
        }
        for dtype in [
            TypeDocument::Facture,
            TypeDocument::Bl,
            TypeDocument::Devis,
            TypeDocument::Commande,
            TypeDocument::Avoir,
        ] {
            assert_eq!(serde_json::to_value(dtype).unwrap(), json!(dtype.code()));
            assert!(crate::db::TYPES_DOCUMENT.contains(&dtype.code()));
        }
        for prix in [PrixType::Public, PrixType::Grossiste] {
            assert_eq!(serde_json::to_value(prix).unwrap(), json!(prix.code()));
        }
    }
}
