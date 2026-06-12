mod config;
mod crypto;
mod db;
mod error;
mod models;
mod providers;
mod routes;
mod service;
mod supabase;
mod uri;

use std::sync::Arc;

use axum::{routing::post, Router};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use config::Config;
use supabase::SupabaseClient;

pub struct AppState {
    pub supabase: SupabaseClient,
    pub config:   Config,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "monkey_vault=debug,tower_http=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::from_env()?;
    let bind   = config.bind_addr.clone();

    let supabase = SupabaseClient::new(&config.supabase_url, &config.supabase_service_key);
    let state    = Arc::new(AppState { supabase, config });

    let app = Router::new()
        .route("/upload", post(routes::upload::handle_upload))
        .route("/health", axum::routing::get(|| async { "ok" }))
        .with_state(state)
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive());

    tracing::info!("monkey-vault listening on {bind}");
    let listener = tokio::net::TcpListener::bind(&bind).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
