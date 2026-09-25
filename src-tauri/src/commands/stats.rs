use crate::db::*;
use tauri::State;
use crate::session::{autoriser, Acces, AuthState};

#[tauri::command]
pub fn get_stats(db: State<DbState>, auth: State<AuthState>, token: String) -> Result<serde_json::Value, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let _me = autoriser(&auth, &conn, &token, Acces::Module("rapports", "voir"))?;

    let total_ventes_30j: f64 = conn.query_row(concat!("SELECT COALESCE(SUM(montant_total - montant_remise),0) FROM ventes v WHERE date >= datetime('now','-30 days','localtime') AND ", filtre_ca!()), [], |r| r.get(0)).unwrap_or(0.0);
    let nb_articles: i64 = conn.query_row("SELECT COUNT(*) FROM articles WHERE actif=1", [], |r| r.get(0)).unwrap_or(0);
    let stock_alerte: i64 = conn.query_row("SELECT COUNT(*) FROM articles WHERE stock <= stock_alerte AND stock_alerte > 0", [], |r| r.get(0)).unwrap_or(0);
    let credit_total: f64 = conn.query_row("SELECT COALESCE(SUM(credit_actuel),0) FROM clients", [], |r| r.get(0)).unwrap_or(0.0);
    let nb_clients: i64 = conn.query_row("SELECT COUNT(*) FROM clients", [], |r| r.get(0)).unwrap_or(0);

    let ca_jour: f64 = conn.query_row(concat!("SELECT COALESCE(SUM(montant_total - montant_remise),0) FROM ventes v WHERE date >= date('now','localtime') AND ", filtre_ca!()), [], |r| r.get(0)).unwrap_or(0.0);
    let ca_mois: f64 = conn.query_row(concat!("SELECT COALESCE(SUM(montant_total - montant_remise),0) FROM ventes v WHERE strftime('%Y-%m', date) = strftime('%Y-%m', 'now', 'localtime') AND ", filtre_ca!()), [], |r| r.get(0)).unwrap_or(0.0);

    let ht_mois: f64 = conn.query_row(concat!("SELECT COALESCE(SUM(COALESCE(montant_ht, montant_total - montant_remise)),0) FROM ventes v WHERE strftime('%Y-%m', date) = strftime('%Y-%m', 'now', 'localtime') AND ", filtre_ca!()), [], |r| r.get(0)).unwrap_or(0.0);

    let cout_achats_mois: f64 = conn.query_row(concat!("
        SELECT COALESCE(SUM(vl.quantite * a.prix_achat * (CASE WHEN vl.total_ligne < 0 THEN -1 ELSE 1 END)), 0)
        FROM vente_articles vl
        JOIN ventes v ON v.id = vl.vente_id
        JOIN articles a ON a.id = vl.article_id
        WHERE strftime('%Y-%m', v.date) = strftime('%Y-%m', 'now', 'localtime') AND ", filtre_ca!(), "
    "), [], |r| r.get(0)).unwrap_or(0.0);
    let benefice_mois = ht_mois - cout_achats_mois;

    let mut stmt = conn.prepare(concat!("
        SELECT a.designation, SUM(CASE WHEN v.dtype = 'avoir' THEN -vl.quantite ELSE vl.quantite END) as qte_vendue
        FROM vente_articles vl
        JOIN ventes v ON v.id = vl.vente_id
        JOIN articles a ON a.id = vl.article_id
        WHERE v.date >= datetime('now','-30 days','localtime') AND ", filtre_ca!(), "
        GROUP BY a.id
        ORDER BY qte_vendue DESC
        LIMIT 5
    ")).map_err(|e| e.to_string())?;
    let top_articles = stmt.query_map([], |r| {
        Ok(serde_json::json!({
            "designation": r.get::<_, String>(0)?,
            "quantite": r.get::<_, f64>(1)?
        }))
    }).map_err(|e| e.to_string())?.filter_map(Result::ok).collect::<Vec<_>>();

    let mut stmt = conn.prepare(concat!("
        SELECT c.nom, SUM(v.montant_total - v.montant_remise) as depense
        FROM ventes v
        JOIN clients c ON c.id = v.client_id
        WHERE v.date >= datetime('now','-30 days','localtime') AND ", filtre_ca!(), "
        GROUP BY c.id
        ORDER BY depense DESC
        LIMIT 5
    ")).map_err(|e| e.to_string())?;
    let top_clients = stmt.query_map([], |r| {
        Ok(serde_json::json!({
            "nom": r.get::<_, String>(0)?,
            "depense": r.get::<_, f64>(1)?
        }))
    }).map_err(|e| e.to_string())?.filter_map(Result::ok).collect::<Vec<_>>();

    Ok(serde_json::json!({
        "total_ventes_30j": total_ventes_30j,
        "nb_articles": nb_articles,
        "stock_alerte": stock_alerte,
        "credit_total": credit_total,
        "nb_clients": nb_clients,
        "ca_jour": ca_jour,
        "ca_mois": ca_mois,
        "benefice_mois": benefice_mois,
        "top_articles": top_articles,
        "top_clients": top_clients,
    }))
}
