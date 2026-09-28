use super::{SUPABASE_PUBLISHABLE_KEY, SUPABASE_URL};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct SessionCloud {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
    pub user: UtilisateurCloud,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UtilisateurCloud {
    pub id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MagasinCloud {
    pub id: String,
    pub nom: String,
}

pub async fn connexion_par_mot_de_passe(
    email: &str,
    mot_de_passe: &str,
) -> Result<SessionCloud, String> {
    let client = reqwest::Client::new();
    let url = format!("{}/auth/v1/token?grant_type=password", SUPABASE_URL);
    let reponse = client
        .post(&url)
        .header("apikey", SUPABASE_PUBLISHABLE_KEY)
        .json(&serde_json::json!({ "email": email, "password": mot_de_passe }))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !reponse.status().is_success() {
        let corps = reponse.text().await.unwrap_or_default();
        return Err(format!("Connexion refusée par le cloud : {}", corps));
    }
    reponse
        .json::<SessionCloud>()
        .await
        .map_err(|e| e.to_string())
}

pub async fn lire_tenant_id(access_token: &str, user_id: &str) -> Result<String, String> {
    #[derive(Deserialize)]
    struct Ligne {
        tenant_id: String,
    }
    let client = reqwest::Client::new();
    let url = format!(
        "{}/rest/v1/profiles?select=tenant_id&id=eq.{}",
        SUPABASE_URL, user_id
    );
    let reponse = client
        .get(&url)
        .bearer_auth(access_token)
        .header("apikey", SUPABASE_PUBLISHABLE_KEY)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !reponse.status().is_success() {
        let corps = reponse.text().await.unwrap_or_default();
        return Err(format!("Profil cloud introuvable : {}", corps));
    }
    let lignes: Vec<Ligne> = reponse.json().await.map_err(|e| e.to_string())?;
    lignes
        .into_iter()
        .next()
        .map(|l| l.tenant_id)
        .ok_or_else(|| "Ce compte n'est associé à aucun tenant".to_string())
}

pub async fn lister_magasins(
    access_token: &str,
    tenant_id: &str,
) -> Result<Vec<MagasinCloud>, String> {
    let client = reqwest::Client::new();
    let url = format!(
        "{}/rest/v1/magasins?select=id,nom&tenant_id=eq.{}",
        SUPABASE_URL, tenant_id
    );
    let reponse = client
        .get(&url)
        .bearer_auth(access_token)
        .header("apikey", SUPABASE_PUBLISHABLE_KEY)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !reponse.status().is_success() {
        let corps = reponse.text().await.unwrap_or_default();
        return Err(format!(
            "Liste des boutiques cloud inaccessible : {}",
            corps
        ));
    }
    reponse
        .json::<Vec<MagasinCloud>>()
        .await
        .map_err(|e| e.to_string())
}
