pub mod client;
pub mod handlers;
pub mod models;
pub mod provider;

use axum::{routing::post, Router};
use crate::core::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/command", post(handlers::process_ai_command))
}
