use axum::{http::StatusCode, response::{IntoResponse, Response}, Json};
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum VaultError {
    #[error("Invalid URI: {0}")]
    InvalidUri(String),
    #[error("Bucket not found: {0}")]
    BucketNotFound(String),
    #[error("Upload failed: {0}")]
    UploadFailed(String),
    #[error("Verification failed")]
    VerificationFailed,
    #[error("Database error: {0}")]
    Database(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Provider error: {0}")]
    Provider(String),
    #[error("Crypto error: {0}")]
    Crypto(String),
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
}

impl IntoResponse for VaultError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            VaultError::InvalidUri(_)      => (StatusCode::BAD_REQUEST, self.to_string()),
            VaultError::BucketNotFound(_)  => (StatusCode::NOT_FOUND, self.to_string()),
            VaultError::VerificationFailed => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            _                              => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
        };
        (status, Json(json!({ "error": message }))).into_response()
    }
}

pub type VaultResult<T> = Result<T, VaultError>;
