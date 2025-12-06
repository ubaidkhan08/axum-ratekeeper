use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

use crate::rate_limiter::PolicyError;

#[derive(Debug, Serialize)]
pub struct ErrorBody {
    pub message: String,
}

#[derive(Debug)]
pub enum ApiError {
    BadRequest(String),
    Unauthorized,
    Forbidden(String),
    PolicyNotFound,
    Internal(anyhow::Error),
}

impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        if err.downcast_ref::<PolicyError>().is_some() {
            ApiError::PolicyNotFound
        } else {
            ApiError::Internal(err)
        }
    }
}

impl From<PolicyError> for ApiError {
    fn from(err: PolicyError) -> Self {
        match err {
            PolicyError::NotFound => ApiError::PolicyNotFound,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            ApiError::BadRequest(msg) => {
                (StatusCode::BAD_REQUEST, Json(ErrorBody { message: msg })).into_response()
            }
            ApiError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                Json(ErrorBody {
                    message: "unauthorized".to_string(),
                }),
            )
                .into_response(),
            ApiError::Forbidden(msg) => {
                (StatusCode::FORBIDDEN, Json(ErrorBody { message: msg })).into_response()
            }
            ApiError::PolicyNotFound => (
                StatusCode::NOT_FOUND,
                Json(ErrorBody {
                    message: "policy not found".to_string(),
                }),
            )
                .into_response(),
            ApiError::Internal(err) => {
                tracing::error!(error = ?err, "internal server error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorBody {
                        message: "internal server error".to_string(),
                    }),
                )
                    .into_response()
            }
        }
    }
}
