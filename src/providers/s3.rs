use async_trait::async_trait;
use aws_config::{BehaviorVersion, Region};
use aws_sdk_s3::{primitives::ByteStream, Client};
use bytes::Bytes;
use sha2::{Digest, Sha256};

use crate::{
    config::Config,
    crypto,
    error::{VaultError, VaultResult},
    models::BucketRow,
};
use super::{StorageDriver, UploadResult};

pub struct S3Driver {
    default_region: String,
    encryption_key: String,
}

impl S3Driver {
    pub fn new(config: &Config) -> Self {
        Self {
            default_region: config.aws_default_region.clone(),
            encryption_key: config.vault_encryption_key.clone(),
        }
    }

    async fn client(&self, bucket: &BucketRow) -> VaultResult<Client> {
        let region = bucket
            .aws_region
            .as_deref()
            .unwrap_or(&self.default_region)
            .to_string();

        let cfg = if let (Some(key_enc), Some(secret_enc)) =
            (&bucket.aws_access_key_enc, &bucket.aws_secret_key_enc)
        {
            let access_key = crypto::decrypt(key_enc, &self.encryption_key)?;
            let secret_key = crypto::decrypt(secret_enc, &self.encryption_key)?;
            aws_config::defaults(BehaviorVersion::latest())
                .region(Region::new(region))
                .credentials_provider(aws_sdk_s3::config::Credentials::new(
                    access_key, secret_key, None, None, "vault",
                ))
                .load()
                .await
        } else {
            aws_config::defaults(BehaviorVersion::latest())
                .region(Region::new(region))
                .profile_name(bucket.aws_profile.as_deref().unwrap_or("default"))
                .load()
                .await
        };

        Ok(Client::new(&cfg))
    }
}

#[async_trait]
impl StorageDriver for S3Driver {
    async fn upload(
        &self,
        bucket:        &BucketRow,
        relative_path: &str,
        data:          Bytes,
        mime_type:     &str,
    ) -> VaultResult<UploadResult> {
        let client = self.client(bucket).await?;
        let s3_bucket = bucket
            .s3_bucket_name
            .as_deref()
            .ok_or_else(|| VaultError::Provider("Missing s3_bucket_name".into()))?;

        let mut hasher = Sha256::new();
        hasher.update(&data);
        let checksum = hex::encode(hasher.finalize());
        let size = data.len() as u64;

        client
            .put_object()
            .bucket(s3_bucket)
            .key(relative_path)
            .body(ByteStream::from(data))
            .content_type(mime_type)
            .send()
            .await
            .map_err(|e| VaultError::Provider(e.to_string()))?;

        Ok(UploadResult {
            uri:          format!("s3://{}/{}", bucket.slug, relative_path),
            provider_ref: relative_path.to_string(),
            checksum:     Some(checksum),
            file_size:    size,
        })
    }

    async fn verify(&self, bucket: &BucketRow, relative_path: &str) -> VaultResult<bool> {
        let client = self.client(bucket).await?;
        let s3_bucket = bucket
            .s3_bucket_name
            .as_deref()
            .ok_or_else(|| VaultError::Provider("Missing s3_bucket_name".into()))?;

        Ok(client
            .head_object()
            .bucket(s3_bucket)
            .key(relative_path)
            .send()
            .await
            .is_ok())
    }
}
