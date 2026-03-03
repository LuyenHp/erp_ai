//! # Auth Middleware – JWT Extraction
//!
//! Extract JWT token từ `Authorization: Bearer {token}` header,
//! decode và set `AuthContext` vào request extensions cho downstream handlers.

use axum::{
    body::Body,
    http::{header::AUTHORIZATION, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde_json::json;

use crate::core::iam::middleware::AuthContext;

use super::models::Claims;

/// JWT secret key – đọc từ env, fallback cho dev.
fn get_jwt_secret() -> String {
    std::env::var("JWT_SECRET").unwrap_or_else(|_| "dev-secret-change-in-production".into())
}

/// Middleware extract JWT → AuthContext.
///
/// Flow:
/// 1. Extract `Authorization: Bearer {token}` header
/// 2. Decode JWT token, validate signature & expiration
/// 3. Set `AuthContext { user_id, tenant_id }` vào request extensions
/// 4. Downstream handlers/middleware (RequirePermission) đọc từ extensions
pub async fn auth_middleware(
    mut request: Request<Body>,
    next: Next,
) -> Response {
    // Extract token từ header
    let token = match extract_bearer_token(&request) {
        Some(t) => t,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "error": "unauthorized",
                    "message": "Missing or invalid Authorization header. Expected: Bearer <token>"
                })),
            )
                .into_response();
        }
    };

    // Decode JWT
    let secret = get_jwt_secret();
    let token_data = match decode::<Claims>(
        &token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    ) {
        Ok(data) => data,
        Err(e) => {
            let message = match e.kind() {
                jsonwebtoken::errors::ErrorKind::ExpiredSignature => "Token has expired",
                jsonwebtoken::errors::ErrorKind::InvalidToken => "Invalid token format",
                _ => "Token validation failed",
            };
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "error": "unauthorized",
                    "message": message
                })),
            )
                .into_response();
        }
    };

    // Set AuthContext vào extensions cho RequirePermission middleware và handlers
    let auth_ctx = AuthContext {
        user_id: token_data.claims.sub,
        tenant_id: token_data.claims.tenant_id,
    };
    request.extensions_mut().insert(auth_ctx);

    next.run(request).await
}

/// Extract bearer token từ Authorization header.
fn extract_bearer_token(request: &Request<Body>) -> Option<String> {
    request
        .headers()
        .get(AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .map(|t| t.to_string())
}
