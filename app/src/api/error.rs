use actix_web::{HttpResponse, ResponseError};
use serde::Serialize;
use std::fmt;

#[derive(Debug)]
pub struct ApiError {
    pub code: String,
    pub message: String,
    pub status: u16,
}

impl ApiError {
    pub fn bad_request(code: &str, message: &str) -> Self {
        Self {
            code: code.to_string(),
            message: message.to_string(),
            status: 400,
        }
    }

    pub fn unauthorized(message: &str) -> Self {
        Self {
            code: "UNAUTHORIZED".to_string(),
            message: message.to_string(),
            status: 401,
        }
    }

    pub fn forbidden(message: &str) -> Self {
        Self {
            code: "FORBIDDEN".to_string(),
            message: message.to_string(),
            status: 403,
        }
    }

    pub fn not_found(resource: &str) -> Self {
        Self {
            code: "NOT_FOUND".to_string(),
            message: format!("{} not found", resource),
            status: 404,
        }
    }

    pub fn internal(message: &str) -> Self {
        Self {
            code: "INTERNAL_ERROR".to_string(),
            message: message.to_string(),
            status: 500,
        }
    }

    pub fn rate_limited(retry_after: u64) -> Self {
        Self {
            code: "RATE_LIMITED".to_string(),
            message: format!("Too many requests. Retry after {} seconds.", retry_after),
            status: 429,
        }
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl ResponseError for ApiError {
    fn error_response(&self) -> HttpResponse {
        let body = ErrorResponse {
            error: ErrorDetail {
                code: self.code.clone(),
                message: self.message.clone(),
            },
        };

        match self.status {
            400 => HttpResponse::BadRequest().json(body),
            401 => HttpResponse::Unauthorized().json(body),
            403 => HttpResponse::Forbidden().json(body),
            404 => HttpResponse::NotFound().json(body),
            429 => HttpResponse::TooManyRequests().json(body),
            _ => HttpResponse::InternalServerError().json(body),
        }
    }
}

#[derive(Serialize)]
struct ErrorResponse {
    error: ErrorDetail,
}

#[derive(Serialize)]
struct ErrorDetail {
    code: String,
    message: String,
}

impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        // SECURITY: Log the full error internally but return generic message
        log::error!("Internal error: {}", err);
        ApiError::internal("An internal error occurred. Please try again later.")
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        // SECURITY: Log the full error internally but return generic message
        // Never expose database details to clients
        log::error!("Database error: {}", err);
        ApiError::internal("A database error occurred. Please try again later.")
    }
}
