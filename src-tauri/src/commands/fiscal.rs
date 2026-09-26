use rusqlite::{params, Connection, OptionalExtension};

pub(crate) const TYPES_FISCAUX: &[&str] = &["facture", "avoir"];
pub(crate) const SEGMENTS_PROFESSIONNELS: &[&str] = &["Professionnel", "Grossiste", "Revendeur"];
pub(crate) const DELAI_ANNULATION_MINUTES: i64 = 15;

pub(crate) fn est_fiscal(dtype: &str) -> bool {
    TYPES_FISCAUX.contains(&dtype)
}

fn chiffres(valeur: &str, min: usize, max: usize) -> bool {
    (min..=max).contains(&valeur.len()) && valeur.chars().all(|c| c.is_ascii_digit())
}

pub(crate) fn ice_valide(ice: &str) -> bool {
    chiffres(ice, 15, 15)
}

pub(crate) fn normaliser_ice(libelle: &str, ice: Option<String>) -> Result<Option<String>, String> {
    match ice.map(|v| v.split_whitespace().collect::<String>()) {
        Some(v) if v.is_empty() => Ok(None),
        Some(v) if ice_valide(&v) => Ok(Some(v)),
        Some(v) => Err(format!(
            "{} invalide : « {} » (15 chiffres attendus)",
            libelle, v
        )),
        None => Ok(None),
    }
}

pub(crate) fn normaliser_if(identifiant: Option<String>) -> Result<Option<String>, String> {
    match identifiant.map(|v| v.trim().to_string()) {
        Some(v) if v.is_empty() => Ok(None),
        Some(v) if chiffres(&v, 1, 15) => Ok(Some(v)),
        Some(v) => Err(format!(
            "Identifiant fiscal (IF) invalide : « {} » (chiffres uniquement)",
            v
        )),
        None => Ok(None),
    }
}

fn setting(conn: &Connection, cle: &str) -> Result<String, String> {
    Ok(conn
        .query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![cle],
            |r| r.get::<_, Option<String>>(0),
        )
        .optional()
        .map_err(|e| e.to_string())?
        .flatten()
        .unwrap_or_default()
        .trim()
        .to_string())
}

pub(crate) fn verifier_mentions_vendeur(conn: &Connection) -> Result<(), String> {
    let ice = setting(conn, "ice")?;
    let identifiant = setting(conn, "if_number")?;
    let rc = setting(conn, "rc_number")?;
    let mut manquantes = Vec::new();
    if !ice_valide(&ice) {
        manquantes.push("ICE (15 chiffres)");
    }
    if identifiant.is_empty() {
        manquantes.push("IF");
    }
    if rc.is_empty() {
        manquantes.push("RC");
    }
    if manquantes.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "Mentions légales manquantes pour émettre une facture : {}. Renseignez-les dans Paramètres > Général",
            manquantes.join(", ")
        ))
    }
}

pub(crate) fn verifier_ice_client(conn: &Connection, client_id: Option<i64>) -> Result<(), String> {
    let Some(id) = client_id else {
        return Ok(());
    };
    let client: Option<(String, Option<String>, Option<String>)> = conn
        .query_row(
            "SELECT nom, segment, ice FROM clients WHERE id = ?1",
            params![id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    let Some((nom, segment, ice)) = client else {
        return Ok(());
    };
    let professionnel = segment
        .as_deref()
        .is_some_and(|s| SEGMENTS_PROFESSIONNELS.contains(&s));
    if professionnel && !ice.as_deref().map(str::trim).is_some_and(ice_valide) {
        return Err(format!(
            "ICE du client « {} » obligatoire (15 chiffres) pour une facture professionnelle",
            nom
        ));
    }
    Ok(())
}

pub(crate) fn verifier_annulation_directe(
    conn: &Connection,
    vente_id: i64,
    dtype: &str,
    motif: Option<&str>,
) -> Result<(), String> {
    if !est_fiscal(dtype) {
        return Ok(());
    }
    if !motif.map(str::trim).is_some_and(|m| m.chars().count() >= 3) {
        return Err("Motif obligatoire pour annuler une facture ou un avoir".to_string());
    }
    let minutes: f64 = conn
        .query_row(
            "SELECT (julianday('now', 'localtime') - julianday(date)) * 1440 FROM ventes WHERE id = ?1",
            params![vente_id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if minutes > DELAI_ANNULATION_MINUTES as f64 {
        return Err(format!(
            "Document émis il y a plus de {} minutes : l'annulation directe n'est plus possible. Émettez un avoir",
            DELAI_ANNULATION_MINUTES
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_db;

    #[test]
    fn test_formats_ice_et_if() {
        assert_eq!(
            normaliser_ice("ICE", Some(" 001 234567 000089 ".into())).unwrap(),
            Some("001234567000089".into())
        );
        assert_eq!(normaliser_ice("ICE", Some("".into())).unwrap(), None);
        assert!(normaliser_ice("ICE", Some("12345".into())).is_err());
        assert!(normaliser_ice("ICE", Some("00123456700008A".into())).is_err());
        assert_eq!(
            normaliser_if(Some(" 1234 ".into())).unwrap(),
            Some("1234".into())
        );
        assert!(normaliser_if(Some("IF-12".into())).is_err());
    }

    #[test]
    fn test_mentions_vendeur_obligatoires() {
        let conn = init_db(":memory:").unwrap();
        let err = verifier_mentions_vendeur(&conn).unwrap_err();
        assert!(
            err.contains("ICE") && err.contains("IF") && err.contains("RC"),
            "{err}"
        );
        conn.execute_batch(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('ice', '001234567000089'), ('if_number', '1234'), ('rc_number', 'RC 1');",
        )
        .unwrap();
        verifier_mentions_vendeur(&conn).unwrap();
    }

    #[test]
    fn test_ice_client_professionnel() {
        let conn = init_db(":memory:").unwrap();
        conn.execute_batch(
            "INSERT INTO clients (id, nom, segment) VALUES (1, 'Particulier', 'Particulier'), (2, 'Grossiste SARL', 'Grossiste');",
        )
        .unwrap();
        verifier_ice_client(&conn, None).unwrap();
        verifier_ice_client(&conn, Some(1)).unwrap();
        assert!(verifier_ice_client(&conn, Some(2))
            .unwrap_err()
            .contains("Grossiste SARL"));
        conn.execute(
            "UPDATE clients SET ice = '001234567000089' WHERE id = 2",
            [],
        )
        .unwrap();
        verifier_ice_client(&conn, Some(2)).unwrap();
    }

    #[test]
    fn test_annulation_directe_limitee() {
        let conn = init_db(":memory:").unwrap();
        conn.execute_batch(
            "INSERT INTO ventes (id, montant_total, dtype) VALUES (1, 10, 'facture');
             INSERT INTO ventes (id, montant_total, dtype, date) VALUES (2, 10, 'facture', datetime('now', 'localtime', '-16 minutes'));
             INSERT INTO ventes (id, montant_total, dtype, date) VALUES (3, 10, 'devis', datetime('now', 'localtime', '-3 days'));",
        )
        .unwrap();
        assert!(verifier_annulation_directe(&conn, 1, "facture", None).is_err());
        assert!(verifier_annulation_directe(&conn, 1, "facture", Some(" ")).is_err());
        verifier_annulation_directe(&conn, 1, "facture", Some("Erreur de saisie")).unwrap();
        assert!(
            verifier_annulation_directe(&conn, 2, "facture", Some("Erreur de saisie"))
                .unwrap_err()
                .contains("Émettez un avoir")
        );
        verifier_annulation_directe(&conn, 3, "devis", None).unwrap();
    }
}
