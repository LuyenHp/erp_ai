use async_trait::async_trait;
use crate::core::ai::models::AICommandResponse;
use crate::core::errors::AppError;

#[async_trait]
pub trait AIProvider: Send + Sync {
    async fn process_command(&self, prompt: &str) -> Result<AICommandResponse, AppError>;
}
