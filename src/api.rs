use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use reqwest::Client;
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

#[derive(Debug, Serialize, Deserialize)]
pub struct Part {
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Content {
    pub role: String,
    pub parts: Vec<Part>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GoogleSearch {}

#[derive(Debug, Serialize, Deserialize)]
pub struct Tool {
    #[serde(rename = "googleSearch")]
    pub google_search: Option<GoogleSearch>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiRequestInner {
    pub contents: Vec<Content>,
    pub tools: Vec<Tool>,
    pub session_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiRequest {
    pub project: String,
    pub model: String,
    pub user_agent: String,
    pub request_type: String,
    pub request_id: String,
    pub request: GeminiRequestInner,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GroundingChunk {
    pub web: Option<WebSource>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct WebSource {
    pub uri: String,
    pub title: String,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GroundingSupport {
    #[serde(default)]
    pub segment: Segment,
    #[serde(default)]
    pub grounding_chunk_indices: Vec<usize>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Segment {
    #[serde(default)]
    pub start_index: usize,
    #[serde(default)]
    pub end_index: usize,
    #[serde(default)]
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GroundingMetadata {
    #[serde(default)]
    pub grounding_chunks: Vec<GroundingChunk>,
    #[serde(default)]
    pub grounding_supports: Vec<GroundingSupport>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeminiResponseInner {
    pub candidates: Vec<Candidate>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    pub content: Option<Content>,
    pub grounding_metadata: Option<GroundingMetadata>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeminiResponse {
    pub response: GeminiResponseInner,
}

pub struct ApiClient {
    client: Client,
    token: String,
    project_id: String,
}

impl ApiClient {
    pub fn new(token: String, project_id: String) -> Self {
        let mut builder = Client::builder();
        
        if std::env::var("HTTPS_PROXY").is_ok() || std::env::var("https_proxy").is_ok() {
            builder = builder.danger_accept_invalid_certs(true);
        }

        let user_agent = "antigravity/cli/1.0.9 darwin/arm64".to_string();
        builder = builder.user_agent(user_agent);

        Self {
            client: builder.build().unwrap_or_else(|_| Client::new()),
            token,
            project_id,
        }
    }

    pub async fn search(&self, query: &str) -> Result<GeminiResponse> {
        let endpoint = "https://daily-cloudcode-pa.googleapis.com/v1internal:generateContent";
        
        // Retrieve model from GSEARCH_MODEL env var or default to gemini-3.1-flash-lite
        let model = std::env::var("GSEARCH_MODEL")
            .unwrap_or_else(|_| "gemini-3.1-flash-lite".to_string());

        let request_id = format!("agent-{}", uuid_v4_short());

        let mut h = DefaultHasher::new();
        query.hash(&mut h);
        let numeric_hash = (h.finish() & 0x7FFF_FFFF_FFFF_FFFF) as i64;
        let session_id = format!("-{}", numeric_hash);

        let request = GeminiRequest {
            project: self.project_id.clone(),
            model: model.clone(),
            user_agent: "antigravity".to_string(),
            request_type: "agent".to_string(),
            request_id,
            request: GeminiRequestInner {
                contents: vec![Content {
                    role: "user".to_string(),
                    parts: vec![Part { text: query.to_string() }],
                }],
                tools: vec![Tool { google_search: Some(GoogleSearch {}) }],
                session_id,
            },
        };

        let response = self.client.post(endpoint)
            .header("Authorization", format!("Bearer {}", self.token))
            .json(&request)
            .send()
            .await
            .context("Failed to send request to Gemini API")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("API request failed with status {}: {}", status, error_text);
        }

        let gemini_response: GeminiResponse = response.json()
            .await
            .context("Failed to parse Gemini API response")?;

        Ok(gemini_response)
    }
}

fn uuid_v4_short() -> String {
    let mut bytes = [0u8; 16];
    for b in bytes.iter_mut() {
        *b = rand::random::<u8>();
    }
    // Set UUID v4 version (4) and variant (10xx) bits
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    
    // Format as string without hyphens or full length for simplicity, matching "uuid::Uuid::new_v4()"
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0], bytes[1], bytes[2], bytes[3],
        bytes[4], bytes[5],
        bytes[6], bytes[7],
        bytes[8], bytes[9],
        bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15]
    )
}
