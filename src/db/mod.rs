use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    error::VaultResult,
    models::BucketRow,
    providers::UploadResult,
    uri::StorageUri,
};

pub async fn get_bucket_with_credentials(
    pool: &PgPool,
    slug: &str,
) -> VaultResult<Option<BucketRow>> {
    let row = sqlx::query_as::<_, BucketRow>(
        r#"
        SELECT
            b.id,
            b.slug,
            b.s3_bucket_name,
            b.dropbox_root_path,
            b.drive_folder_id,
            b.local_sub_path,
            b.instance_id,
            b.space_id,
            p.type::text              AS provider_type,
            c.aws_profile,
            c.aws_access_key_enc,
            c.aws_secret_key_enc,
            c.aws_region,
            c.dropbox_access_token_enc,
            c.google_sa_json_enc,
            c.local_base_path
        FROM  storage_buckets b
        JOIN  storage_providers p             ON p.id = b.provider_id
        LEFT JOIN storage_provider_credentials c ON c.provider_id = p.id
        WHERE b.slug = $1
          AND b.is_active = true
          AND p.is_active = true
        LIMIT 1
        "#,
    )
    .bind(slug)
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

pub async fn insert_file_pending(
    pool:          &PgPool,
    bucket:        &BucketRow,
    parsed:        &StorageUri,
    uri:           &str,
    mime_type:     &str,
    file_size:     i64,
    uploaded_by:   Option<Uuid>,
    instance_id:   Option<Uuid>,
    space_id:      Option<Uuid>,
) -> VaultResult<Uuid> {
    let rec = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO storage_files
            (bucket_id, relative_path, uri, file_name, file_size, mime_type,
             upload_status, uploaded_by, instance_id, space_id)
        VALUES ($1, $2, $3, $4, $5, $6, 'pending', $7, $8, $9)
        RETURNING id
        "#,
    )
    .bind(bucket.id)
    .bind(&parsed.relative_path)
    .bind(uri)
    .bind(&parsed.file_name)
    .bind(file_size)
    .bind(mime_type)
    .bind(uploaded_by)
    .bind(instance_id)
    .bind(space_id)
    .fetch_one(pool)
    .await?;

    Ok(rec)
}

pub async fn mark_file_failed(
    pool:    &PgPool,
    file_id: Uuid,
    message: &str,
) -> VaultResult<()> {
    sqlx::query(
        "UPDATE storage_files SET upload_status = 'failed', error_message = $2, updated_at = now() WHERE id = $1",
    )
    .bind(file_id)
    .bind(message)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn mark_file_verified(
    pool:    &PgPool,
    file_id: Uuid,
    result:  &UploadResult,
) -> VaultResult<()> {
    sqlx::query(
        r#"
        UPDATE storage_files
        SET upload_status   = 'verified',
            provider_ref    = $2,
            checksum_sha256 = $3,
            file_size       = $4,
            updated_at      = now()
        WHERE id = $1
        "#,
    )
    .bind(file_id)
    .bind(&result.provider_ref)
    .bind(result.checksum.as_deref())
    .bind(result.file_size as i64)
    .execute(pool)
    .await?;
    Ok(())
}
