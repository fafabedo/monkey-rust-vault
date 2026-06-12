use async_trait::async_trait;
use bytes::Bytes;
use sha2::{Digest, Sha256};

use crate::{
    config::Config,
    crypto,
    error::{VaultError, VaultResult},
    models::BucketRow,
};
use super::{StorageDriver, UploadResult};

pub struct DropboxDriver {
    encryption_key: String,
}

impl DropboxDriver {
    pub fn new(config: &Config) -> Self {
        Self { encryption_key: config.vault_encryption_key.clone() }
    }

    fn full_path(bucket: &BucketRow, relative_path: &str) -> String {
        match &bucket.dropbox_root_path {
            Some(root) => format!("{}/{}", root.trim_end_matches('/'), relative_path),
            None       => format!("/{}", relative_path),
        }
    }
}

#[async_trait]
impl StorageDriver for DropboxDriver {
    async fn upload(
        &self,
        bucket:        &BucketRow,
        relative_path: &str,
        data:          Bytes,
        _mime_type:    &str,
    ) -> VaultResult<UploadResult> {
        let token_enc = bucket
            .dropbox_access_token_enc
            .as_deref()
            .ok_or_else(|| VaultError::Provider("Missing dropbox_access_token_enc".into()))?;
        let token = crypto::decrypt(token_enc, &self.encryption_key)?;

        let dest_path = Self::full_path(bucket, relative_path);

        let mut hasher = Sha256::new();
        hasher.update(&data);
        let checksum = hex::encode(hasher.finalize());
        let size = data.len() as u64;

        let api_arg = serde_json::json!({
            "path": dest_path,
            "mode": "overwrite",
            "autorename": false,
            "mute": false
        });

        let client = reqwest::Client::new();
        let resp: serde_json::Value = client
            .post("https://content.dropboxapi.com/2/files/upload")
            .header("Authorization", format!("Bearer {token}"))
            .header("Content-Type", "application/octet-stream")
            .header("Dropbox-API-Arg", api_arg.to_string())
            .body(data)
            .send()
            .await
            .map_err(|e| VaultError::Provider(e.to_string()))?
            .json()
            .await
            .map_err(|e| VaultError::Provider(e.to_string()))?;

        if resp.get("error").is_some() {
            return Err(VaultError::Provider(format!("Dropbox upload error: {resp}")));
        }

        let path_display = resp["path_display"]
            .as_str()
            .unwrap_or(&dest_path)
            .to_string();

        Ok(UploadResult {
            uri:          format!("drop://{}/{}", bucket.slug, relative_path),
            provider_ref: path_display,
            checksum:     Some(checksum),
            file_size:    size,
        })
    }

    async fn verify(&self, bucket: &BucketRow, relative_path: &str) -> VaultResult<bool> {
        let token_enc = bucket
            .dropbox_access_token_enc
            .as_deref()
            .ok_or_else(|| VaultError::Provider("Missing dropbox_access_token_enc".into()))?;
        let token = crypto::decrypt(token_enc, &self.encryption_key)?;

        let dest_path = Self::full_path(bucket, relative_path);

        let client = reqwest::Client::new();
        let resp = client
            .post("https://api.dropboxapi.com/2/files/get_metadata")
            .header("Authorization", format!("Bearer {token}"))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({ "path": dest_path }))
            .send()
            .await
            .map_err(|e| VaultError::Provider(e.to_string()))?;

        Ok(resp.status().is_success())
    }
}
