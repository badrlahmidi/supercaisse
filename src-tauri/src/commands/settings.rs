use crate::db::*;
use crate::session::{autoriser, Acces, AuthState};
use rusqlite::params;
use tauri::State;

use super::log_audit;

#[tauri::command(async)]
pub fn get_settings(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
) -> Result<Settings, String> {
    let conn = db.lecture()?;
    let _me = autoriser(&auth, &conn, &token, Acces::Connecte)?;
    let mut stmt = conn
        .prepare("SELECT key, value FROM settings")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| e.to_string())?;
    let mut map = std::collections::HashMap::new();
    for row in rows {
        let (k, v) = row.map_err(|e| e.to_string())?;
        map.insert(k, v);
    }
    Ok(Settings {
        shop_name: map
            .get("shop_name")
            .cloned()
            .unwrap_or_else(|| "SuperCaisse".to_string()),
        shop_address: map.get("shop_address").filter(|s| !s.is_empty()).cloned(),
        shop_phone: map.get("shop_phone").filter(|s| !s.is_empty()).cloned(),
        shop_email: map.get("shop_email").filter(|s| !s.is_empty()).cloned(),
        ice: map.get("ice").filter(|s| !s.is_empty()).cloned(),
        if_number: map.get("if_number").filter(|s| !s.is_empty()).cloned(),
        rc_number: map.get("rc_number").filter(|s| !s.is_empty()).cloned(),
        patente: map.get("patente").filter(|s| !s.is_empty()).cloned(),
        default_tva: map
            .get("default_tva")
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(20.0),
        receipt_footer: map.get("receipt_footer").filter(|s| !s.is_empty()).cloned(),
        currency: map
            .get("currency")
            .cloned()
            .unwrap_or_else(|| "MAD".to_string()),
        printer_name: map.get("printer_name").filter(|s| !s.is_empty()).cloned(),
        fidelite_actif: map.get("fidelite_actif").cloned(),
        autoriser_stock_negatif: map.get("autoriser_stock_negatif").cloned(),
        fidelite_dh_pour_1_point: map.get("fidelite_dh_pour_1_point").cloned(),
        fidelite_valeur_1_point: map.get("fidelite_valeur_1_point").cloned(),
        business_type: map.get("business_type").cloned(),
        idle_timeout: map.get("idle_timeout").cloned(),
        logo_base64: map.get("logo_base64").filter(|s| !s.is_empty()).cloned(),
        receipt_header: map.get("receipt_header").filter(|s| !s.is_empty()).cloned(),
        doc_primary_color: map
            .get("doc_primary_color")
            .filter(|s| !s.is_empty())
            .cloned(),
    })
}

#[tauri::command(async)]
pub fn update_settings(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    shop_name: String,
    shop_address: Option<String>,
    shop_phone: Option<String>,
    shop_email: Option<String>,
    ice: Option<String>,
    if_number: Option<String>,
    rc_number: Option<String>,
    patente: Option<String>,
    default_tva: f64,
    receipt_footer: Option<String>,
    currency: String,
    printer_name: Option<String>,
    business_type: Option<String>,
    fidelite_actif: Option<String>,
    fidelite_dh_pour_1_point: Option<String>,
    fidelite_valeur_1_point: Option<String>,
    idle_timeout: Option<String>,
    logo_base64: Option<String>,
    receipt_header: Option<String>,
    doc_primary_color: Option<String>,
    autoriser_stock_negatif: Option<String>,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let me = autoriser(&auth, &conn, &token, Acces::Module("settings", "modifier"))?;
    let pairs: Vec<(&str, String)> = vec![
        ("shop_name", shop_name),
        ("shop_address", shop_address.unwrap_or_default()),
        ("shop_phone", shop_phone.unwrap_or_default()),
        ("shop_email", shop_email.unwrap_or_default()),
        ("ice", ice.unwrap_or_default()),
        ("if_number", if_number.unwrap_or_default()),
        ("rc_number", rc_number.unwrap_or_default()),
        ("patente", patente.unwrap_or_default()),
        ("default_tva", default_tva.to_string()),
        ("receipt_footer", receipt_footer.unwrap_or_default()),
        ("currency", currency),
        ("printer_name", {
            let nom = printer_name
                .filter(|n| !n.trim().is_empty())
                .unwrap_or_else(|| "POS-80".to_string());
            super::print::valider_imprimante(&nom)?;
            nom.trim().to_string()
        }),
        (
            "business_type",
            business_type.unwrap_or_else(|| "standard".to_string()),
        ),
        (
            "fidelite_actif",
            fidelite_actif.unwrap_or_else(|| "true".to_string()),
        ),
        (
            "autoriser_stock_negatif",
            match autoriser_stock_negatif.as_deref() {
                Some("true") => "true".to_string(),
                _ => "false".to_string(),
            },
        ),
        (
            "fidelite_dh_pour_1_point",
            fidelite_dh_pour_1_point.unwrap_or_else(|| "100".to_string()),
        ),
        (
            "fidelite_valeur_1_point",
            fidelite_valeur_1_point.unwrap_or_else(|| "1".to_string()),
        ),
        (
            "idle_timeout",
            idle_timeout.unwrap_or_else(|| "300".to_string()),
        ),
        ("logo_base64", logo_base64.unwrap_or_default()),
        ("receipt_header", receipt_header.unwrap_or_default()),
        ("doc_primary_color", doc_primary_color.unwrap_or_default()),
    ];
    for (key, value) in pairs {
        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            params![key, value],
        ).map_err(|e| e.to_string())?;
    }
    log_audit(
        &conn,
        Some(me.user_id),
        "modifier_parametres",
        "Mise à jour des paramètres boutique",
        None,
        None,
    );
    Ok(())
}
