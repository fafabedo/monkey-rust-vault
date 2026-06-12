use bytes::Bytes;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    config::Config,
    db,
    error::{VaultError, VaultResult},
    providers::make_driver,
    uri::StorageUri,
};

pub struct UploadParams {
    pub uri:         String,
    pub data:        Bytes,
    pub mime_type:   String,
    pub uploaded_by: Option<Uuid>,
    pub instance_id: Option<Uuid>,
    pub space_id:    Option<Uuid>,
}

pub struct UploadOutput {
    pub uri:     String,
    pub file_id: Uuid,
}

pub async fn upload_file(
    pool:   &PgPool,
    config: &Config,
    params: UploadParams,
) -> VaultResult<UploadOutput> {
    let parsed = StorageUri::parse(&params.uri)?;

    let bucket = db::get_bucket_with_credentials(pool, &parsed.bucket_slug)
        .await?
        .ok_or_else(|| VaultError::BucketNotFound(parsed.bucket_slug.clone()))?;

    let driver = make_driver(&bucket.provider_type, config)?;

    let file_size = params.data.len() as i64;
    let file_id = db::insert_file_pending(
        pool,
        &bucket,
        &parsed,
        &params.uri,
        &params.mime_type,
        file_size,
        params.uploaded_by,
        params.instance_id,
        params.space_id,
    )
    .await?;

    let result = match driver
        .upload(&bucket, &parsed.relative_path, params.data, &params.mime_type)
        .await
    {
        Ok(r) => r,
        Err(e) => {
            db::mark_file_failed(pool, file_id, &e.to_string()).await.ok();
            return Err(e);
        }
    };

    let verified = driver
        .verify(&bucket, &parsed.relative_path)
        .await
        .unwrap_or(false);

    if !verified {
        db::mark_file_failed(pool, file_id, "Verification failed after upload")
            .await
            .ok();
        return Err(VaultError::VerificationFailed);
    }

    db::mark_file_verified(pool, file_id, &result).await?;

    Ok(UploadOutput { uri: result.uri, file_id })
}
