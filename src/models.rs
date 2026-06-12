use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, sqlx::FromRow)]
pub struct BucketRow {
    pub id:                       Uuid,
    pub slug:                     String,
    pub s3_bucket_name:           Option<String>,
    pub dropbox_root_path:        Option<String>,
    pub drive_folder_id:          Option<String>,
    pub local_sub_path:           Option<String>,
    pub instance_id:              Option<Uuid>,
    pub space_id:                 Option<Uuid>,
    // joined from storage_providers
    pub provider_type:            String,
    // joined from storage_provider_credentials
    pub aws_profile:              Option<String>,
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
