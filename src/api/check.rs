use std::sync::Arc;

use axum::{extract::State, http::HeaderMap, Json};
use serde::{Deserialize, Serialize};

use crate::{
    metrics::Metrics,
    rate_limiter::{CheckInput, CheckOutput, RateLimiter},
    state::AppState,
};

use super::error::ApiError;

#[derive(Debug, Deserialize)]
pub struct CheckRequest {
    pub tenant_id: String,
    pub identifier: String,
    #[serde(default = "default_policy_key")]
    pub policy_key: String,
    #[serde(default = "default_cost")]
    pub cost: u32,
}

#[derive(Debug, Serialize)]
pub struct CheckResponse {
    pub allowed: bool,
    pub remaining: u32,
    pub retry_after_ms: i64,
    pub reset_at_ms: i64,
}

fn default_policy_key() -> String {
    "default".to_string()
}

fn default_cost() -> u32 {
    1
}

pub async fn check(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(payload): Json<CheckRequest>,
) -> Result<Json<CheckResponse>, ApiError> {
    if payload.cost == 0 {
        return Err(ApiError::BadRequest(
            "cost must be greater than zero".to_string(),
        ));
    }

    validate_tenant_auth(&state, &headers, &payload.tenant_id).await?;

    let policy = state
        .policy_store
        .get_policy(&payload.tenant_id, &payload.policy_key)
        .await?;

    let now_ms = current_time_millis();
    let input = CheckInput {
        tenant_id: &payload.tenant_id,
        identifier: &payload.identifier,
        policy_key: &payload.policy_key,
        policy: &policy,
        cost: payload.cost,
        now_ms,
    };

    let output = state.rate_limiter.check(input).await?;
    record_metrics(&state.metrics, &output);

    let reset_at_ms = now_ms + output.retry_after_ms;

    Ok(Json(CheckResponse {
        allowed: output.allowed,
        remaining: output.remaining,
        retry_after_ms: output.retry_after_ms,
        reset_at_ms,
    }))
}

fn record_metrics(metrics: &Metrics, output: &CheckOutput) {
    if output.allowed {
        metrics.record_allowed();
    } else {
        metrics.record_denied();
    }
}

fn current_time_millis() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    now.as_millis() as i64
}

async fn validate_tenant_auth(
    state: &AppState,
    headers: &HeaderMap,
    tenant_id: &str,
) -> Result<(), ApiError> {
    let provided = headers
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .ok_or(ApiError::Unauthorized)?;

    let stored = state.policy_store.get_api_key(tenant_id).await?;
    match stored {
        Some(expected) if expected == provided => Ok(()),
        _ => Err(ApiError::Unauthorized),
    }
}
