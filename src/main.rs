use axum::{
    routing::{post, get},
    Router,
    http::StatusCode,
};
use std::sync::Arc;
use tracing::info;

use sentryshark::config::AppConfig;
use sentryshark::db::Database;
use sentryshark::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let config = Arc::new(AppConfig::load()?);
    let db_path = config.database_config().path.clone();
    let database = Arc::new(Database::new(&db_path)?);

    let state = AppState { config, database };

    let dashboard_enabled = state.config.dashboard_config().enabled;

    let mut app = Router::new()
        .route("/webhook/github", post(sentryshark::github::webhook_handler))
        .route("/webhook/gitlab", post(sentryshark::gitlab::webhook_handler))
        .route("/health", get(health_check));

    if dashboard_enabled {
        app = app
            .route("/dashboard", get(sentryshark::dashboard::dashboard_handler))
            .route("/dashboard/stats", get(sentryshark::dashboard::stats_api_handler));
        info!("📊 Dashboard enabled at /dashboard");
    }

    let app = app.with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    info!("🦈 SentryShark v0.3.0 listening on {}", listener.local_addr()?);

    axum::serve(listener, app).await?;
    Ok(())
}

async fn health_check() -> StatusCode {
    StatusCode::OK
}
