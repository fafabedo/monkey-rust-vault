use reqwest::{Client, RequestBuilder};
use serde::Deserialize;
use uuid::Uuid;

use crate::error::{VaultError, VaultResult};

const SCHEMA: &str = "monkey_vault";

/// Thin wrapper around reqwest that attaches Supabase auth headers to every request.
#[derive(Clone)]
pub struct SupabaseClient {
    http:        Client,
    rest_url:    String,
    service_key: String,
}

impl SupabaseClient {
    pub fn new(supabase_url: &str, service_key: &str) -> Self {
        Self {
            http:        Client::new(),
            rest_url:    format!("{}/rest/v1", supabase_url.trim_end_matches('/')),
            service_key: service_key.to_string(),
        }
    }

    pub fn get(&self, table: &str) -> RequestBuilder {
        self.http
            .get(format!("{}/{}", self.rest_url, table))
            .header("apikey", &self.service_key)
            .header("Authorization", format!("Bearer {}", self.service_key))
            .header("Accept-Profile", SCHEMA)
    }

    /// POST with `Prefer: return=representation` so the inserted row is returned.
    pub fn post(&self, table: &str) -> RequestBuilder {
        self.http
            .post(format!("{}/{}", self.rest_url, table))
            .header("apikey", &self.service_key)
            .header("Authorization", format!("Bearer {}", self.service_key))
            .header("Content-Type", "application/json")
            .header("Content-Profile", SCHEMA)
            .header("Prefer", "return=representation")
    }

    /// POST with upsert on a named unique constraint — on conflict, resets the row.
    pub fn upsert(&self, table: &str, on_conflict: &str) -> RequestBuilder {
        self.http
            .post(format!("{}/{}", self.rest_url, table))
            .header("apikey", &self.service_key)
            .header("Authorization", format!("Bearer {}", self.service_key))
            .header("Content-Type", "application/json")
            .header("Content-Profile", SCHEMA)
            .header("Prefer", "resolution=merge-duplicates,return=representation")
            .query(&[("on_conflict", on_conflict)])
    }

    pub fn patch(&self, table: &str) -> RequestBuilder {
        self.http
            .patch(format!("{}/{}", self.rest_url, table))
            .header("apikey", &self.service_key)
            .header("Authorization", format!("Bearer {}", self.service_key))
            .header("Content-Type", "application/json")
            .header("Content-Profile", SCHEMA)
    }

    fn get_public(&self, table: &str) -> RequestBuilder {
        self.http
            .get(format!("{}/{}", self.rest_url, table))
            .header("apikey", &self.service_key)
            .header("Authorization", format!("Bearer {}", self.service_key))
    }

    /// Resolves a Supabase auth UUID (JWT `sub`) to the app-level `public.user.id` bigint.
    pub async fn get_user_bigint_id(&self, auth_uuid: Uuid) -> VaultResult<i64> {
        #[derive(Deserialize)]
        struct Row { id: i64 }

        let response = self.get_public("user")
            .query(&[
                ("user_id", format!("eq.{auth_uuid}")),
                ("select",  "id".into()),
                ("limit",   "1".into()),
            ])
            .send()
            .await?;

        if !response.status().is_success() {
            let err = response.text().await.unwrap_or_default();
            return Err(VaultError::Database(format!("get_user_bigint_id: {err}")));
        }

        response.json::<Vec<Row>>().await?
            .into_iter()
            .next()
            .map(|r| r.id)
            .ok_or_else(|| VaultError::Database(format!("user not found: {auth_uuid}")))
    }

    /// Resolves a space's external UUID to its internal `public.space.id` bigint.
    pub async fn get_space_bigint_id(&self, space_uuid: Uuid) -> VaultResult<i64> {
        #[derive(Deserialize)]
        struct Row { id: i64 }

        let response = self.get_public("space")
            .query(&[
                ("uuid",   format!("eq.{space_uuid}")),
                ("select", "id".into()),
                ("limit",  "1".into()),
            ])
            .send()
            .await?;

        if !response.status().is_success() {
            let err = response.text().await.unwrap_or_default();
            return Err(VaultError::Database(format!("get_space_bigint_id: {err}")));
        }

        response.json::<Vec<Row>>().await?
            .into_iter()
            .next()
            .map(|r| r.id)
            .ok_or_else(|| VaultError::Database(format!("space not found: {space_uuid}")))
    }

    /// Returns `Ok(())` if the user holds an active owner or editor permission for the space,
    /// otherwise `Err(VaultError::Forbidden)`.
    pub async fn check_space_permission(
        &self,
        space_bigint_id: i64,
        user_bigint_id:  i64,
    ) -> VaultResult<()> {
        let response = self.get_public("space_permission")
            .query(&[
                ("space_id", format!("eq.{space_bigint_id}")),
                ("user",     format!("eq.{user_bigint_id}")),
                ("active",   "eq.true".into()),
                ("or",       "(owner.eq.true,editor.eq.true)".into()),
                ("select",   "id".into()),
                ("limit",    "1".into()),
            ])
            .send()
            .await?;

        if !response.status().is_success() {
            let err = response.text().await.unwrap_or_default();
            return Err(VaultError::Database(format!("check_space_permission: {err}")));
        }

        let rows: Vec<serde_json::Value> = response.json().await?;
        if rows.is_empty() {
            return Err(VaultError::Forbidden);
        }
        Ok(())
    }
}
