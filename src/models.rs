use serde::Serialize;
use uuid::Uuid;

/// Flat view of a bucket row, assembled from the Supabase PostgREST response.
/// Used by all StorageDriver impls.
#[derive(Debug)]
#[allow(dead_code)]
pub struct BucketRow {
    pub id:                       Uuid,
    pub slug:                     String,
    pub s3_bucket_name:           Option<String>,
    pub dropbox_root_path:        Option<String>,
    pub drive_folder_id:          Option<String>,
    pub local_sub_path:           Option<String>,
    pub instance_id:              Option<Uuid>,
    pub space_id:                 Option<Uuid>,
    // from storage_providers
    pub provider_type:            String,
    // from storage_provider_credentials
    pub aws_access_key_enc:       Option<String>,
    pub aws_secret_key_enc:       Option<String>,
    pub aws_region:               Option<String>,
    pub dropbox_access_token_enc: Option<String>,
    pub google_sa_json_enc:       Option<String>,
    pub local_base_path:          Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UploadResponse {
    pub success: bool,
    pub uri:     String,
    pub file_id: Uuid,
}
