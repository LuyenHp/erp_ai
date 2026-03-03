use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct AICommandRequest {
    pub prompt: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AICommandResponse {
    pub message: String,
    pub action: Option<AIAction>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum AIAction {
    #[serde(rename = "create_department")]
    CreateDepartment { name: String, parent_id: Option<String> },
    #[serde(rename = "assign_role")]
    AssignRole { user_email: String, role_code: String },
    #[serde(rename = "navigate")]
    Navigate { path: String },
}

// Gemini API structures
#[derive(Debug, Serialize)]
pub struct GeminiRequest {
    pub contents: Vec<GeminiContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_instruction: Option<GeminiSystemInstruction>,
}

#[derive(Debug, Serialize)]
pub struct GeminiSystemInstruction {
    pub parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize)]
pub struct GeminiContent {
    pub role: String,
    pub parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize)]
pub struct GeminiPart {
    pub text: String,
}

#[derive(Debug, Deserialize)]
pub struct GeminiResponse {
    pub candidates: Vec<GeminiCandidate>,
}

#[derive(Debug, Deserialize)]
pub struct GeminiCandidate {
    pub content: GeminiCandidateContent,
}

#[derive(Debug, Deserialize)]
pub struct GeminiCandidateContent {
    pub parts: Vec<GeminiCandidatePart>,
}

#[derive(Debug, Deserialize)]
pub struct GeminiCandidatePart {
    pub text: String,
}
