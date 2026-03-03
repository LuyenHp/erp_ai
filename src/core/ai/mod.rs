pub mod client;
pub mod handlers;
pub mod models;

use axum::{routing::post, Router};
use sqlx::{Pool, Postgres};

pub fn routes() -> Router<Pool<Postgres>> {
    Router::new()
        .route("/command", post(handlers::process_ai_command))
}
