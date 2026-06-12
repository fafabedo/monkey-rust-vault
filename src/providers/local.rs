use async_trait::async_trait;
use bytes::Bytes;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use tokio::fs;

use crate::{
    error::{VaultError, VaultResult},
    models::BucketRow,
};
use super::{StorageDriver, UploadResult};

pub struct LocalDriver;

#[async_trait]
impl StorageDriver for LocalDriver {
    async fn upload(
        &self,
        bucket:        &BucketRow,
        relative_path: &str,
        data:          Bytes,
        _mime_type:    &str,
    ) -> VaultResult<UploadResult> {
        let base = bucket
            .local_base_path
            .as_deref()
            .ok_or_else(|| VaultError::Provider("Missing local_base_path".into()))?;

        let mut path = PathBuf::from(base);
        if let Some(sub) = &bucket.local_sub_path {
            path.push(sub);
        }
        path.push(relative_path);

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }
        fs::write(&path, &data).await?;

        let mut hasher = Sha256::new();
        hasher.update(&data);
        let checksum = hex::encode(hasher.finalize());

        Ok(UploadResult {
            uri:          format!("fs://{}/{}", bucket.slug, relative_path),
            provider_ref: path.to_string_lossy().into_owned(),
            checksum:     Some(checksum),
            file_size:    data.len() as u64,
        })
    }

    async fn verify(&self, bucket: &BucketRow, relative_path: &str) -> VaultResult<bool> {
        let base = bucket
            .local_base_path
            .as_deref()
            .ok_or_else(|| VaultError::Provider("Missing local_base_path".into()))?;

        let mut path = PathBuf::from(base);
        if let Some(sub) = &bucket.local_sub_path {
            path.push(sub);
        }
        path.push(relative_path);

        Ok(path.exists())
    }
}
