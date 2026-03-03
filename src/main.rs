//! # ERP AI – Entry Point
//!
//! Axum HTTP server with auto-migration, JWT auth, and IAM routes.

mod core;
mod skills;

use axum::{middleware, routing::get, Json, Router};
use serde_json::{json, Value};
use std::net::SocketAddr;
use tracing_subscriber::EnvFilter;

use crate::core::auth;
use crate::core::iam::routes::iam_routes;

/// Health check endpoint
async fn health_check() -> Json<Value> {
    Json(json!({
        "status": "healthy",
        "service": "erp_ai",
        "version": "0.1.0"
    }))
}

use std::sync::Arc;
use crate::core::ai::client::AIClient;
use crate::core::cache::CacheManager;
use crate::core::AppState;

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    // Load .env file (nếu có)
    let _ = dotenvy::dotenv();

    // Database connection pool
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");

    let pool = core::db::create_pool(&database_url)
        .await
        .expect("Failed to create database pool");

    // Auto-run migrations
    core::db::run_migrations(&pool)
        .await
        .expect("Failed to run migrations");

    // Initialize AppState
    let cache = CacheManager::new();
    let ai = Arc::new(AIClient::new(cache.clone()));
    let state = AppState { pool, ai, cache };

    // Auth routes (public – no JWT required)
    let auth_routes = Router::new()
        .route("/register", axum::routing::post(auth::handlers::register))
        .route("/login", axum::routing::post(auth::handlers::login));

    // API routes (protected – JWT required)
    let api_routes = iam_routes()
        .route("/auth/me", get(auth::handlers::me))
        .nest("/ai", crate::core::ai::routes())
        .layer(middleware::from_fn(auth::middleware::auth_middleware));

    // Build main router
    let app = Router::new()
        .route("/health", get(health_check))
        .nest("/auth", auth_routes)
        .nest("/api", api_routes)
        .with_state(state);

    // Bind address
    let host = std::env::var("APP_HOST").unwrap_or_else(|_| "0.0.0.0".into());
    let port: u16 = std::env::var("APP_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);
    let addr: SocketAddr = format!("{host}:{port}")
        .parse()
        .expect("Invalid APP_HOST:APP_PORT");

    tracing::info!("🚀 ERP AI server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind address");

    axum::serve(listener, app)
        .await
        .expect("Server error");
}
