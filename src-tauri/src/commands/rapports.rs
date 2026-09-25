use crate::db::*;
use rusqlite::params;
use tauri::State;
use crate::session::{autoriser, Acces, AuthState};

#[tauri::command]
pub fn get_rapport_x(db: State<DbState>, auth: State<AuthState>, token: String, session_id: i64) -> Result<serde_json::Value, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let me = autoriser(&auth, &conn, &token, Acces::Connecte)?;
    super::sessions::verifier_session_propre(&conn, &me, session_id, "voir")?;

    let (date_ouverture, fond_initial): (String, f64) = conn.query_row(
        "SELECT date_ouverture, fond_initial FROM sessions_caisse WHERE id = ?1",
        params![session_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    ).map_err(|_| "Session introuvable".to_string())?;

    let nb_ventes: i64 = conn.query_row(
        concat!("SELECT COUNT(*) FROM ventes v WHERE v.session_id = ?1 AND ", filtre_ca!()),
        params![session_id], |r| r.get(0),
    ).unwrap_or(0);

    let ca_total: f64 = conn.query_row(
        concat!("SELECT COALESCE(SUM(v.montant_total - v.montant_remise), 0) FROM ventes v WHERE v.session_id = ?1 AND ", filtre_ca!()),
        params![session_id], |r| r.get(0),
    ).unwrap_or(0.0);

    let total_remises: f64 = conn.query_row(
        concat!("SELECT COALESCE(SUM(v.montant_remise), 0) FROM ventes v WHERE v.session_id = ?1 AND ", filtre_ca!()),
        params![session_id], |r| r.get(0),
    ).unwrap_or(0.0);

    let nb_annulations: i64 = conn.query_row(
        "SELECT COUNT(*) FROM ventes WHERE session_id = ?1 AND statut = 'annulee'",
        params![session_id], |r| r.get(0),
    ).unwrap_or(0);

    let nb_articles_vendus: f64 = conn.query_row(
        concat!("SELECT COALESCE(SUM(", quantite_signee!("va"), "), 0) FROM vente_articles va JOIN ventes v ON v.id = va.vente_id WHERE v.session_id = ?1 AND ", filtre_ca!()),
        params![session_id], |r| r.get(0),
    ).unwrap_or(0.0);

    let mut stmt = conn.prepare(
        "SELECT vp.mode, COALESCE(SUM(vp.montant), 0), COUNT(DISTINCT vp.vente_id)
         FROM vente_paiements vp JOIN ventes v ON v.id = vp.vente_id
         WHERE vp.session_id = ?1 AND v.statut != 'annulee'
         GROUP BY vp.mode ORDER BY vp.mode"
    ).map_err(|e| e.to_string())?;
    let par_mode = stmt.query_map(params![session_id], |r| {
        Ok(serde_json::json!({
            "mode": r.get::<_, String>(0)?,
            "total": r.get::<_, f64>(1)?,
            "count": r.get::<_, i64>(2)?
        }))
    }).map_err(|e| e.to_string())?.filter_map(Result::ok).collect::<Vec<_>>();

    Ok(serde_json::json!({
        "session_id": session_id,
        "date_ouverture": date_ouverture,
        "fond_initial": fond_initial,
        "nb_ventes": nb_ventes,
        "ca_total": ca_total,
        "total_remises": total_remises,
        "nb_annulations": nb_annulations,
        "nb_articles_vendus": nb_articles_vendus,
        "par_mode": par_mode,
    }))
}

#[tauri::command]
pub fn get_rapport_detaille(db: State<DbState>, auth: State<AuthState>, token: String, debut: Option<String>, fin: Option<String>) -> Result<serde_json::Value, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let _me = autoriser(&auth, &conn, &token, Acces::Module("rapports", "voir"))?;

    let date_filter = |col: &str| -> (String, Vec<String>) {
        let mut clause = String::new();
        let mut params = Vec::new();
        if let Some(d) = &debut { if !d.is_empty() { clause.push_str(&format!(" AND {} >= ?", col)); params.push(d.clone()); } }
        if let Some(f) = &fin { if !f.is_empty() { clause.push_str(&format!(" AND {} <= ?", col)); params.push(format!("{} 23:59:59", f)); } }
        (clause, params)
    };

    let (wc, wp) = date_filter("v.date");
    let base_sql = format!(
        "SELECT COALESCE(SUM(v.montant_total - v.montant_remise), 0),
                COALESCE(SUM(v.montant_remise), 0),
                COUNT(*)
         FROM ventes v WHERE {} {}", filtre_ca!(), wc
    );
    let params_ref: Vec<&dyn rusqlite::types::ToSql> = wp.iter().map(|s| s as &dyn rusqlite::types::ToSql).collect();
    let (ca_total, total_remises, nb_ventes): (f64, f64, i64) = conn.query_row(
        &base_sql, params_ref.as_slice(),
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    ).unwrap_or((0.0, 0.0, 0));

    let (wc2, wp2) = date_filter("v.date");
    let marge_sql = format!(
        "SELECT COALESCE(SUM(COALESCE(va.montant_ht, va.quantite * va.prix_unitaire)
                    - va.quantite * a.prix_achat * (CASE WHEN va.total_ligne < 0 THEN -1 ELSE 1 END)), 0)
         FROM vente_articles va
         JOIN ventes v ON v.id = va.vente_id
         JOIN articles a ON a.id = va.article_id
         WHERE {} {}", filtre_ca!(), wc2
    );
    let params_ref2: Vec<&dyn rusqlite::types::ToSql> = wp2.iter().map(|s| s as &dyn rusqlite::types::ToSql).collect();
    let marge_brute: f64 = conn.query_row(&marge_sql, params_ref2.as_slice(), |r| r.get(0)).unwrap_or(0.0);

    let (wc3, wp3) = date_filter("v.date");
    let tva_sql = format!(
        "SELECT COALESCE(SUM(COALESCE(va.montant_tva, va.quantite * va.prix_unitaire * va.tva / 100.0)), 0)
         FROM vente_articles va
         JOIN ventes v ON v.id = va.vente_id
         WHERE {} {}", filtre_ca!(), wc3
    );
    let params_ref3: Vec<&dyn rusqlite::types::ToSql> = wp3.iter().map(|s| s as &dyn rusqlite::types::ToSql).collect();
    let tva_collectee: f64 = conn.query_row(&tva_sql, params_ref3.as_slice(), |r| r.get(0)).unwrap_or(0.0);

    let (wc4, wp4) = date_filter("v.date");
    let top_sql = format!(
        "SELECT a.designation, SUM(CASE WHEN v.dtype = 'avoir' THEN -va.quantite ELSE va.quantite END) as qty, SUM(va.total_ligne) as total
         FROM vente_articles va
         JOIN ventes v ON v.id = va.vente_id
         JOIN articles a ON a.id = va.article_id
         WHERE {} {}
         GROUP BY va.article_id ORDER BY qty DESC LIMIT 10", filtre_ca!(), wc4
    );
    let params_ref4: Vec<&dyn rusqlite::types::ToSql> = wp4.iter().map(|s| s as &dyn rusqlite::types::ToSql).collect();
    let mut stmt = conn.prepare(&top_sql).map_err(|e| e.to_string())?;
    let top_articles = stmt.query_map(params_ref4.as_slice(), |r| {
        Ok(serde_json::json!({
            "designation": r.get::<_, String>(0)?,
            "quantite": r.get::<_, f64>(1)?,
            "total": r.get::<_, f64>(2)?
        }))
    }).map_err(|e| e.to_string())?.filter_map(Result::ok).collect::<Vec<_>>();

    let (wc5, wp5) = date_filter("v.date");
    let rotation_sql = format!(
        "SELECT a.id, a.designation, a.stock, COALESCE(SUM(CASE WHEN v.id IS NOT NULL THEN (CASE WHEN v.dtype = 'avoir' THEN -va.quantite ELSE va.quantite END) ELSE 0 END), 0) as vendu
         FROM articles a
         LEFT JOIN vente_articles va ON va.article_id = a.id
         LEFT JOIN ventes v ON v.id = va.vente_id AND {} {}
         WHERE a.actif = 1
         GROUP BY a.id ORDER BY vendu DESC LIMIT 20", filtre_ca!(), wc5
    );
    let params_ref5: Vec<&dyn rusqlite::types::ToSql> = wp5.iter().map(|s| s as &dyn rusqlite::types::ToSql).collect();
    let mut stmt2 = conn.prepare(&rotation_sql).map_err(|e| e.to_string())?;
    let rotation_stock = stmt2.query_map(params_ref5.as_slice(), |r| {
        Ok(serde_json::json!({
            "id": r.get::<_, i64>(0)?,
            "designation": r.get::<_, String>(1)?,
            "stock_actuel": r.get::<_, f64>(2)?,
            "quantite_vendue": r.get::<_ , f64>(3)?
        }))
    }).map_err(|e| e.to_string())?.filter_map(Result::ok).collect::<Vec<_>>();

    let (wc6, wp6) = date_filter("v.date");
    let daily_sql = format!(
        "SELECT date(v.date) as jour, COALESCE(SUM(v.montant_total - v.montant_remise), 0) as ca, COUNT(*) as nb
         FROM ventes v WHERE {} {}
         GROUP BY jour ORDER BY jour", filtre_ca!(), wc6
    );
    let params_ref6: Vec<&dyn rusqlite::types::ToSql> = wp6.iter().map(|s| s as &dyn rusqlite::types::ToSql).collect();
    let mut stmt3 = conn.prepare(&daily_sql).map_err(|e| e.to_string())?;
    let ventes_par_jour = stmt3.query_map(params_ref6.as_slice(), |r| {
        Ok(serde_json::json!({
            "jour": r.get::<_, String>(0)?,
            "total": r.get::<_, f64>(1)?,
            "nb": r.get::<_, i64>(2)?
        }))
    }).map_err(|e| e.to_string())?.filter_map(Result::ok).collect::<Vec<_>>();

    let (wc7, wp7) = date_filter("v.date");
    let mode_sql = format!(
        "SELECT mode_paiement, COALESCE(SUM(montant_total - montant_remise), 0) as total, COUNT(*) as nb
         FROM ventes v WHERE {} {}
         GROUP BY mode_paiement", filtre_ca!(), wc7
    );
    let params_ref7: Vec<&dyn rusqlite::types::ToSql> = wp7.iter().map(|s| s as &dyn rusqlite::types::ToSql).collect();
    let mut stmt4 = conn.prepare(&mode_sql).map_err(|e| e.to_string())?;
    let par_mode = stmt4.query_map(params_ref7.as_slice(), |r| {
        Ok(serde_json::json!({
            "mode": r.get::<_, String>(0)?,
            "total": r.get::<_, f64>(1)?,
            "nb": r.get::<_, i64>(2)?
        }))
    }).map_err(|e| e.to_string())?.filter_map(Result::ok).collect::<Vec<_>>();

    Ok(serde_json::json!({
        "ca_total": ca_total,
        "total_remises": total_remises,
        "nb_ventes": nb_ventes,
        "marge_brute": marge_brute,
        "tva_collectee": tva_collectee,
        "top_articles": top_articles,
        "rotation_stock": rotation_stock,
        "ventes_par_jour": ventes_par_jour,
        "par_mode": par_mode,
    }))
}
