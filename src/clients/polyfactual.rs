use crate::types::{Citation, PolyfactualResearchResponse, ResponseMetadata};
use crate::{AppError, Result};
use chrono::Utc;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tracing::info;

const POLYFACTUAL_API_URL: &str = "https://api.polyfactual.com/v1/research";
const MAX_QUERY_LENGTH: usize = 1000;
const TIMEOUT_SECS: u64 = 300; // 5 minutes

#[derive(Debug, Serialize)]
struct PolyfactualRequest {
    query: String,
}

#[derive(Debug, Deserialize)]
struct PolyfactualResponse {
    answer: String,
    citations: Vec<PolyfactualCitation>,
}

#[derive(Debug, Deserialize)]
struct PolyfactualCitation {
    source: String,
    url: Option<String>,
    relevance: Option<f64>,
}

pub struct PolyfactualClient {
    client: Client,
    api_key: String,
}

impl PolyfactualClient {
    pub fn new() -> Result<Self> {
        let api_key = std::env::var("POLYFACTUAL_API_KEY")
            .map_err(|_| AppError::Validation("POLYFACTUAL_API_KEY not set".to_string()))?;

        let client = Client::builder()
            .timeout(Duration::from_secs(TIMEOUT_SECS))
            .build()
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to create HTTP client: {}", e)))?;

        Ok(Self { client, api_key })
    }

    pub async fn research(&self, query: String) -> Result<PolyfactualResearchResponse> {
        let start = Instant::now();

        // Validate query length
        if query.len() > MAX_QUERY_LENGTH {
            return Err(AppError::Validation(format!(
                "Query exceeds maximum length of {} characters",
                MAX_QUERY_LENGTH
            )));
        }

        info!("Making Polyfactual research request: {}", query);

        let request = PolyfactualRequest {
            query: query.clone(),
        };

        let response = self
            .client
            .post(POLYFACTUAL_API_URL)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| AppError::ExternalApi(format!("Polyfactual API request failed: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(AppError::ExternalApi(format!(
                "Polyfactual API returned {}: {}",
                status, error_text
            )));
        }

        let polyfactual_response: PolyfactualResponse = response
            .json()
            .await
            .map_err(|e| AppError::ExternalApi(format!("Failed to parse Polyfactual response: {}", e)))?;

        let execution_time = start.elapsed().as_millis() as u64;

        Ok(PolyfactualResearchResponse {
            answer: polyfactual_response.answer,
            citations: polyfactual_response
                .citations
                .into_iter()
                .map(|c| Citation {
                    source: c.source,
                    url: c.url,
                    relevance: c.relevance.unwrap_or(0.0),
                })
                .collect(),
            metadata: ResponseMetadata {
                timestamp: Utc::now().to_rfc3339(),
                execution_time_ms: execution_time,
                model_used: None,
                retries: 0,
            },
        })
    }
}

