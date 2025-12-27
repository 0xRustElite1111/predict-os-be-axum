use crate::clients::ai::AiClient;
use crate::types::AiAnalysis;
use crate::{AppError, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::warn;

const GROK_API_URL: &str = "https://api.x.ai/v1/chat/completions";
const MAX_RETRIES: u32 = 3;
const TIMEOUT_SECS: u64 = 120;

#[derive(Debug, Serialize)]
struct GrokRequest {
    model: String,
    messages: Vec<Message>,
    response_format: ResponseFormat,
    temperature: f64,
}

#[derive(Debug, Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    type_: String,
}

#[derive(Debug, Deserialize)]
struct GrokResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ChoiceMessage,
}

#[derive(Debug, Deserialize)]
struct ChoiceMessage {
    content: String,
}

pub struct GrokClient {
    client: Client,
    api_key: String,
}

impl GrokClient {
    pub fn new() -> Result<Self> {
        let api_key = std::env::var("GROK_API_KEY")
            .map_err(|_| AppError::Validation("GROK_API_KEY not set".to_string()))?;

        let client = Client::builder()
            .timeout(Duration::from_secs(TIMEOUT_SECS))
            .build()
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to create HTTP client: {}", e)))?;

        Ok(Self { client, api_key })
    }

    async fn call_with_retry(&self, prompt: String) -> Result<AiAnalysis> {
        let mut last_error = None;

        for attempt in 0..MAX_RETRIES {
            match self.call_api(&prompt).await {
                Ok(analysis) => {
                    if attempt > 0 {
                        tracing::info!("Grok API call succeeded on attempt {}", attempt + 1);
                    }
                    return Ok(analysis);
                }
                Err(e) => {
                    last_error = Some(e);
                    if attempt < MAX_RETRIES - 1 {
                        let delay = Duration::from_millis(2_u64.pow(attempt) * 100);
                        warn!("Grok API call failed, retrying in {:?}...", delay);
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            AppError::ExternalApi("Grok API call failed after retries".to_string())
        }))
    }

    async fn call_api(&self, prompt: &str) -> Result<AiAnalysis> {
        let request = GrokRequest {
            model: "grok-beta".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            response_format: ResponseFormat {
                type_: "json_object".to_string(),
            },
            temperature: 0.7,
        };

        let response = self
            .client
            .post(GROK_API_URL)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| AppError::ExternalApi(format!("Grok API request failed: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(AppError::ExternalApi(format!(
                "Grok API returned {}: {}",
                status, error_text
            )));
        }

        let grok_response: GrokResponse = response
            .json()
            .await
            .map_err(|e| AppError::ExternalApi(format!("Failed to parse Grok response: {}", e)))?;

        let content = grok_response
            .choices
            .first()
            .and_then(|c| Some(c.message.content.clone()))
            .ok_or_else(|| AppError::ExternalApi("No content in Grok response".to_string()))?;

        // Parse JSON from content
        let analysis: AiAnalysis = serde_json::from_str(&content)
            .map_err(|e| AppError::ExternalApi(format!("Failed to parse AI analysis JSON: {}", e)))?;

        Ok(analysis)
    }
}

#[async_trait::async_trait]
impl AiClient for GrokClient {
    async fn analyze_markets(&self, prompt: String) -> Result<AiAnalysis> {
        self.call_with_retry(prompt).await
    }

    fn provider_name(&self) -> &'static str {
        "grok"
    }
}

