use crate::db::*;
use crate::paths::{ensure_dir, AppDirs};
use base64::engine::general_purpose;
use base64::Engine;
use tauri::State;

#[tauri::command]
pub fn print_ticket(texte: String) -> Result<(), String> {
    let path = std::env::temp_dir().join("ticket_impression.txt");
    std::fs::write(&path, &texte).map_err(|e| format!("Erreur écriture ticket: {}", e))?;

    if cfg!(target_os = "windows") {
        let path_str = path.to_string_lossy().replace("'", "''");
        let ps = format!(
            "Start-Process -FilePath 'notepad.exe' -ArgumentList '/p', '{}' -WindowStyle Hidden -Wait",
            path_str
        );
        let _ = std::process::Command::new("powershell")
            .args(["-NonInteractive", "-Command", &ps])
            .output();
    } else {
        let _ = std::process::Command::new("lp")
            .arg(path.to_string_lossy().as_ref())
            .output();
    }
    Ok(())
}

#[tauri::command]
pub fn print_escpos(db: State<DbState>, base64_data: String) -> Result<(), String> {
    let printer_name: String = {
        let conn = db.conn.lock().map_err(|e| e.to_string())?;
        conn.query_row("SELECT value FROM settings WHERE key = 'printer_name'", [], |r| r.get(0))
            .unwrap_or_else(|_| "POS-80".to_string())
    };

    let bytes = general_purpose::STANDARD.decode(base64_data)
        .map_err(|e| format!("Erreur de décodage base64: {}", e))?;

    let path = std::env::temp_dir().join("ticket_escpos.bin");
    std::fs::write(&path, &bytes).map_err(|e| format!("Erreur d'écriture du flux binaire: {}", e))?;

    let path_str = path.to_string_lossy().to_string();

    if cfg!(target_os = "windows") {
        let printer_path = if printer_name.starts_with("\\\\") {
            printer_name.clone()
        } else {
            format!("\\\\localhost\\{}", printer_name)
        };
        let args = format!("COPY /B \"{}\" \"{}\"", path_str, printer_path);
        let output = std::process::Command::new("cmd")
            .args(["/c", &args])
            .output()
            .map_err(|e| e.to_string())?;
        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Erreur d'impression ESC/POS: {}", err));
        }
    } else if printer_name.starts_with("/dev/") {
        std::fs::write(&printer_name, &bytes)
            .map_err(|e| format!("Erreur écriture vers {}: {}", printer_name, e))?;
    } else {
        let output = std::process::Command::new("lp")
            .args(["-d", &printer_name, "-o", "raw", &path_str])
            .output()
            .map_err(|e| format!("Erreur lp: {}", e))?;
        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Erreur d'impression ESC/POS: {}", err));
        }
    }

    Ok(())
}

#[tauri::command]
pub fn open_cash_drawer(db: State<DbState>) -> Result<(), String> {
    let drawer_kick = vec![0x1B, 0x70, 0x00, 0x19, 0xFA];
    let base64_data = general_purpose::STANDARD.encode(&drawer_kick);
    print_escpos(db, base64_data)
}

#[tauri::command]
pub fn print_receipt(data: String) -> Result<(), String> {
    let path = std::env::temp_dir().join("ticket_impression.html");
    std::fs::write(&path, &data).map_err(|e| format!("Erreur écriture ticket: {}", e))?;

    if cfg!(target_os = "windows") {
        let ps = format!(
            "Start-Process -FilePath '{}' -WindowStyle Normal -Wait",
            path.to_string_lossy().replace("'", "''")
        );
        let _ = std::process::Command::new("powershell")
            .args(["-Command", &ps])
            .output();
    } else if cfg!(target_os = "macos") {
        let _ = std::process::Command::new("open")
            .arg(path.to_string_lossy().as_ref())
            .output();
    } else {
        let _ = std::process::Command::new("xdg-open")
            .arg(path.to_string_lossy().as_ref())
            .output();
    }
    Ok(())
}

#[tauri::command]
pub fn save_document_pdf(dirs: State<AppDirs>, base64_data: String, filename: String) -> Result<String, String> {
    let bytes = general_purpose::STANDARD.decode(&base64_data)
        .map_err(|e| format!("Erreur de décodage base64: {}", e))?;

    let safe_name: String = filename
        .chars()
        .filter(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | '.'))
        .collect();
    let safe_name = if safe_name.is_empty() { "document.pdf".to_string() } else { safe_name };
    let safe_name = if safe_name.to_lowercase().ends_with(".pdf") { safe_name } else { format!("{}.pdf", safe_name) };

    let docs_dir = dirs.pdf_documents();
    ensure_dir(&docs_dir)?;
    let doc_path = docs_dir.join(safe_name);
    std::fs::write(&doc_path, &bytes).map_err(|e| format!("Erreur d'écriture du PDF: {}", e))?;
    Ok(doc_path.to_string_lossy().to_string())
}
