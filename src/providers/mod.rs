use async_trait::async_trait;
use bytes::Bytes;
use std::sync::Arc;

use crate::{config::Config, error::VaultResult, models::BucketRow};

pub struct UploadResult {
    pub uri:          String,
    pub provider_ref: String,
    pub checksum:     Option<String>,
    pub file_size:    u64,
}

#[async_trait]
pub trait StorageDriver: Send + Sync {
    async fn upload(
        &self,
        bucket:        &BucketRow,
        relative_path: &str,
        data:          Bytes,
        mime_type:     &str,
    ) -> VaultResult<UploadResult>;

    async fn verify(
        &self,
        bucket:        &BucketRow,
        relative_path: &str,
    ) -> VaultResult<bool>;
}

pub mod dropbox;
pub mod google_drive;
pub mod local;
pub mod s3;

pub fn make_driver(
    provider_type: &str,
    config: &Config,
) -> VaultResult<Arc<dyn StorageDriver>> {
    match provider_type {
        "s3"           => Ok(Arc::new(s3::S3Driver::new(config))),
        "local"        => Ok(Arc::new(local::LocalDriver)),
        "dropbox"      => Ok(Arc::new(dropbox::DropboxDriver::new(config))),
        "google_drive" => Ok(Arc::new(google_drive::GoogleDriveDriver::new(config))),
        other => Err(crate::error::VaultError::Provider(
            format!("Unknown provider type: {other}"),
        )),
    }
}
