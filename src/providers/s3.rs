use async_trait::async_trait;
use bytes::Bytes;
use object_store::{aws::AmazonS3Builder, path::Path, ObjectStore, ObjectStoreExt};
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

    fn build_store(&self, bucket: &BucketRow) -> VaultResult<impl ObjectStore> {
        let s3_bucket = bucket
            .s3_bucket_name
            .as_deref()
            .ok_or_else(|| VaultError::Provider("Missing s3_bucket_name".into()))?;

        let region = bucket
            .aws_region
            .as_deref()
            .unwrap_or(&self.default_region);

        let mut builder = AmazonS3Builder::new()
            .with_bucket_name(s3_bucket)
            .with_region(region)
            .with_virtual_hosted_style_request(true);

        if let (Some(key_enc), Some(secret_enc)) =
            (&bucket.aws_access_key_enc, &bucket.aws_secret_key_enc)
        {
            let access_key = crypto::decrypt(key_enc, &self.encryption_key)?;
            let secret_key = crypto::decrypt(secret_enc, &self.encryption_key)?;
            builder = builder
                .with_access_key_id(access_key)
                .with_secret_access_key(secret_key);
        }
        // No explicit credentials → object_store falls back to env vars / instance profile.

        builder
            .build()
            .map_err(|e| VaultError::Provider(e.to_string()))
    }
}

#[async_trait]
impl StorageDriver for S3Driver {
    async fn upload(
        &self,
        bucket:        &BucketRow,
        relative_path: &str,
        data:          Bytes,
        _mime_type:    &str,
    ) -> VaultResult<UploadResult> {
        let store = self.build_store(bucket)?;
        let path  = Path::from(relative_path);

        let mut hasher = Sha256::new();
        hasher.update(&data);
        let checksum = hex::encode(hasher.finalize());
        let size = data.len() as u64;

        let put_result = store
            .put(&path, data.into())
            .await
            .map_err(|e| VaultError::Provider(e.to_string()))?;

        let provider_ref = put_result
            .e_tag
            .unwrap_or_else(|| relative_path.to_string());

        Ok(UploadResult {
            uri:          format!("s3://{}/{}", bucket.slug, relative_path),
            provider_ref,
            checksum:     Some(checksum),
            file_size:    size,
        })
    }

    async fn verify(&self, bucket: &BucketRow, relative_path: &str) -> VaultResult<bool> {
        let store = self.build_store(bucket)?;
        let path  = Path::from(relative_path);
        Ok(store.head(&path).await.is_ok())
    }
}
