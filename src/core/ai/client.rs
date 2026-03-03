use crate::core::errors::AppError;
use crate::core::ai::models::*;
use reqwest::Client;
use std::env;

pub struct GeminiClient {
    client: Client,
    api_key: String,
}

impl GeminiClient {
    pub fn new() -> Self {
        let api_key = env::var("GOOGLE_AI_KEY").expect("GOOGLE_AI_KEY must be set");
        Self {
            client: Client::new(),
            api_key,
        }
    }

    pub async fn process_command(&self, prompt: &str) -> Result<AICommandResponse, AppError> {
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
            
            Actions available:
            - create_department: { "name": string, "parent_id": string | null }
            - assign_role: { "user_email": string, "role_code": string }
            - navigate: { "path": string }
            
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

        // Extract JSON from potential markdown code blocks
        let json_text = if let Some(start) = text.find('{') {
            if let Some(end) = text.rfind('}') {
                &text[start..=end]
            } else { &text }
        } else { &text };

        let ai_res: AICommandResponse = serde_json::from_str(json_text)
            .map_err(|e| AppError::Internal(format!("AI returned invalid JSON: {}. Original: {}", e, text)))?;

        Ok(ai_res)
    }
}
