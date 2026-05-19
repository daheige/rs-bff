use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

#[derive(thiserror::Error, Debug)]
pub enum AppError {
    #[error("grpc error: {0}")]
    Grpc(#[from] tonic::Status),

    #[error("grpc transport error: {0}")]
    GrpcTransport(#[from] tonic::transport::Error),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("internal error: {0}")]
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::Grpc(status) => {
                let code = match status.code() {
                    tonic::Code::NotFound => StatusCode::NOT_FOUND,
                    tonic::Code::InvalidArgument => StatusCode::BAD_REQUEST,
                    tonic::Code::Unauthenticated => StatusCode::UNAUTHORIZED,
                    tonic::Code::PermissionDenied => StatusCode::FORBIDDEN,
                    tonic::Code::AlreadyExists => StatusCode::CONFLICT,
                    tonic::Code::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
                    tonic::Code::DeadlineExceeded => StatusCode::GATEWAY_TIMEOUT,
                    _ => StatusCode::INTERNAL_SERVER_ERROR,
                };
                (code, status.message().to_string())
            }
            AppError::GrpcTransport(_) => (
                StatusCode::BAD_GATEWAY,
                "upstream service unavailable".to_string(),
            ),
            AppError::Serialization(_) => (
                StatusCode::BAD_REQUEST,
                "request body serialization failed".to_string(),
            ),
            AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
        };

        let body = Json(json!({
            "error": message,
            "code": status.as_u16(),
        }));

        (status, body).into_response()
    }
}
