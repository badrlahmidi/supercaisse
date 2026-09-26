use crate::db::*;
use crate::session::{autoriser, Acces, AuthState};
use rusqlite::{params, Connection, OptionalExtension};
use tauri::State;

use super::calcul::{en_dh, montant_saisi};
use super::contrats::{EcritureJournal, ListeJournal};

#[tauri::command(async)]
pub fn get_journal_caisse(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    debut: Option<String>,
    fin: Option<String>,
    sens: Option<String>,
    recherche: Option<String>,
    page: Option<i64>,
    par_page: Option<i64>,
) -> Result<ListeJournal, String> {
    let conn = db.lecture()?;
    let _me = autoriser(&auth, &conn, &token, Acces::Module("journal", "voir"))?;
    lister_journal(&conn, &debut, &fin, &sens, &recherche, page, par_page)
}

const SOURCE_JOURNAL: &str = "journal_caisse j LEFT JOIN utilisateurs u ON j.utilisateur_id = u.id";

pub(crate) fn lister_journal(
    conn: &Connection,
    debut: &Option<String>,
    fin: &Option<String>,
    sens: &Option<String>,
    recherche: &Option<String>,
    page: Option<i64>,
    par_page: Option<i64>,
) -> Result<ListeJournal, String> {
    let mut filtre = super::Filtre::new();
    filtre.periode("j.date", debut, fin);
    match sens.as_deref() {
        Some("entree") => filtre.clause.push_str(" AND j.jtype != 'sortie'"),
        Some("sortie") => filtre.clause.push_str(" AND j.jtype = 'sortie'"),
        _ => {}
    }
    if let Some(r) = recherche
        .as_ref()
        .map(|r| r.trim())
        .filter(|r| !r.is_empty())
    {
        let n = filtre.valeurs.len() + 1;
        filtre.ajouter(
            &format!(
                "(j.description LIKE ?{n} OR u.nom LIKE ?{n} OR j.jtype LIKE ?{n} OR CAST(j.id AS TEXT) LIKE ?{n})"
            ),
            format!("%{}%", r),
        );
    }
    let valeurs: Vec<&dyn rusqlite::types::ToSql> =
        filtre.valeurs.iter().map(|v| v.as_ref()).collect();
    let (entrees, sorties): (f64, f64) = conn
        .query_row(
            &format!(
                "SELECT COALESCE(SUM(CASE WHEN j.jtype != 'sortie' THEN ROUND(ABS(j.montant) * 100) END), 0),
                        COALESCE(SUM(CASE WHEN j.jtype = 'sortie' THEN ROUND(ABS(j.montant) * 100) END), 0)
                 FROM {} WHERE 1=1 {}",
                SOURCE_JOURNAL, filtre.clause
            ),
            valeurs.as_slice(),
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|e| e.to_string())?;
    let page = super::paginer(
        conn,
        "j.id, j.date, j.utilisateur_id, j.jtype, j.montant, j.description, u.nom as user_nom",
        SOURCE_JOURNAL,
        &filtre,
        "j.date DESC, j.id DESC",
        page,
        par_page,
        |row| {
            Ok(EcritureJournal {
                id: row.get(0)?,
                date: row.get(1)?,
                utilisateur_id: row.get(2)?,
                jtype: row.get(3)?,
                montant: row.get(4)?,
                description: row.get(5)?,
                user_nom: row.get(6)?,
            })
        },
    )?;
    Ok(ListeJournal {
        page,
        total_entrees: en_dh(entrees.round() as i64),
        total_sorties: en_dh(sorties.round() as i64),
    })
}

#[tauri::command(async)]
pub fn add_journal_caisse(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    jtype: String,
    montant: f64,
    description: Option<String>,
) -> Result<i64, String> {
    let mut conn = db.conn.lock().map_err(|e| e.to_string())?;
    let me = autoriser(&auth, &conn, &token, Acces::Module("journal", "creer"))?;
    let montant = montant_saisi("Montant", montant)?;
    super::valeur_autorisee("Type d'opération", &jtype, TYPES_JOURNAL)?;
    let utilisateur_id = Some(me.user_id);
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    let session_id: Option<i64> = if let Some(uid) = utilisateur_id {
        tx.query_row(
            "SELECT id FROM sessions_caisse WHERE caissier_id = ?1 AND statut = 'ouverte'",
            params![uid],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?
    } else {
        None
    };

    tx.execute(
        "INSERT INTO journal_caisse (utilisateur_id, jtype, montant, description, session_id) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![utilisateur_id, jtype, montant, description, session_id],
    ).map_err(|e| e.to_string())?;
    let id = tx.last_insert_rowid();
    tx.commit().map_err(|e| e.to_string())?;
    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_journal_pagine_avec_totaux() {
        let conn = crate::db::init_db(":memory:").unwrap();
        conn.execute_batch(
            "INSERT INTO journal_caisse (jtype, montant, description, date) VALUES
                ('encaissement', 120, 'Vente 1', '2026-09-20 10:00:00'),
                ('entree', 50, 'Appoint', '2026-09-20 11:00:00'),
                ('sortie', -15, 'Café', '2026-09-21 09:00:00'),
                ('sortie', 20, 'Transport', '2026-09-22 09:00:00')",
        )
        .unwrap();
        let tout = lister_journal(&conn, &None, &None, &None, &None, Some(0), Some(2)).unwrap();
        assert_eq!((tout.page.total, tout.page.lignes.len()), (4, 2));
        assert_eq!((tout.total_entrees, tout.total_sorties), (170.0, 35.0));
        assert_eq!(
            tout.page.lignes[0].description.as_deref(),
            Some("Transport")
        );

        let sorties = lister_journal(
            &conn,
            &Some("2026-09-21".into()),
            &None,
            &Some("sortie".into()),
            &None,
            None,
            None,
        )
        .unwrap();
        assert_eq!(
            (
                sorties.page.total,
                sorties.total_entrees,
                sorties.total_sorties
            ),
            (2, 0.0, 35.0)
        );

        let cafe =
            lister_journal(&conn, &None, &None, &None, &Some("caf".into()), None, None).unwrap();
        assert_eq!(cafe.page.total, 1);
    }
}
