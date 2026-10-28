use axum::{
    extract::{Extension, Multipart, State},
    Json,
};
use bytes::Bytes;
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    error::{VaultError, VaultResult},
    models::UploadResponse,
    service::{upload_file, UploadParams},
    supabase::SupabaseClient,
    AppState,
};

pub async fn handle_upload(
    State(state):  State<Arc<AppState>>,
    auth:          Option<Extension<AuthUser>>,
    mut multipart: Multipart,
) -> VaultResult<Json<UploadResponse>> {
    let mut uri:         Option<String> = None;
    let mut file_data:   Option<Bytes>  = None;
    let mut mime_type:   String         = "application/octet-stream".into();
    let mut instance_id: Option<Uuid>   = None;
    let mut space_id:    Option<Uuid>   = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| VaultError::UploadFailed(e.to_string()))?
    {
        match field.name() {
            Some("uri") => {
                uri = Some(
                    field.text().await
                        .map_err(|e| VaultError::UploadFailed(e.to_string()))?,
                );
            }
            Some("instance_id") => {
                instance_id = field.text().await.ok()
                    .and_then(|s| Uuid::parse_str(&s).ok());
            }
            Some("space_id") => {
                space_id = field.text().await.ok()
                    .and_then(|s| Uuid::parse_str(&s).ok());
            }
            Some("file") => {
                if let Some(ct) = field.content_type() {
                    mime_type = ct.to_string();
                }
                file_data = Some(
                    field.bytes().await
                        .map_err(|e| VaultError::UploadFailed(e.to_string()))?,
                );
            }
            _ => {}
        }
    }

    let uri       = uri.ok_or_else(|| VaultError::InvalidUri("Missing `uri` field".into()))?;
    let file_data = file_data.ok_or_else(|| VaultError::UploadFailed("Missing `file` field".into()))?;

    let auth_user = auth.map(|Extension(u)| u);

    // When a space is specified and the request is authenticated, enforce permission.
    if let (Some(space), Some(user)) = (space_id, &auth_user) {
        assert_space_access(&state.supabase, space, user).await?;
    }

    let uploaded_by = auth_user.as_ref().map(|u| u.user_id);

    let output = upload_file(
        &state.supabase,
        &state.config,
        UploadParams {
            uri,
            data: file_data,
            mime_type,
            uploaded_by,
            instance_id,
            space_id,
        },
    )
    .await?;

    Ok(Json(UploadResponse {
        success: true,
        uri:     output.uri,
        file_id: output.file_id,
    }))
}

/// Resolves space UUID and auth user to bigint IDs, then checks active owner/editor permission.
async fn assert_space_access(
    supabase:   &SupabaseClient,
    space_uuid: Uuid,
    user:       &AuthUser,
) -> VaultResult<()> {
    let space_bigint_id = supabase.get_space_bigint_id(space_uuid).await?;
    let user_bigint_id  = supabase.get_user_bigint_id(user.user_id).await?;
    supabase.check_space_permission(space_bigint_id, user_bigint_id).await
}
