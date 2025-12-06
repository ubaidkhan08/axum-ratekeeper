use std::sync::Arc;

use anyhow::Result;
use axum_server::{tls_rustls::RustlsConfig, Handle};
use tokio::signal;
use tracing_subscriber::{fmt, EnvFilter};

mod api;
mod config;
mod metrics;
mod rate_limiter;
mod redis_pool;
mod state;

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    let config = config::Config::from_env()?;
    let state = state::AppState::initialize(config).await?;
    let app = api::router(Arc::clone(&state));

    let bind_addr = state.config.bind_addr;

    if let (Some(cert_path), Some(key_path)) = (
        state.config.tls_cert_path.clone(),
        state.config.tls_key_path.clone(),
    ) {
        let tls_config = RustlsConfig::from_pem_file(cert_path, key_path).await?;
        tracing::info!("listening with TLS on {}", bind_addr);

        let handle = Handle::new();
        let shutdown_handle = handle.clone();
        tokio::spawn(async move {
            shutdown_signal().await;
            shutdown_handle.shutdown();
        });

        axum_server::bind_rustls(bind_addr, tls_config)
            .handle(handle)
            .serve(app.into_make_service())
            .await?;
    } else {
        let listener = tokio::net::TcpListener::bind(bind_addr).await?;
        tracing::info!("listening on {}", bind_addr);

        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal())
            .await?;
    }

    Ok(())
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,axum::rejection=trace"));

    fmt().with_env_filter(filter).init();
}

async fn shutdown_signal() {
    if signal::ctrl_c().await.is_ok() {
        tracing::info!("shutdown signal received");
    }
}
