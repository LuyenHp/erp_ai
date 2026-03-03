use deadpool_redis::{Config, Connection, Pool, Runtime, redis};
use crate::core::errors::AppError;
use std::env;
use serde_json::Value;

#[derive(Clone)]
pub struct CacheManager {
    pool: Pool,
}

impl CacheManager {
    pub fn new() -> Self {
        let redis_url = env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());
        let cfg = Config::from_url(redis_url);
        let pool = cfg.create_pool(Some(Runtime::Tokio1)).expect("Failed to create Redis pool");
        
        Self { pool }
    }

    async fn get_conn(&self) -> Result<Connection, AppError> {
        self.pool.get().await.map_err(|e| AppError::Internal(format!("Redis pool error: {}", e)))
    }

    pub async fn get_ai_cache(&self, key: &str) -> Result<Option<Value>, AppError> {
        let mut conn = self.get_conn().await?;
        let val: Option<String> = redis::cmd("GET")
            .arg(format!("ai_cache:{}", key))
            .query_async(&mut conn)
            .await
            .map_err(|e| AppError::Internal(format!("Redis GET error: {}", e)))?;

        match val {
            Some(s) => {
                tracing::info!("🎯 Redis: Cache hit for key: {}", key);
                let json: Value = serde_json::from_str(&s)
                    .map_err(|e| AppError::Internal(format!("Redis data corrupted: {}", e)))?;
                Ok(Some(json))
            }
            None => {
                tracing::info!("⚪ Redis: Cache miss for key: {}", key);
                Ok(None)
            }
        }
    }

    pub async fn set_ai_cache(&self, key: &str, value: &Value, ttl_secs: u64) -> Result<(), AppError> {
        let mut conn = self.get_conn().await?;
        let json_str = serde_json::to_string(value)
            .map_err(|e| AppError::Internal(format!("JSON serialization error: {}", e)))?;

        let _: () = redis::cmd("SETEX")
            .arg(format!("ai_cache:{}", key))
            .arg(ttl_secs)
            .arg(json_str)
            .query_async(&mut conn)
            .await
            .map_err(|e| {
                tracing::error!("❌ Redis: SETEX error for key {}: {}", key, e);
                AppError::Internal(format!("Redis SETEX error: {}", e))
            })?;
        
        tracing::info!("💾 Redis: Cached AI response for: {}", key);
        Ok(())
    }
}
