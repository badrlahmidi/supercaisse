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
        row_uuid: &str,
    ) -> Result<(), String>;
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
        let url = format!("{}/rest/v1/{}?on_conflict=id", self.base_url, table);
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
        row_uuid: &str,
    ) -> Result<(), String> {
        let url = format!("{}/rest/v1/{}?id=eq.{}", self.base_url, table, row_uuid);
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
}
