const TAILLE_MAX_MESSAGE: usize = 4000;

pub(crate) fn tronquer(message: &str) -> String {
    let nettoye: String = message
        .chars()
        .map(|c| if c.is_control() && c != '\n' { ' ' } else { c })
        .collect();
    if nettoye.chars().count() <= TAILLE_MAX_MESSAGE {
        nettoye
    } else {
        let debut: String = nettoye.chars().take(TAILLE_MAX_MESSAGE).collect();
        format!("{}… (tronqué)", debut)
    }
}

#[tauri::command(async)]
pub fn journaliser_frontend(niveau: String, message: String) -> Result<(), String> {
    let message = tronquer(&message);
    match niveau.as_str() {
        "error" => log::error!(target: "frontend", "{}", message),
        "warn" => log::warn!(target: "frontend", "{}", message),
        _ => log::info!(target: "frontend", "{}", message),
    }
    Ok(())
}

pub fn installer_hook_panique() {
    let precedent = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let trace = std::backtrace::Backtrace::force_capture();
        log::error!("PANIQUE : {}\n{}", info, trace);
        log::logger().flush();
        precedent(info);
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_tronque_et_nettoye() {
        assert_eq!(tronquer("a\tb\nc"), "a b\nc");
        let long = "é".repeat(TAILLE_MAX_MESSAGE + 10);
        let t = tronquer(&long);
        assert!(t.ends_with("(tronqué)"));
        assert!(t.starts_with(&"é".repeat(TAILLE_MAX_MESSAGE)));
        assert!(!t.starts_with(&"é".repeat(TAILLE_MAX_MESSAGE + 1)));
    }
}
