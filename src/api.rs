use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use reqwest::Client;

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
pub struct GeminiRequestInner {
    pub contents: Vec<Content>,
    pub tools: Vec<Tool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeminiRequest {
    pub model: String,
    pub project: String,
    pub request: GeminiRequestInner,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroundingChunk {
    pub web: Option<WebSource>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WebSource {
    pub uri: String,
    pub title: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroundingSupport {
    pub segment: Segment,
    pub grounding_chunk_indices: Vec<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Segment {
    pub start_index: usize,
    pub end_index: usize,
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroundingMetadata {
    pub grounding_chunks: Vec<GroundingChunk>,
    pub grounding_supports: Vec<GroundingSupport>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeminiResponseInner {
    pub candidates: Vec<Candidate>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Candidate {
    pub content: Option<Content>,
    pub grounding_metadata: Option<GroundingMetadata>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeminiResponse {
    pub response: GeminiResponseInner,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadCodeAssistMetadata {
    pub ide_type: String,
    pub platform: String,
    pub plugin_type: String,
    pub duet_project: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadCodeAssistRequest {
    pub cloudaicompanion_project: Option<String>,
    pub metadata: LoadCodeAssistMetadata,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadCodeAssistResponse {
    pub cloudaicompanion_project: Option<String>,
    // other fields omitted as we only need the resolved project ID
}

pub struct ApiClient {
    client: Client,
    token: String,
    project_id: Option<String>,
}

impl ApiClient {
    pub fn new(token: String, project_id: Option<String>) -> Self {
        let mut builder = Client::builder();
        
        if std::env::var("HTTPS_PROXY").is_ok() || std::env::var("https_proxy").is_ok() {
            builder = builder.danger_accept_invalid_certs(true);
        }

        let user_agent = format!(
            "GeminiCLI/{}/gemini-3.1-flash-lite-preview ({os}; {arch}; cli)",
            env!("CARGO_PKG_VERSION"),
            os = std::env::consts::OS,
            arch = std::env::consts::ARCH,
        );
        builder = builder.user_agent(user_agent);

        Self {
            client: builder.build().unwrap_or_else(|_| Client::new()),
            token,
            project_id,
        }
    }

    pub async fn resolve_project_id(&self) -> Result<String> {
        let endpoint = "https://cloudcode-pa.googleapis.com/v1internal:loadCodeAssist";

        let request = LoadCodeAssistRequest {
            cloudaicompanion_project: self.project_id.clone(),
            metadata: LoadCodeAssistMetadata {
                ide_type: "IDE_UNSPECIFIED".to_string(),
                platform: "PLATFORM_UNSPECIFIED".to_string(),
                plugin_type: "GEMINI".to_string(),
                duet_project: self.project_id.clone(),
            },
        };

        let response = self.client.post(endpoint)
            .header("Authorization", format!("Bearer {}", self.token))
            .json(&request)
            .send()
            .await
            .context("Failed to send loadCodeAssist request")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            // Don't bail entirely, fallback if possible
            if let Some(pid) = &self.project_id {
                return Ok(pid.clone());
            }
            anyhow::bail!("loadCodeAssist failed with status {}: {}", status, error_text);
        }

        let load_res: LoadCodeAssistResponse = response.json()
            .await
            .context("Failed to parse loadCodeAssist response")?;

        if let Some(resolved_project) = load_res.cloudaicompanion_project {
            Ok(resolved_project)
        } else if let Some(pid) = &self.project_id {
            Ok(pid.clone())
        } else {
            anyhow::bail!("No project ID returned by loadCodeAssist, and no fallback provided.");
        }
    }

    pub async fn search(&self, query: &str) -> Result<GeminiResponse> {
        let resolved_project_id = self.resolve_project_id().await?;
        
        let endpoint = "https://cloudcode-pa.googleapis.com/v1internal:generateContent";
        
        let request = GeminiRequest {
            model: "gemini-3.1-flash-lite-preview".to_string(),
            project: resolved_project_id,
            request: GeminiRequestInner {
                contents: vec![Content {
                    role: "user".to_string(),
                    parts: vec![Part { text: query.to_string() }],
                }],
                tools: vec![Tool { google_search: Some(GoogleSearch {}) }],
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
