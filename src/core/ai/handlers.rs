use axum::{extract::State, Json, response::IntoResponse};
use crate::core::ai::models::*;
use crate::core::ai::client::AIClient;
use crate::core::errors::AppError;
use once_cell::sync::Lazy;
use sqlx::{Pool, Postgres};

static AI_CLIENT: Lazy<AIClient> = Lazy::new(|| AIClient::new());

pub async fn process_ai_command(
    State(pool): State<Pool<Postgres>>,
    auth: axum::Extension<crate::core::iam::middleware::AuthContext>,
    Json(payload): Json<AICommandRequest>,
) -> Result<impl IntoResponse, AppError> {
    let mut response = AI_CLIENT.process_command(&payload.prompt).await?;
    
    // Execute action if provided
    if let Some(action) = &response.action {
        match action {
            AIAction::CreateDepartment { name, parent_id } => {
                // Execute creation logic (simplified version of IAM handler)
                let parent_path: Option<String> = if let Some(pid) = parent_id {
                    sqlx::query_scalar("SELECT path FROM departments WHERE id = $1 AND tenant_id = $2")
                        .bind(uuid::Uuid::parse_str(pid).map_err(|_| AppError::BadRequest("Invalid parent ID".into()))?)
                        .bind(auth.tenant_id)
                        .fetch_optional(&pool)
                        .await
                        .map_err(AppError::from)?
                } else { None };

                let path = if let Some(pp) = parent_path { format!("{}.{}", pp, name.to_lowercase().replace(' ', "_")) } 
                           else { name.to_lowercase().replace(' ', "_") };

                sqlx::query("INSERT INTO departments (tenant_id, name, code, path, parent_id) VALUES ($1, $2, $3, $4::ltree, $5)")
                    .bind(auth.tenant_id)
                    .bind(name)
                    .bind(name.to_lowercase().replace(' ', "_"))
                    .bind(path)
                    .bind(parent_id.as_ref().and_then(|id| uuid::Uuid::parse_str(id).ok()))
                    .execute(&pool)
                    .await
                    .map_err(AppError::from)?;
                
                response.message = format!("✅ Đã tạo phòng ban: {}", name);
            },
            _ => {} // Other actions handled by frontend or logic TBD
        }
    }

    Ok(Json(response))
}
