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

    pub fn conflict(code: &str, message: &str) -> Self {
        Self {
            code: code.to_string(),
            message: message.to_string(),
            status: 409,
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
            409 => HttpResponse::Conflict().json(body),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bad_request_error() {
        let err = ApiError::bad_request("INVALID_INPUT", "Input is invalid");
        assert_eq!(err.code, "INVALID_INPUT");
        assert_eq!(err.message, "Input is invalid");
        assert_eq!(err.status, 400);
    }

    #[test]
    fn test_unauthorized_error() {
        let err = ApiError::unauthorized("Not authenticated");
        assert_eq!(err.code, "UNAUTHORIZED");
        assert_eq!(err.message, "Not authenticated");
        assert_eq!(err.status, 401);
    }

    #[test]
    fn test_forbidden_error() {
        let err = ApiError::forbidden("Access denied");
        assert_eq!(err.code, "FORBIDDEN");
        assert_eq!(err.message, "Access denied");
        assert_eq!(err.status, 403);
    }

    #[test]
    fn test_not_found_error() {
        let err = ApiError::not_found("Market");
        assert_eq!(err.code, "NOT_FOUND");
        assert_eq!(err.message, "Market not found");
        assert_eq!(err.status, 404);
    }

    #[test]
    fn test_internal_error() {
        let err = ApiError::internal("Something went wrong");
        assert_eq!(err.code, "INTERNAL_ERROR");
        assert_eq!(err.message, "Something went wrong");
        assert_eq!(err.status, 500);
    }

    #[test]
    fn test_rate_limited_error() {
        let err = ApiError::rate_limited(60);
        assert_eq!(err.code, "RATE_LIMITED");
        assert_eq!(err.message, "Too many requests. Retry after 60 seconds.");
        assert_eq!(err.status, 429);
    }

    #[test]
    fn test_conflict_error() {
        let err = ApiError::conflict("DUPLICATE", "Resource already exists");
        assert_eq!(err.code, "DUPLICATE");
        assert_eq!(err.message, "Resource already exists");
        assert_eq!(err.status, 409);
    }

    #[test]
    fn test_display_trait() {
        let err = ApiError::bad_request("TEST_CODE", "Test message");
        let display = format!("{}", err);
        assert_eq!(display, "TEST_CODE: Test message");
    }

    #[test]
    fn test_debug_trait() {
        let err = ApiError::bad_request("TEST", "msg");
        let debug = format!("{:?}", err);
        assert!(debug.contains("TEST"));
        assert!(debug.contains("msg"));
    }

    #[test]
    fn test_from_anyhow_error() {
        let anyhow_err = anyhow::anyhow!("Some internal error");
        let api_err: ApiError = anyhow_err.into();
        assert_eq!(api_err.code, "INTERNAL_ERROR");
        assert_eq!(api_err.status, 500);
        // Message should be generic, not exposing internal details
        assert!(!api_err.message.contains("Some internal error"));
    }

    #[test]
    fn test_all_status_codes_unique() {
        let errors = vec![
            ApiError::bad_request("", ""),
            ApiError::unauthorized(""),
            ApiError::forbidden(""),
            ApiError::not_found(""),
            ApiError::internal(""),
            ApiError::rate_limited(0),
            ApiError::conflict("", ""),
        ];
        let statuses: Vec<u16> = errors.iter().map(|e| e.status).collect();
        assert_eq!(statuses, vec![400, 401, 403, 404, 500, 429, 409]);
    }
}
