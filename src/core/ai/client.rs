use crate::core::errors::AppError;
use crate::core::ai::models::*;
use crate::core::ai::provider::AIProvider;
use reqwest::Client;
use crate::core::cache::CacheManager;
use std::env;
use async_trait::async_trait;
use tracing;

pub struct GeminiProvider {
    client: Client,
    api_key: String,
}

impl GeminiProvider {
    pub fn new() -> Self {
        let api_key = env::var("GOOGLE_AI_KEY").expect("GOOGLE_AI_KEY must be set");
        Self {
            client: Client::new(),
            api_key,
        }
    }
}

#[async_trait]
impl AIProvider for GeminiProvider {
    async fn process_command(&self, prompt: &str) -> Result<AICommandResponse, AppError> {
        let system_prompt = r#"
            You are the "Brain" of an AI-Native ERP System. 
            Your goal is to understand user commands and convert them into system actions.
            
            Return ONLY a valid JSON object in the following format:
            {
                "message": "A friendly confirmation message in Vietnamese clearly explaining what was understood and done.",
                "action": {
                    "type": "create_department",
                    "payload": { "name": "Department Name", "parent_id": null }
                }
            }
            
            Actions available (all fields MUST be inside "payload"):
            - create_department: "action": { "type": "create_department", "payload": { "name": string, "parent_id": string | null } }
            - assign_role: "action": { "type": "assign_role", "payload": { "user_email": string, "role_code": string } }
            - navigate: "action": { "type": "navigate", "payload": { "path": string } }
            
            Valid paths for navigation:
            - "Tổng quan": "/"
            - "Sơ đồ tổ chức" or "Phòng ban": "/iam/org"
            - "Phân quyền" or "Roles": "/iam/roles"
            - "Duyệt yêu cầu": "/iam/approvals"
            - "Nhân sự" or "Nhân viên": "/employees"
            - "Audit Log" or "Nhật ký": "/audit"

            If no specific action is identified, set "action" to null.
            If the prompt is just a question, answer it in "message" and set "action" to null.
            Always reply in Vietnamese for the "message".
        "#;

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent?key={}",
            self.api_key
        );

        let request = GeminiRequest {
            system_instruction: Some(GeminiSystemInstruction {
                parts: vec![GeminiPart { text: system_prompt.to_string() }],
            }),
            contents: vec![
                GeminiContent {
                    role: "user".to_string(),
                    parts: vec![GeminiPart { text: prompt.to_string() }],
                },
            ],
        };

        let response = self.client.post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("Gemini API error: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(AppError::Internal(format!("Gemini API error (Status {}): {}", status, text)));
        }

        let gemini_res: GeminiResponse = response.json().await
            .map_err(|e| AppError::Internal(format!("Failed to parse Gemini response: {}", e)))?;

        let text = gemini_res.candidates.first()
            .and_then(|c| c.content.parts.first())
            .map(|p| p.text.clone())
            .ok_or_else(|| AppError::Internal("Empty response from AI".to_string()))?;

        extract_json_response(&text)
    }
}

pub struct OllamaProvider {
    client: Client,
    model: String,
    base_url: String,
}

impl OllamaProvider {
    pub fn new() -> Self {
        let model = env::var("OLLAMA_MODEL").unwrap_or_else(|_| "deepseek-r1:7b".to_string());
        let base_url = env::var("OLLAMA_BASE_URL").unwrap_or_else(|_| "http://localhost:11434".to_string());
        Self {
            client: Client::new(),
            model,
            base_url,
        }
    }
}

#[async_trait]
impl AIProvider for OllamaProvider {
    async fn process_command(&self, prompt: &str) -> Result<AICommandResponse, AppError> {
        let system_prompt = r#"
            You are the "Brain" of an AI-Native ERP System. 
            Your goal is to understand user commands and convert them into system actions.
            
            Return ONLY a valid JSON object in the following format:
            {
                "message": "A friendly confirmation message in Vietnamese clearly explaining what was understood and done.",
                "action": {
                    "type": "create_department",
                    "payload": { "name": "Department Name", "parent_id": null }
                }
            }
            
            Actions available (all fields MUST be inside "payload"):
            - create_department: "action": { "type": "create_department", "payload": { "name": string, "parent_id": string | null } }
            - assign_role: "action": { "type": "assign_role", "payload": { "user_email": string, "role_code": string } }
            - navigate: "action": { "type": "navigate", "payload": { "path": string } }
            
            Valid paths for navigation:
            - "Tổng quan": "/"
            - "Sơ đồ tổ chức" or "Phòng ban": "/iam/org"
            - "Phân quyền" or "Roles": "/iam/roles"
            - "Duyệt yêu cầu": "/iam/approvals"
            - "Nhân sự" or "Nhân viên": "/employees"
            - "Audit Log" or "Nhật ký": "/audit"

            If no specific action is identified, set "action" to null.
            If the prompt is just a question, answer it in "message" and set "action" to null.
            Always reply in Vietnamese for the "message".
        "#;
        
        let url = format!("{}/api/chat", self.base_url);
        
        let request = serde_json::json!({
            "model": self.model,
            "messages": [
                { "role": "system", "content": system_prompt },
                { "role": "user", "content": prompt }
            ],
            "stream": false,
            "format": "json"
        });

        let response = self.client.post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("Ollama API error: {}", e)))?;

        let res_body: serde_json::Value = response.json().await
            .map_err(|e| AppError::Internal(format!("Failed to parse Ollama response: {}", e)))?;

        let text = res_body["message"]["content"].as_str()
            .ok_or_else(|| AppError::Internal("Empty response from Ollama".to_string()))?;

        extract_json_response(text)
    }
}

pub struct OpenAIProvider {
    client: Client,
    api_key: String,
    model: String,
    base_url: String,
}

impl OpenAIProvider {
    pub fn new() -> Self {
        let api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set");
        let model = env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4o".to_string());
        let base_url = env::var("OPENAI_BASE_URL").unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
        Self {
            client: Client::new(),
            api_key,
            model,
            base_url,
        }
    }
}

#[async_trait]
impl AIProvider for OpenAIProvider {
    async fn process_command(&self, prompt: &str) -> Result<AICommandResponse, AppError> {
        let url = format!("{}/chat/completions", self.base_url);
        let system_prompt = r#"
            You are the "Brain" of an AI-Native ERP System. 
            Return ONLY a valid JSON object in the following format:
            {
                "message": "Phản hồi bằng tiếng Việt",
                "action": { "type": "navigate", "payload": { "path": "/" } }
            }
            Reply in Vietnamese.
        "#;

        let request = serde_json::json!({
            "model": self.model,
            "messages": [
                { "role": "system", "content": system_prompt },
                { "role": "user", "content": prompt }
            ],
            "response_format": { "type": "json_object" }
        });

        let response = self.client.post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("OpenAI API error: {}", e)))?;

        let res_body: serde_json::Value = response.json().await
            .map_err(|e| AppError::Internal(format!("Failed to parse OpenAI response: {}", e)))?;

        let text = res_body["choices"][0]["message"]["content"].as_str()
            .ok_or_else(|| AppError::Internal("Empty response from OpenAI".to_string()))?;

        extract_json_response(text)
    }
}

pub struct AIClient {
    provider: Box<dyn AIProvider>,
    cache: CacheManager,
}

impl AIClient {
    pub fn new(cache: CacheManager) -> Self {
        let provider_type = env::var("AI_PROVIDER").unwrap_or_else(|_| "gemini".to_string());
        let provider: Box<dyn AIProvider> = match provider_type.as_str() {
            "gemini" => Box::new(GeminiProvider::new()),
            "ollama" => Box::new(OllamaProvider::new()),
            "openai" => Box::new(OpenAIProvider::new()),
            _ => panic!("Unsupported AI provider: {}", provider_type),
        };

        Self { provider, cache }
    }

    pub async fn process_command(&self, prompt: &str) -> Result<AICommandResponse, AppError> {
        let cache_key = prompt.trim().to_lowercase();
        
        // 1. Check Redis Cache
        match self.cache.get_ai_cache(&cache_key).await {
            Ok(Some(cached)) => {
                if let Ok(response) = serde_json::from_value::<AICommandResponse>(cached) {
                    tracing::info!("🚀 AI Cache result returned for: {}", cache_key);
                    return Ok(response);
                }
            },
            Ok(None) => {
                // Already logged miss inside get_ai_cache
            },
            Err(e) => {
                tracing::warn!("⚠️ Redis lookup error for {}: {}", cache_key, e);
            }
        }

        // 2. Call AI Provider
        tracing::info!("🤖 AI Cache miss, calling provider: {}", cache_key);
        let response = self.provider.process_command(prompt).await?;
        
        // 3. Save to Redis Cache (TTL 24h = 86400s)
        if let Ok(val) = serde_json::to_value(&response) {
            if let Err(e) = self.cache.set_ai_cache(&cache_key, &val, 86400).await {
                tracing::error!("❌ Failed to save AI response to Redis: {}", e);
            }
        }
        
        Ok(response)
    }
}

fn extract_json_response(text: &str) -> Result<AICommandResponse, AppError> {
    let json_text = if let Some(start) = text.find('{') {
        if let Some(end) = text.rfind('}') {
            &text[start..=end]
        } else { text }
    } else { text };

    serde_json::from_str(json_text)
        .map_err(|e| AppError::Internal(format!("AI returned invalid JSON: {}. Original: {}", e, text)))
}
