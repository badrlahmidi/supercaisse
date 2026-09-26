use crate::db::*;
use crate::session::{autoriser, Acces, AuthState};
use tauri::State;

use super::contrats::{MouvementStockLigne, Page};

#[tauri::command(async)]
pub fn get_mouvements_stock(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    article_id: Option<i64>,
    mtype: Option<String>,
    debut: Option<String>,
    fin: Option<String>,
    page: Option<i64>,
    par_page: Option<i64>,
) -> Result<Page<MouvementStockLigne>, String> {
    let conn = db.lecture()?;
    let _me = autoriser(&auth, &conn, &token, Acces::Module("stock", "voir"))?;
    let mut filtre = super::Filtre::new();
    if let Some(aid) = article_id {
        filtre.ajouter("m.article_id = ?", aid);
    }
    if let Some(t) = mtype.filter(|t| !t.is_empty()) {
        filtre.ajouter("m.mtype = ?", t);
    }
    filtre.periode("m.date", &debut, &fin);
    super::paginer(
        &conn,
        "m.id, m.date, m.article_id, a.designation, m.quantite, m.mtype, m.reference_id, m.reference_type",
        "mouvements_stock m JOIN articles a ON m.article_id = a.id",
        &filtre,
        "m.date DESC, m.id DESC",
        page,
        par_page,
        |row| {
            Ok(MouvementStockLigne {
                id: row.get(0)?,
                date: row.get(1)?,
                article_id: row.get(2)?,
                designation: row.get(3)?,
                quantite: row.get(4)?,
                mtype: row.get(5)?,
                reference_id: row.get(6)?,
                reference_type: row.get(7)?,
            })
        },
    )
}

#[tauri::command(async)]
pub fn get_articles_stock_alerte(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
) -> Result<Vec<super::contrats::ArticleAlerte>, String> {
    let conn = db.lecture()?;
    let _me = autoriser(&auth, &conn, &token, Acces::Module("reappro", "voir"))?;
    let mut stmt = conn
        .prepare(
            "SELECT a.id, a.designation, a.stock, a.stock_alerte, c.nom as categorie_nom,
                f.nom as fournisseur_nom, a.fournisseur_id, a.prix_achat,
                CAST((a.stock_alerte * 2 - a.stock) AS INTEGER) as suggestion_qte
         FROM articles a
         LEFT JOIN categories c ON a.categorie_id = c.id
         LEFT JOIN fournisseurs f ON a.fournisseur_id = f.id
         WHERE a.actif=1 AND a.stock <= a.stock_alerte AND a.stock_alerte > 0
         ORDER BY (a.stock_alerte - a.stock) DESC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok(super::contrats::ArticleAlerte {
                id: row.get(0)?,
                designation: row.get(1)?,
                stock: row.get(2)?,
                stock_alerte: row.get(3)?,
                categorie_nom: row.get(4)?,
                fournisseur_nom: row.get(5)?,
                fournisseur_id: row.get(6)?,
                prix_achat: row.get(7)?,
                suggestion_qte: row.get(8)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}
