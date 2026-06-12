use anyhow::{Context, Result};

pub struct Config {
    pub database_url:         String,
    pub vault_encryption_key: String,
    pub aws_default_region:   String,
    pub port:                 u16,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            database_url: std::env::var("DATABASE_URL")
                .context("DATABASE_URL must be set")?,
            vault_encryption_key: std::env::var("VAULT_ENCRYPTION_KEY")
                .context("VAULT_ENCRYPTION_KEY must be set")?,
            aws_default_region: std::env::var("AWS_DEFAULT_REGION")
                .unwrap_or_else(|_| "us-east-1".into()),
            port: std::env::var("PORT")
                .unwrap_or_else(|_| "4200".into())
                .parse()
                .context("PORT must be a valid number")?,
        })
    }
}
