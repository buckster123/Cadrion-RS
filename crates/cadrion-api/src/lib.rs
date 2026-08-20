//! Cadrion HTTP API.

#![deny(unsafe_code)]

mod jobs;
mod openapi;
mod routes;
mod state;

pub use openapi::openapi_doc;
pub use routes::router;
pub use state::{AppConfig, AppState};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Bind and serve until cancelled.
pub async fn serve(cfg: AppConfig) -> anyhow::Result<()> {
    let state = AppState::new(cfg.clone());
    let app = router(state);
    let listener = tokio::net::TcpListener::bind(&cfg.bind).await?;
    tracing::info!("cadrion-api listening on http://{}", cfg.bind);
    axum::serve(listener, app).await?;
    Ok(())
}
