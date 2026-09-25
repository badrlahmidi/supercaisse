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

    const COMMANDES_NON_TYPEES_MAX: usize = 29;

    #[test]
    fn test_aucune_nouvelle_commande_non_typee() {
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
            non_typees.len() <= COMMANDES_NON_TYPEES_MAX,
            "{} commandes utilisent serde_json::Value (maximum {}) : typez les nouvelles commandes dans contrats.rs. {:?}",
            non_typees.len(),
            COMMANDES_NON_TYPEES_MAX,
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
