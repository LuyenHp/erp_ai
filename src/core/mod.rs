pub mod auth;
pub mod db;
pub mod errors;
pub mod iam;
pub mod ai;
pub mod cache;

use std::sync::Arc;
use crate::core::ai::client::AIClient;
use crate::core::cache::CacheManager;

#[derive(Clone)]
pub struct AppState {
    pub pool: sqlx::PgPool,
    pub ai: Arc<AIClient>,
    pub cache: CacheManager,
}
