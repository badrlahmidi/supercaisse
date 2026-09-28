use super::outbox::cle_conflit_cloud;
use super::{SyncCredentials, SUPABASE_PUBLISHABLE_KEY, SUPABASE_URL};
use serde_json::Value;

pub trait SupabaseClient {
    async fn upsert(
        &self,
        creds: &SyncCredentials,
        table: &str,
        ligne: Value,
    ) -> Result<(), String>;

    async fn supprimer(
        &self,
        creds: &SyncCredentials,
        table: &str,
        filtres: &[(String, String)],
    ) -> Result<(), String>;

    async fn recuperer(
        &self,
        creds: &SyncCredentials,
        table: &str,
        depuis: Option<&str>,
        limite: i64,
    ) -> Result<Vec<Value>, String>;
}

pub struct ReqwestSupabaseClient {
    http: reqwest::Client,
    base_url: String,
    api_key: String,
}

impl ReqwestSupabaseClient {
    pub fn nouveau() -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url: SUPABASE_URL.to_string(),
            api_key: SUPABASE_PUBLISHABLE_KEY.to_string(),
        }
    }
}

impl Default for ReqwestSupabaseClient {
    fn default() -> Self {
        Self::nouveau()
    }
}

impl SupabaseClient for ReqwestSupabaseClient {
    async fn upsert(
        &self,
        creds: &SyncCredentials,
        table: &str,
        mut ligne: Value,
    ) -> Result<(), String> {
        if let Value::Object(objet) = &mut ligne {
            objet.insert("tenant_id".into(), Value::String(creds.tenant_id.clone()));
        }
        let url = format!(
            "{}/rest/v1/{}?on_conflict={}",
            self.base_url,
            table,
            cle_conflit_cloud(table)
        );
        let reponse = self
            .http
            .post(&url)
            .bearer_auth(&creds.access_token)
            .header("apikey", &self.api_key)
            .header("Prefer", "resolution=merge-duplicates,return=minimal")
            .json(&ligne)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !reponse.status().is_success() {
            let corps = reponse.text().await.unwrap_or_default();
            return Err(format!("upsert {} refusé : {}", table, corps));
        }
        Ok(())
    }

    async fn supprimer(
        &self,
        creds: &SyncCredentials,
        table: &str,
        filtres: &[(String, String)],
    ) -> Result<(), String> {
        let filtre = filtres
            .iter()
            .map(|(colonne, valeur)| format!("{}=eq.{}", colonne, valeur))
            .collect::<Vec<_>>()
            .join("&");
        let url = format!("{}/rest/v1/{}?{}", self.base_url, table, filtre);
        let reponse = self
            .http
            .delete(&url)
            .bearer_auth(&creds.access_token)
            .header("apikey", &self.api_key)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !reponse.status().is_success() {
            let corps = reponse.text().await.unwrap_or_default();
            return Err(format!("suppression {} refusée : {}", table, corps));
        }
        Ok(())
    }

    async fn recuperer(
        &self,
        creds: &SyncCredentials,
        table: &str,
        depuis: Option<&str>,
        limite: i64,
    ) -> Result<Vec<Value>, String> {
        let mut url = format!(
            "{}/rest/v1/{}?tenant_id=eq.{}&order=updated_at.asc&limit={}",
            self.base_url, table, creds.tenant_id, limite
        );
        if let Some(depuis) = depuis {
            url.push_str(&format!("&updated_at=gt.{}", depuis));
        }
        let reponse = self
            .http
            .get(&url)
            .bearer_auth(&creds.access_token)
            .header("apikey", &self.api_key)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !reponse.status().is_success() {
            let corps = reponse.text().await.unwrap_or_default();
            return Err(format!("récupération {} refusée : {}", table, corps));
        }
        reponse
            .json::<Vec<Value>>()
            .await
            .map_err(|e| e.to_string())
    }
}
