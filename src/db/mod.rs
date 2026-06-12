use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::{VaultError, VaultResult},
    models::BucketRow,
    providers::UploadResult,
    supabase::SupabaseClient,
    uri::StorageUri,
};

// ── PostgREST response shapes ────────────────────────────────────────────────
// PostgREST embeds many-to-one (FK on this table) as an object, and
// one-to-many as an array. The chain here is:
//   storage_buckets.provider_id → storage_providers.id
//   storage_provider_credentials.provider_id → storage_providers.id

#[derive(Deserialize)]
struct BucketApiRow {
    id:                Uuid,
    slug:              String,
    s3_bucket_name:    Option<String>,
    dropbox_root_path: Option<String>,
    drive_folder_id:   Option<String>,
    local_sub_path:    Option<String>,
    instance_id:       Option<Uuid>,
    space_id:          Option<Uuid>,
    storage_providers: ProviderApiRow,
}

#[derive(Deserialize)]
struct ProviderApiRow {
    #[serde(rename = "type")]
    provider_type:                String,
    is_active:                    bool,
    storage_provider_credentials: Option<CredsApiRow>,
}

#[derive(Deserialize)]
struct CredsApiRow {
    aws_access_key_enc:       Option<String>,
    aws_secret_key_enc:       Option<String>,
    aws_region:               Option<String>,
    dropbox_access_token_enc: Option<String>,
    google_sa_json_enc:       Option<String>,
    local_base_path:          Option<String>,
}

#[derive(Deserialize)]
struct InsertedRow {
    id: Uuid,
}

// ── Public helpers ────────────────────────────────────────────────────────────

pub async fn get_bucket_with_credentials(
    supabase: &SupabaseClient,
    slug:     &str,
) -> VaultResult<Option<BucketRow>> {
    let select = [
        "id,slug,s3_bucket_name,dropbox_root_path,drive_folder_id,",
        "local_sub_path,instance_id,space_id,",
        "storage_providers!provider_id(",
        "type,is_active,",
        "storage_provider_credentials(",
        "aws_access_key_enc,aws_secret_key_enc,aws_region,",
        "dropbox_access_token_enc,google_sa_json_enc,local_base_path))",
    ]
    .concat();

    let slug_filter = format!("eq.{slug}");
    let response = supabase
        .get("storage_buckets")
        .query(&[
            ("slug",      slug_filter.as_str()),
            ("is_active", "eq.true"),
            ("select",    select.as_str()),
            ("limit",     "1"),
        ])
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let body   = response.text().await.unwrap_or_default();
        return Err(VaultError::Database(format!(
            "get_bucket_with_credentials: Supabase returned {status} — {body}"
        )));
    }

    let body = response.text().await?;
    tracing::debug!("get_bucket_with_credentials raw response: {body}");

    let rows: Vec<BucketApiRow> = serde_json::from_str(&body).map_err(|e| {
        VaultError::Database(format!(
            "get_bucket_with_credentials: parse error — {e} | body: {body}"
        ))
    })?;

    let row = match rows.into_iter().next() {
        Some(r) => r,
        None    => return Ok(None),
    };

    if !row.storage_providers.is_active {
        return Ok(None);
    }

    let creds = row.storage_providers.storage_provider_credentials;

    Ok(Some(BucketRow {
        id:                       row.id,
        slug:                     row.slug,
        s3_bucket_name:           row.s3_bucket_name,
        dropbox_root_path:        row.dropbox_root_path,
        drive_folder_id:          row.drive_folder_id,
        local_sub_path:           row.local_sub_path,
        instance_id:              row.instance_id,
        space_id:                 row.space_id,
        provider_type:            row.storage_providers.provider_type,
        aws_access_key_enc:       creds.as_ref().and_then(|c| c.aws_access_key_enc.clone()),
        aws_secret_key_enc:       creds.as_ref().and_then(|c| c.aws_secret_key_enc.clone()),
        aws_region:               creds.as_ref().and_then(|c| c.aws_region.clone()),
        dropbox_access_token_enc: creds.as_ref().and_then(|c| c.dropbox_access_token_enc.clone()),
        google_sa_json_enc:       creds.as_ref().and_then(|c| c.google_sa_json_enc.clone()),
        local_base_path:          creds.and_then(|c| c.local_base_path),
    }))
}

#[derive(Serialize)]
struct PendingFileBody<'a> {
    bucket_id:     Uuid,
    relative_path: &'a str,
    uri:           &'a str,
    file_name:     &'a str,
    file_size:     i64,
    mime_type:     &'a str,
    upload_status: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    uploaded_by:   Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    instance_id:   Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    space_id:      Option<Uuid>,
}

pub async fn insert_file_pending(
    supabase:      &SupabaseClient,
    bucket:        &BucketRow,
    parsed:        &StorageUri,
    uri:           &str,
    mime_type:     &str,
    file_size:     i64,
    uploaded_by:   Option<Uuid>,
    instance_id:   Option<Uuid>,
    space_id:      Option<Uuid>,
) -> VaultResult<Uuid> {
    let body = PendingFileBody {
        bucket_id:     bucket.id,
        relative_path: &parsed.relative_path,
        uri,
        file_name:     &parsed.file_name,
        file_size,
        mime_type,
        upload_status: "pending",
        uploaded_by,
        instance_id,
        space_id,
    };

    let response = supabase
        .upsert("storage_files", "bucket_id,relative_path")
        .json(&body)
        .send()
        .await?;

    if !response.status().is_success() {
        let err = response.text().await.unwrap_or_default();
        return Err(VaultError::Database(format!("insert_file_pending: {err}")));
    }

    let rows: Vec<InsertedRow> = response.json().await?;
    rows.into_iter()
        .next()
        .map(|r| r.id)
        .ok_or_else(|| VaultError::Database("insert returned no rows".into()))
}

/// Returns true if a non-failed file already exists at this path in the bucket.
pub async fn path_exists(
    supabase:      &SupabaseClient,
    bucket_id:     Uuid,
    relative_path: &str,
) -> VaultResult<bool> {
    let bucket_filter = format!("eq.{bucket_id}");
    let path_filter   = format!("eq.{relative_path}");

    let response = supabase
        .get("storage_files")
        .query(&[
            ("bucket_id",     bucket_filter.as_str()),
            ("relative_path", path_filter.as_str()),
            ("upload_status", "neq.failed"),
            ("select",        "id"),
            ("limit",         "1"),
        ])
        .send()
        .await?;

    if !response.status().is_success() {
        let err = response.text().await.unwrap_or_default();
        return Err(VaultError::Database(format!("path_exists: {err}")));
    }

    let rows: Vec<serde_json::Value> = response.json().await?;
    Ok(!rows.is_empty())
}

pub async fn mark_file_failed(
    supabase: &SupabaseClient,
    file_id:  Uuid,
    message:  &str,
) -> VaultResult<()> {
    let response = supabase
        .patch("storage_files")
        .query(&[("id", &format!("eq.{file_id}"))])
        .json(&serde_json::json!({
            "upload_status": "failed",
            "error_message": message
        }))
        .send()
        .await?;

    if !response.status().is_success() {
        let err = response.text().await.unwrap_or_default();
        return Err(VaultError::Database(format!("mark_file_failed: {err}")));
    }
    Ok(())
}

pub async fn mark_file_verified(
    supabase: &SupabaseClient,
    file_id:  Uuid,
    result:   &UploadResult,
) -> VaultResult<()> {
    let response = supabase
        .patch("storage_files")
        .query(&[("id", &format!("eq.{file_id}"))])
        .json(&serde_json::json!({
            "upload_status":   "verified",
            "provider_ref":    result.provider_ref,
            "checksum_sha256": result.checksum,
            "file_size":       result.file_size as i64
        }))
        .send()
        .await?;

    if !response.status().is_success() {
        let err = response.text().await.unwrap_or_default();
        return Err(VaultError::Database(format!("mark_file_verified: {err}")));
    }
    Ok(())
}
