use reqwest::{Client, RequestBuilder};

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
    }

    /// POST with `Prefer: return=representation` so the inserted row is returned.
    pub fn post(&self, table: &str) -> RequestBuilder {
        self.http
            .post(format!("{}/{}", self.rest_url, table))
            .header("apikey", &self.service_key)
            .header("Authorization", format!("Bearer {}", self.service_key))
            .header("Content-Type", "application/json")
            .header("Prefer", "return=representation")
    }

    /// POST with upsert on a named unique constraint — on conflict, resets the row.
    pub fn upsert(&self, table: &str, on_conflict: &str) -> RequestBuilder {
        self.http
            .post(format!("{}/{}", self.rest_url, table))
            .header("apikey", &self.service_key)
            .header("Authorization", format!("Bearer {}", self.service_key))
            .header("Content-Type", "application/json")
            .header("Prefer", "resolution=merge-duplicates,return=representation")
            .query(&[("on_conflict", on_conflict)])
    }

    pub fn patch(&self, table: &str) -> RequestBuilder {
        self.http
            .patch(format!("{}/{}", self.rest_url, table))
            .header("apikey", &self.service_key)
            .header("Authorization", format!("Bearer {}", self.service_key))
            .header("Content-Type", "application/json")
    }
}
