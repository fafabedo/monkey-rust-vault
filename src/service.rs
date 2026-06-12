use bytes::Bytes;
use std::path::Path;
use uuid::Uuid;

use crate::{
    config::Config,
    db,
    error::{VaultError, VaultResult},
    providers::make_driver,
    supabase::SupabaseClient,
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

/// Returns the first path that does not already exist in storage_files.
/// `2026/06/test.csv` → `2026/06/test_1.csv` → `2026/06/test_2.csv` …
async fn resolve_unique_path(
    supabase:      &SupabaseClient,
    bucket_id:     Uuid,
    relative_path: &str,
) -> VaultResult<String> {
    if !db::path_exists(supabase, bucket_id, relative_path).await? {
        return Ok(relative_path.to_string());
    }

    let p     = Path::new(relative_path);
    let dir   = p.parent().map(|d| d.to_string_lossy().into_owned()).unwrap_or_default();
    let stem  = p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let ext   = p.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();

    for n in 1u32.. {
        let candidate = if dir.is_empty() {
            format!("{stem}_{n}{ext}")
        } else {
            format!("{dir}/{stem}_{n}{ext}")
        };

        if !db::path_exists(supabase, bucket_id, &candidate).await? {
            tracing::debug!("path conflict — renamed to {candidate}");
            return Ok(candidate);
        }
    }

    Err(VaultError::UploadFailed("could not find a unique path after many attempts".into()))
}

pub async fn upload_file(
    supabase: &SupabaseClient,
    config:   &Config,
    params:   UploadParams,
) -> VaultResult<UploadOutput> {
    let mut parsed = StorageUri::parse(&params.uri)?;

    let bucket = db::get_bucket_with_credentials(supabase, &parsed.bucket_slug)
        .await?
        .ok_or_else(|| VaultError::BucketNotFound(parsed.bucket_slug.clone()))?;

    let driver = make_driver(&bucket.provider_type, config)?;

    // Rename if a file already exists at this path in the same bucket.
    let unique_path = resolve_unique_path(supabase, bucket.id, &parsed.relative_path).await?;
    if unique_path != parsed.relative_path {
        parsed.file_name    = Path::new(&unique_path)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| unique_path.clone());
        parsed.relative_path = unique_path;
    }

    let final_uri = format!("{}://{}/{}", parsed.scheme.as_str(), bucket.slug, parsed.relative_path);

    let file_size = params.data.len() as i64;
    let file_id = db::insert_file_pending(
        supabase,
        &bucket,
        &parsed,
        &final_uri,
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
            db::mark_file_failed(supabase, file_id, &e.to_string()).await.ok();
            return Err(e);
        }
    };

    let verified = driver
        .verify(&bucket, &parsed.relative_path)
        .await
        .unwrap_or(false);

    if !verified {
        db::mark_file_failed(supabase, file_id, "Verification failed after upload")
            .await
            .ok();
        return Err(VaultError::VerificationFailed);
    }

    db::mark_file_verified(supabase, file_id, &result).await?;

    Ok(UploadOutput { uri: result.uri, file_id })
}
