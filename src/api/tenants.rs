use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::{
    rate_limiter::{Algorithm, RatePolicy},
    state::AppState,
};

use super::error::ApiError;

#[derive(Debug, Serialize)]
pub struct PolicyResponse {
    pub tenant_id: String,
    pub policy_key: String,
    pub policy: RatePolicy,
}

pub async fn get_policy(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((tenant_id, policy_key)): Path<(String, String)>,
) -> Result<Json<PolicyResponse>, ApiError> {
    validate_tenant_auth(&state, &headers, &tenant_id).await?;

    let policy = state
        .policy_store
        .get_policy(&tenant_id, &policy_key)
        .await?;

    Ok(Json(PolicyResponse {
        tenant_id,
        policy_key,
        policy,
    }))
}

#[derive(Debug, Deserialize)]
pub struct UpsertPolicyRequest {
    pub capacity: u32,
    pub refill_tokens_per_sec: f64,
    #[serde(default)]
    pub algorithm: Algorithm,
    pub api_key: Option<String>,
}

pub async fn upsert_policy(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((tenant_id, policy_key)): Path<(String, String)>,
    Json(body): Json<UpsertPolicyRequest>,
) -> Result<axum::response::Response, ApiError> {
    require_admin(&state, &headers)?;

    if body.capacity == 0 || body.refill_tokens_per_sec <= 0.0 {
        return Err(ApiError::BadRequest(
            "capacity and refill_tokens_per_sec must be positive".to_string(),
        ));
    }

    let policy = RatePolicy {
        capacity: body.capacity,
        refill_tokens_per_sec: body.refill_tokens_per_sec,
        algorithm: body.algorithm,
    };

    state
        .policy_store
        .set_policy(&tenant_id, &policy_key, policy)
        .await?;

    if let Some(api_key) = body.api_key {
        state
            .policy_store
            .set_api_key(&tenant_id, Some(api_key))
            .await?;
    }

    Ok(axum::response::Response::builder()
        .status(axum::http::StatusCode::NO_CONTENT)
        .body(axum::body::Body::empty())
        .unwrap())
}

async fn validate_tenant_auth(
    state: &AppState,
    headers: &HeaderMap,
    tenant_id: &str,
) -> Result<(), ApiError> {
    if let Some(admin) = state.config.admin_api_key.as_ref() {
        if headers
            .get("x-admin-api-key")
            .and_then(|v| v.to_str().ok())
            .filter(|v| *v == admin)
            .is_some()
        {
            return Ok(());
        }
    }

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

fn require_admin(state: &AppState, headers: &HeaderMap) -> Result<(), ApiError> {
    let expected = state.config.admin_api_key.as_ref();
    match expected {
        Some(key) => {
            let provided = headers
                .get("x-admin-api-key")
                .and_then(|v| v.to_str().ok())
                .ok_or(ApiError::Unauthorized)?;
            if provided == key {
                Ok(())
            } else {
                Err(ApiError::Unauthorized)
            }
        }
        None => Err(ApiError::Forbidden(
            "admin api key not configured".to_string(),
        )),
    }
}
