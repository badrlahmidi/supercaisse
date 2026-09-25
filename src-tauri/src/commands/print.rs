use crate::db::*;
use crate::paths::{ensure_dir, AppDirs};
use crate::session::AuthState;
use base64::engine::general_purpose;
use base64::Engine;
use rusqlite::OptionalExtension;
use tauri::State;

fn lancer(commande: &mut std::process::Command, action: &str) -> Result<(), String> {
    let sortie = commande
        .output()
        .map_err(|e| format!("{} impossible : {}", action, e))?;
    if sortie.status.success() {
        Ok(())
    } else {
        let detail = String::from_utf8_lossy(&sortie.stderr).trim().to_string();
        log::warn!("{} en échec ({}) : {}", action, sortie.status, detail);
        Err(format!(
            "{} impossible ({}) : {}",
            action, sortie.status, detail
        ))
    }
}

#[tauri::command(async)]
pub fn print_ticket(auth: State<AuthState>, token: String, texte: String) -> Result<(), String> {
    let _me = auth.session(&token)?;
    let path = std::env::temp_dir().join("ticket_impression.txt");
    std::fs::write(&path, &texte).map_err(|e| format!("Erreur écriture ticket: {}", e))?;

    if cfg!(target_os = "windows") {
        let path_str = path.to_string_lossy().replace("'", "''");
        let ps = format!(
            "Start-Process -FilePath 'notepad.exe' -ArgumentList '/p', '{}' -WindowStyle Hidden -Wait",
            path_str
        );
        lancer(
            std::process::Command::new("powershell").args(["-NonInteractive", "-Command", &ps]),
            "Impression",
        )?;
    } else {
        lancer(
            std::process::Command::new("lp").arg(path.to_string_lossy().as_ref()),
            "Impression",
        )?;
    }
    Ok(())
}

#[derive(Debug, PartialEq)]
pub(crate) enum CibleImpression {
    Partage(String),
    Peripherique(String),
    Cups(String),
}

fn nom_simple_valide(nom: &str) -> bool {
    !nom.is_empty()
        && nom.len() <= 64
        && !nom.starts_with(['-', '.', ' '])
        && nom
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, ' ' | '_' | '-' | '.'))
}

pub(crate) fn valider_imprimante(nom: &str) -> Result<CibleImpression, String> {
    let nom = nom.trim();
    let invalide = || {
        format!(
            "Nom d'imprimante invalide : « {} » (lettres, chiffres, espace, - _ . uniquement)",
            nom
        )
    };
    if let Some(reste) = nom.strip_prefix(r"\\") {
        let (hote, partage) = reste.split_once('\\').ok_or_else(invalide)?;
        let hote_valide = !hote.is_empty()
            && hote
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'));
        if hote_valide && nom_simple_valide(partage) {
            return Ok(CibleImpression::Partage(nom.to_string()));
        }
        return Err(invalide());
    }
    if let Some(dev) = nom.strip_prefix("/dev/") {
        let dev = dev.strip_prefix("usb/").unwrap_or(dev);
        let valide = ["lp", "ttyUSB", "ttyACM", "ttyS"].iter().any(|p| {
            dev.strip_prefix(p).is_some_and(|n| {
                !n.is_empty() && n.len() <= 3 && n.chars().all(|c| c.is_ascii_digit())
            })
        });
        if valide {
            return Ok(CibleImpression::Peripherique(nom.to_string()));
        }
        return Err(invalide());
    }
    if nom_simple_valide(nom) {
        return Ok(if cfg!(target_os = "windows") {
            CibleImpression::Partage(format!(r"\\localhost\{}", nom))
        } else {
            CibleImpression::Cups(nom.to_string())
        });
    }
    Err(invalide())
}

#[tauri::command(async)]
pub fn print_escpos(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
    base64_data: String,
) -> Result<(), String> {
    auth.session(&token)?;
    imprimer_escpos(&db, base64_data)
}

fn imprimer_escpos(db: &DbState, base64_data: String) -> Result<(), String> {
    let printer_name: String = {
        let conn = db.conn.lock().map_err(|e| e.to_string())?;
        conn.query_row(
            "SELECT value FROM settings WHERE key = 'printer_name'",
            [],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?
        .unwrap_or_else(|| "POS-80".to_string())
    };
    let cible = valider_imprimante(&printer_name)?;

    let bytes = general_purpose::STANDARD
        .decode(base64_data)
        .map_err(|e| format!("Erreur de décodage base64: {}", e))?;

    match cible {
        CibleImpression::Partage(chemin) | CibleImpression::Peripherique(chemin) => {
            std::fs::write(&chemin, &bytes)
                .map_err(|e| format!("Erreur d'impression ESC/POS vers {}: {}", chemin, e))?;
        }
        CibleImpression::Cups(nom) => {
            let path =
                std::env::temp_dir().join(format!("ticket_escpos_{}.bin", std::process::id()));
            std::fs::write(&path, &bytes)
                .map_err(|e| format!("Erreur d'écriture du flux binaire: {}", e))?;
            let output = std::process::Command::new("lp")
                .arg("-d")
                .arg(&nom)
                .arg("-o")
                .arg("raw")
                .arg("--")
                .arg(&path)
                .output()
                .map_err(|e| format!("Erreur lp: {}", e))?;
            let _ = std::fs::remove_file(&path);
            if !output.status.success() {
                let err = String::from_utf8_lossy(&output.stderr);
                return Err(format!("Erreur d'impression ESC/POS: {}", err));
            }
        }
    }

    Ok(())
}

#[tauri::command(async)]
pub fn open_cash_drawer(
    db: State<DbState>,
    auth: State<AuthState>,
    token: String,
) -> Result<(), String> {
    auth.session(&token)?;
    let drawer_kick = vec![0x1B, 0x70, 0x00, 0x19, 0xFA];
    imprimer_escpos(&db, general_purpose::STANDARD.encode(&drawer_kick))
}

#[tauri::command(async)]
pub fn print_receipt(auth: State<AuthState>, token: String, data: String) -> Result<(), String> {
    let _me = auth.session(&token)?;
    let path = std::env::temp_dir().join("ticket_impression.html");
    std::fs::write(&path, &data).map_err(|e| format!("Erreur écriture ticket: {}", e))?;

    if cfg!(target_os = "windows") {
        let ps = format!(
            "Start-Process -FilePath '{}' -WindowStyle Normal -Wait",
            path.to_string_lossy().replace("'", "''")
        );
        lancer(
            std::process::Command::new("powershell").args(["-Command", &ps]),
            "Impression",
        )?;
    } else if cfg!(target_os = "macos") {
        lancer(
            std::process::Command::new("open").arg(path.to_string_lossy().as_ref()),
            "Impression",
        )?;
    } else {
        lancer(
            std::process::Command::new("xdg-open").arg(path.to_string_lossy().as_ref()),
            "Impression",
        )?;
    }
    Ok(())
}

#[tauri::command(async)]
pub fn save_document_pdf(
    dirs: State<AppDirs>,
    auth: State<AuthState>,
    token: String,
    base64_data: String,
    filename: String,
) -> Result<String, String> {
    auth.session(&token)?;
    let bytes = general_purpose::STANDARD
        .decode(&base64_data)
        .map_err(|e| format!("Erreur de décodage base64: {}", e))?;

    let safe_name: String = filename
        .chars()
        .filter(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | '.'))
        .collect();
    let safe_name = if safe_name.is_empty() {
        "document.pdf".to_string()
    } else {
        safe_name
    };
    let safe_name = if safe_name.to_lowercase().ends_with(".pdf") {
        safe_name
    } else {
        format!("{}.pdf", safe_name)
    };

    let docs_dir = dirs.pdf_documents();
    ensure_dir(&docs_dir)?;
    let doc_path = docs_dir.join(safe_name);
    std::fs::write(&doc_path, &bytes).map_err(|e| format!("Erreur d'écriture du PDF: {}", e))?;
    Ok(doc_path.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_noms_valides() {
        assert!(valider_imprimante("POS-80").is_ok());
        assert!(valider_imprimante("EPSON TM-T20II").is_ok());
        assert_eq!(
            valider_imprimante(r"\\caisse-02\POS 80").unwrap(),
            CibleImpression::Partage(r"\\caisse-02\POS 80".into())
        );
        assert_eq!(
            valider_imprimante("/dev/usb/lp0").unwrap(),
            CibleImpression::Peripherique("/dev/usb/lp0".into())
        );
        assert_eq!(
            valider_imprimante("/dev/ttyUSB1").unwrap(),
            CibleImpression::Peripherique("/dev/ttyUSB1".into())
        );
    }

    #[test]
    fn test_injections_refusees() {
        for nom in [
            r#"POS" & calc & ""#,
            "POS & powershell -enc AAAA",
            "POS | del *",
            "POS`whoami`",
            "$(reboot)",
            "-o raw",
            "..",
            r"\\localhost\..\..\Windows",
            r"\\host&calc\POS",
            "/dev/../home/user/.bashrc",
            "/dev/sda",
            "/dev/lp0; rm -rf /",
            "",
            "POS\nPOS",
        ] {
            assert!(valider_imprimante(nom).is_err(), "accepté à tort : {nom}");
        }
    }
}
