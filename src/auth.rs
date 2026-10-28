use axum::{extract::Request, http::StatusCode, middleware::Next, response::Response};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::Deserialize;
use uuid::Uuid;

/// Carries the authenticated caller's Supabase UUID, extracted from the JWT `sub` claim.
/// Inserted into request extensions by `require_auth`; handlers extract it as
/// `Option<Extension<AuthUser>>` — `None` means AUTH_BYPASS mode.
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: Uuid,
}

#[derive(Deserialize)]
struct Claims {
    sub: String,
}

/// Axum middleware that validates the Supabase Bearer JWT on every request.
/// Skipped entirely when `AUTH_BYPASS=true` (no `AuthUser` inserted → handlers use fallback).
/// Apply via `route_layer` so /health is excluded.
pub async fn require_auth(mut request: Request, next: Next) -> Result<Response, StatusCode> {
    if std::env::var("AUTH_BYPASS").as_deref() == Ok("true") {
        return Ok(next.run(request).await);
    }

    let token = request
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let secret = std::env::var("SUPABASE_JWT_SECRET")
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::new(Algorithm::HS256),
    )
    .map_err(|_| StatusCode::UNAUTHORIZED)?;

    let user_id = Uuid::parse_str(&token_data.claims.sub)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    request.extensions_mut().insert(AuthUser { user_id });
    Ok(next.run(request).await)
}
