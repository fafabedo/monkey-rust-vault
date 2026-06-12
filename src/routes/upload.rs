use axum::{
    extract::{Multipart, State},
    Json,
};
use bytes::Bytes;
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    error::{VaultError, VaultResult},
    models::UploadResponse,
    service::{upload_file, UploadParams},
    AppState,
};

pub async fn handle_upload(
    State(state): State<Arc<AppState>>,
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
                    field
                        .text()
                        .await
                        .map_err(|e| VaultError::UploadFailed(e.to_string()))?,
                );
            }
            Some("instance_id") => {
                instance_id = field
                    .text()
                    .await
                    .ok()
                    .and_then(|s| Uuid::parse_str(&s).ok());
            }
            Some("space_id") => {
                space_id = field
                    .text()
                    .await
                    .ok()
                    .and_then(|s| Uuid::parse_str(&s).ok());
            }
            Some("file") => {
                if let Some(ct) = field.content_type() {
                    mime_type = ct.to_string();
                }
                file_data = Some(
                    field
                        .bytes()
                        .await
                        .map_err(|e| VaultError::UploadFailed(e.to_string()))?,
                );
            }
            _ => {}
        }
    }

    let uri = uri.ok_or_else(|| VaultError::InvalidUri("Missing `uri` field".into()))?;
    let file_data =
        file_data.ok_or_else(|| VaultError::UploadFailed("Missing `file` field".into()))?;

    let output = upload_file(
        &state.db,
        &state.config,
        UploadParams {
            uri,
            data: file_data,
            mime_type,
            uploaded_by: None,
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
