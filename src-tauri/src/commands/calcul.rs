pub(crate) const TOLERANCE_MONTANT: f64 = 0.011;

const CORRECTION_DEMI_CENTIME: f64 = 1e-6;

pub(crate) fn vers_centimes(x: f64) -> i64 {
    let centimes = x * 100.0;
    (centimes + centimes.signum() * CORRECTION_DEMI_CENTIME).round() as i64
}

pub(crate) fn en_dh(centimes: i64) -> f64 {
    centimes as f64 / 100.0
}

pub(crate) fn round2(x: f64) -> f64 {
    en_dh(vers_centimes(x))
}

pub(crate) fn somme_dh<I: IntoIterator<Item = f64>>(montants: I) -> f64 {
    en_dh(montants.into_iter().map(vers_centimes).sum())
}

pub(crate) fn montant_saisi(libelle: &str, valeur: f64) -> Result<f64, String> {
    if !valeur.is_finite() || valeur.abs() >= 1e12 {
        return Err(format!("{} invalide : {}", libelle, valeur));
    }
    Ok(round2(valeur))
}

pub(crate) fn montant_positif(libelle: &str, valeur: f64) -> Result<f64, String> {
    let m = montant_saisi(libelle, valeur)?;
    if m < 0.0 {
        return Err(format!("{} ne peut pas être négatif : {}", libelle, valeur));
    }
    Ok(m + 0.0)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct LigneCalculee {
    pub total_ligne: f64,
    pub montant_ht: f64,
    pub montant_tva: f64,
}

pub(crate) fn calculer_ligne(
    quantite: f64,
    prix_unitaire_ht: f64,
    tva: f64,
    remise_ligne: f64,
    remise_globale: f64,
) -> LigneCalculee {
    let brut_ht = round2(quantite * prix_unitaire_ht * (1.0 - remise_ligne / 100.0));
    let brut_tva = round2(brut_ht * tva / 100.0);
    let montant_ht = round2(
        quantite * prix_unitaire_ht * (1.0 - remise_ligne / 100.0) * (1.0 - remise_globale / 100.0),
    );
    let montant_tva = round2(montant_ht * tva / 100.0);
    LigneCalculee {
        total_ligne: round2(brut_ht + brut_tva),
        montant_ht,
        montant_tva,
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct TotauxDocument {
    pub montant_total: f64,
    pub montant_ht: f64,
    pub montant_tva: f64,
    pub net_ttc: f64,
    pub montant_remise: f64,
}

pub(crate) fn totaliser(lignes: &[LigneCalculee]) -> TotauxDocument {
    let total: i64 = lignes.iter().map(|l| vers_centimes(l.total_ligne)).sum();
    let ht: i64 = lignes.iter().map(|l| vers_centimes(l.montant_ht)).sum();
    let tva: i64 = lignes.iter().map(|l| vers_centimes(l.montant_tva)).sum();
    TotauxDocument {
        montant_total: en_dh(total),
        montant_ht: en_dh(ht),
        montant_tva: en_dh(tva),
        net_ttc: en_dh(ht + tva),
        montant_remise: en_dh(total - ht - tva),
    }
}

pub(crate) fn valider_pourcentage(libelle: &str, valeur: f64) -> Result<f64, String> {
    if !valeur.is_finite() || !(0.0..=100.0).contains(&valeur) {
        return Err(format!(
            "{} invalide : {} (attendu entre 0 et 100)",
            libelle, valeur
        ));
    }
    Ok(valeur)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arrondi_au_centime() {
        assert_eq!(round2(1.005), 1.01);
        assert_eq!(round2(-1.005), -1.01);
        assert_eq!(round2(3.0 * 8.335), 25.01);
        assert_eq!(round2(0.1 + 0.2), 0.3);
        assert_eq!(round2(2.675), 2.68);
        assert_eq!(round2(1.004999), 1.0);
        assert_eq!(vers_centimes(19.99), 1999);
        assert_eq!(vers_centimes(-0.015), -2);
    }

    #[test]
    fn test_sommes_exactes() {
        let dixiemes = std::iter::repeat_n(0.1, 1000);
        assert_ne!(dixiemes.clone().sum::<f64>(), 100.0);
        assert_eq!(somme_dh(dixiemes), 100.0);
        assert_eq!(somme_dh([0.1, 0.2, -0.3]), 0.0);
    }

    #[test]
    fn test_montants_saisis() {
        assert_eq!(montant_saisi("Montant", 10.456).unwrap(), 10.46);
        assert!(montant_saisi("Montant", f64::NAN).is_err());
        assert!(montant_saisi("Montant", f64::INFINITY).is_err());
        assert!(montant_positif("Montant", -1.0).is_err());
        assert_eq!(montant_positif("Montant", -0.001).unwrap(), 0.0);
    }

    #[test]
    fn test_ligne_simple() {
        let l = calculer_ligne(2.0, 100.0, 20.0, 0.0, 0.0);
        assert_eq!(
            l,
            LigneCalculee {
                total_ligne: 240.0,
                montant_ht: 200.0,
                montant_tva: 40.0
            }
        );
    }

    #[test]
    fn test_remise_ligne_appliquee_avant_tva() {
        let l = calculer_ligne(1.0, 100.0, 20.0, 10.0, 0.0);
        assert_eq!(
            l,
            LigneCalculee {
                total_ligne: 108.0,
                montant_ht: 90.0,
                montant_tva: 18.0
            }
        );
    }

    #[test]
    fn test_remise_globale_reduit_la_base_tva() {
        let l = calculer_ligne(1.0, 100.0, 20.0, 0.0, 10.0);
        assert_eq!(l.total_ligne, 120.0);
        assert_eq!(l.montant_ht, 90.0);
        assert_eq!(l.montant_tva, 18.0);
        let t = totaliser(&[l]);
        assert_eq!(
            t,
            TotauxDocument {
                montant_total: 120.0,
                montant_ht: 90.0,
                montant_tva: 18.0,
                net_ttc: 108.0,
                montant_remise: 12.0
            }
        );
    }

    #[test]
    fn test_taux_multiples_et_arrondis() {
        let lignes = [
            calculer_ligne(3.0, 3.33, 20.0, 0.0, 5.0),
            calculer_ligne(1.5, 12.49, 7.0, 15.0, 5.0),
            calculer_ligne(1.0, 9.99, 0.0, 0.0, 5.0),
        ];
        let t = totaliser(&lignes);
        assert_eq!(t.net_ttc, round2(t.montant_ht + t.montant_tva));
        assert_eq!(t.montant_remise, round2(t.montant_total - t.net_ttc));
        assert_eq!(
            t.montant_tva,
            round2(lignes.iter().map(|l| l.montant_tva).sum())
        );
        assert_eq!(
            lignes[0],
            LigneCalculee {
                total_ligne: 11.99,
                montant_ht: 9.49,
                montant_tva: 1.9
            }
        );
    }

    #[test]
    fn test_parite_avec_le_calcul_frontend() {
        let lignes = [
            calculer_ligne(3.0, 3.33, 20.0, 0.0, 5.0),
            calculer_ligne(1.5, 12.49, 7.0, 15.0, 5.0),
            calculer_ligne(1.0, 9.99, 0.0, 0.0, 5.0),
            calculer_ligne(7.0, 1.15, 10.0, 3.0, 5.0),
        ];
        assert_eq!(
            lignes[1],
            LigneCalculee {
                total_ligne: 17.03,
                montant_ht: 15.13,
                montant_tva: 1.06
            }
        );
        assert_eq!(
            lignes[3],
            LigneCalculee {
                total_ligne: 8.59,
                montant_ht: 7.42,
                montant_tva: 0.74
            }
        );
        assert_eq!(
            totaliser(&lignes),
            TotauxDocument {
                montant_total: 47.6,
                montant_ht: 41.53,
                montant_tva: 3.7,
                net_ttc: 45.23,
                montant_remise: 2.37,
            }
        );
    }

    #[test]
    fn test_pourcentages_bornes() {
        assert!(valider_pourcentage("Remise", -1.0).is_err());
        assert!(valider_pourcentage("Remise", 100.5).is_err());
        assert!(valider_pourcentage("Remise", f64::NAN).is_err());
        assert_eq!(valider_pourcentage("Remise", 12.5).unwrap(), 12.5);
    }
}
