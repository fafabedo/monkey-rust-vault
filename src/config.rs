use anyhow::{Context, Result};

pub struct Config {
    pub supabase_url:         String,
    pub supabase_service_key: String,
    pub vault_encryption_key: String,
    pub aws_default_region:   String,
    pub bind_addr:            String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            supabase_url: std::env::var("SUPABASE_URL")
                .context("SUPABASE_URL must be set")?,
            supabase_service_key: std::env::var("SUPABASE_SERVICE_KEY")
                .context("SUPABASE_SERVICE_KEY must be set")?,
            vault_encryption_key: std::env::var("VAULT_ENCRYPTION_KEY")
                .context("VAULT_ENCRYPTION_KEY must be set")?,
            aws_default_region: std::env::var("AWS_DEFAULT_REGION")
                .unwrap_or_else(|_| "us-east-1".into()),
            bind_addr: std::env::var("BIND_ADDR")
                .unwrap_or_else(|_| "127.0.0.1:4200".into()),
        })
    }
}
