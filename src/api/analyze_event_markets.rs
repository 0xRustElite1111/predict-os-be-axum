use axum::{extract::State, Json};
use chrono::Utc;
use std::sync::Arc;
use std::time::Instant;

use crate::api::AppState;
use crate::clients::ai::prompts::build_analysis_prompt;
use crate::clients::{create_ai_client, AiProvider, DomeClient};
use crate::types::{AnalyzeEventMarketsRequest, AnalyzeEventMarketsResponse, ResponseMetadata};
use crate::Result;

pub async fn handler(
    State(state): State<Arc<AppState>>,
    Json(request): Json<AnalyzeEventMarketsRequest>,
) -> Result<Json<AnalyzeEventMarketsResponse>> {
    let start = Instant::now();
    let mut retries = 0;

    // Validate request
    if request.url.is_empty() {
        return Err(crate::AppError::Validation("URL is required".to_string()));
    }

    // Determine AI provider
    let provider = match request.model.as_deref() {
        Some("openai") => AiProvider::OpenAi,
        _ => AiProvider::Grok, // Default to Grok
    };

    // Fetch market data from Dome API
    let market_data = state
        .dome_clients
        .dome
        .get_market_by_url(&request.url)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch market data: {}", e);
            e
        })?;

    // Build AI prompt
    let prompt = build_analysis_prompt(&market_data, request.question.as_ref());
    println!("prompt ------------> {:?}", prompt);
    // Call AI with retry logic (handled in client)
    println!("provider ------------> {:?}", provider);
    let ai_client = create_ai_client(provider.clone())?;

    tracing::info!("ai_client ------------> {}", ai_client.provider_name());
    let analysis = match ai_client.analyze_markets(prompt).await {
        Ok(analysis) => analysis,
        Err(e) => {
            // Retry once with different provider if Grok fails
            if matches!(provider, AiProvider::Grok) {
                retries = 1;
                tracing::warn!("Grok failed, retrying with OpenAI");
                let openai_client = create_ai_client(AiProvider::OpenAi)?;
                openai_client
                    .analyze_markets(build_analysis_prompt(
                        &market_data,
                        request.question.as_ref(),
                    ))
                    .await?
            } else {
                return Err(e);
            }
        }
    };

    let execution_time = start.elapsed().as_millis() as u64;

    let recommendation = analysis.recommendation.clone();
    Ok(Json(AnalyzeEventMarketsResponse {
        recommendation,
        analysis,
        market_data,
        metadata: ResponseMetadata {
            timestamp: Utc::now().to_rfc3339(),
            execution_time_ms: execution_time,
            model_used: Some(ai_client.provider_name().to_string()),
            retries,
        },
    }))
}

pub struct Clients {
    pub dome: DomeClient,
}

impl Clients {
    pub fn new() -> Result<Self> {
        Ok(Self {
            dome: DomeClient::new()?,
        })
    }
}
