use async_trait::async_trait;
use bytes::Bytes;
use chrono::Utc;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    config::Config,
    crypto,
    error::{VaultError, VaultResult},
    models::BucketRow,
};
use super::{StorageDriver, UploadResult};

pub struct GoogleDriveDriver {
    encryption_key: String,
}

impl GoogleDriveDriver {
    pub fn new(config: &Config) -> Self {
        Self { encryption_key: config.vault_encryption_key.clone() }
    }
}

#[derive(Serialize)]
struct JwtClaims {
    iss:   String,
    scope: String,
    aud:   String,
    iat:   i64,
    exp:   i64,
}

#[derive(Deserialize)]
struct ServiceAccountJson {
    client_email: String,
    private_key:  String,
    token_uri:    String,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
}

async fn fetch_access_token(sa_json: &str) -> VaultResult<String> {
    let sa: ServiceAccountJson = serde_json::from_str(sa_json)
        .map_err(|e| VaultError::Provider(format!("Invalid service account JSON: {e}")))?;

    let now = Utc::now().timestamp();
    let claims = JwtClaims {
        iss:   sa.client_email,
        scope: "https://www.googleapis.com/auth/drive".into(),
        aud:   sa.token_uri.clone(),
        iat:   now,
        exp:   now + 3600,
    };

    let key = EncodingKey::from_rsa_pem(sa.private_key.as_bytes())
        .map_err(|e| VaultError::Provider(format!("Invalid RSA private key: {e}")))?;

    let jwt = encode(&Header::new(Algorithm::RS256), &claims, &key)
        .map_err(|e| VaultError::Provider(format!("JWT encode failed: {e}")))?;

    let client = reqwest::Client::new();
    let resp: TokenResponse = client
        .post(&sa.token_uri)
        .form(&[
            ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
            ("assertion", jwt.as_str()),
        ])
        .send()
        .await
        .map_err(|e| VaultError::Provider(e.to_string()))?
        .json()
        .await
        .map_err(|e| VaultError::Provider(format!("Token response parse failed: {e}")))?;

    Ok(resp.access_token)
}

async fn multipart_upload(
    access_token: &str,
    folder_id:    Option<&str>,
    file_name:    &str,
    data:         Bytes,
    mime_type:    &str,
) -> VaultResult<String> {
    let metadata = match folder_id {
        Some(fid) => serde_json::json!({ "name": file_name, "parents": [fid] }),
        None      => serde_json::json!({ "name": file_name }),
    };

    let boundary = "vault_drive_boundary_7a9f3c";
    let meta_str = metadata.to_string();

    let mut body: Vec<u8> = Vec::new();
    body.extend_from_slice(format!("--{boundary}\r\nContent-Type: application/json; charset=UTF-8\r\n\r\n").as_bytes());
    body.extend_from_slice(meta_str.as_bytes());
    body.extend_from_slice(format!("\r\n--{boundary}\r\nContent-Type: {mime_type}\r\n\r\n").as_bytes());
    body.extend_from_slice(&data);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

    let client = reqwest::Client::new();
    let resp: serde_json::Value = client
        .post("https://www.googleapis.com/upload/drive/v3/files?uploadType=multipart")
        .header("Authorization", format!("Bearer {access_token}"))
        .header("Content-Type", format!("multipart/related; boundary={boundary}"))
        .body(body)
        .send()
        .await
        .map_err(|e| VaultError::Provider(e.to_string()))?
        .json()
        .await
        .map_err(|e| VaultError::Provider(e.to_string()))?;

    resp["id"]
        .as_str()
        .map(String::from)
        .ok_or_else(|| VaultError::Provider(format!("Drive upload failed: {resp}")))
}

#[async_trait]
impl StorageDriver for GoogleDriveDriver {
    async fn upload(
        &self,
        bucket:        &BucketRow,
        relative_path: &str,
        data:          Bytes,
        mime_type:     &str,
    ) -> VaultResult<UploadResult> {
        let sa_enc = bucket
            .google_sa_json_enc
            .as_deref()
            .ok_or_else(|| VaultError::Provider("Missing google_sa_json_enc".into()))?;
        let sa_json = crypto::decrypt(sa_enc, &self.encryption_key)?;

        let access_token = fetch_access_token(&sa_json).await?;

        let file_name = relative_path.rsplit('/').next().unwrap_or(relative_path);

        let mut hasher = Sha256::new();
        hasher.update(&data);
        let checksum = hex::encode(hasher.finalize());
        let size = data.len() as u64;

        let drive_id = multipart_upload(
            &access_token,
            bucket.drive_folder_id.as_deref(),
            file_name,
            data,
            mime_type,
        )
        .await?;

        Ok(UploadResult {
            uri:          format!("drive://{}/{}", bucket.slug, relative_path),
            provider_ref: drive_id,
            checksum:     Some(checksum),
            file_size:    size,
        })
    }

    async fn verify(&self, bucket: &BucketRow, relative_path: &str) -> VaultResult<bool> {
        let sa_enc = bucket
            .google_sa_json_enc
            .as_deref()
            .ok_or_else(|| VaultError::Provider("Missing google_sa_json_enc".into()))?;
        let sa_json = crypto::decrypt(sa_enc, &self.encryption_key)?;
        let access_token = fetch_access_token(&sa_json).await?;

        // provider_ref (Drive file ID) is not available here; do a name+folder search instead
        let file_name = relative_path.rsplit('/').next().unwrap_or(relative_path);
        let query = match &bucket.drive_folder_id {
            Some(fid) => format!("name='{file_name}' and '{fid}' in parents and trashed=false"),
            None      => format!("name='{file_name}' and trashed=false"),
        };

        let client = reqwest::Client::new();
        let resp: serde_json::Value = client
            .get("https://www.googleapis.com/drive/v3/files")
            .query(&[("q", query.as_str()), ("fields", "files(id)")])
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await
            .map_err(|e| VaultError::Provider(e.to_string()))?
            .json()
            .await
            .map_err(|e| VaultError::Provider(e.to_string()))?;

        Ok(resp["files"]
            .as_array()
            .map(|f| !f.is_empty())
            .unwrap_or(false))
    }
}
