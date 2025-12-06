use std::sync::Arc;

use axum::{
    http::{header, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Router,
};

use crate::state::AppState;

pub mod check;
pub mod error;
pub mod tenants;

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/v1/check", post(check::check))
        .route(
            "/v1/tenants/:tenant_id/policies/:policy_key",
            get(tenants::get_policy).put(tenants::upsert_policy),
        )
        .route("/health/live", get(health))
        .route("/health/ready", get(health))
        .route("/metrics", get(metrics))
        .with_state(state)
}

async fn health() -> impl IntoResponse {
    StatusCode::OK
}

async fn metrics(state: axum::extract::State<Arc<AppState>>) -> impl IntoResponse {
    let body = state.metrics.export();
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/plain; version=0.0.4")],
        body,
    )
}
