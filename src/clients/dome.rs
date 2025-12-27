use crate::types::{MarketData, Outcome, Platform};
use crate::{AppError, Result};
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;
use url::Url;

const DOME_API_BASE: &str = "https://api.dome.xyz/v1";

#[derive(Debug, Deserialize)]
struct DomeMarketResponse {
    id: String,
    question: String,
    slug: Option<String>,
    ticker: Option<String>,
    #[allow(dead_code)]
    platform: String,
    outcomes: Vec<DomeOutcome>,
    volume_24h: Option<f64>,
    liquidity: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct DomeOutcome {
    id: String,
    name: String,
    price: f64,
    volume_24h: Option<f64>,
}

pub struct DomeClient {
    client: Client,
    api_key: String,
}

impl DomeClient {
    pub fn new() -> Result<Self> {
        let api_key = std::env::var("DOME_API_KEY")
            .map_err(|_| AppError::Validation("DOME_API_KEY not set".to_string()))?;

        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to create HTTP client: {}", e)))?;

        Ok(Self { client, api_key })
    }

    pub async fn get_market_by_url(&self, url: &str) -> Result<MarketData> {
        // Extract identifier from URL
        let identifier = self.extract_identifier(url)?;
        let platform = self.detect_platform(url)?;

        let endpoint = match platform {
            Platform::Polymarket => format!("{}/markets/polymarket/{}", DOME_API_BASE, identifier),
            Platform::Kalshi => format!("{}/markets/kalshi/{}", DOME_API_BASE, identifier),
        };

        let response = self
            .client
            .get(&endpoint)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
            .map_err(|e| AppError::ExternalApi(format!("Dome API request failed: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(AppError::ExternalApi(format!(
                "Dome API returned {}: {}",
                status, error_text
            )));
        }

        let dome_response: DomeMarketResponse = response
            .json()
            .await
            .map_err(|e| AppError::ExternalApi(format!("Failed to parse Dome response: {}", e)))?;

        Ok(MarketData {
            id: dome_response.id,
            question: dome_response.question,
            slug: dome_response.slug,
            ticker: dome_response.ticker,
            platform,
            outcomes: dome_response
                .outcomes
                .into_iter()
                .map(|o| Outcome {
                    id: o.id,
                    name: o.name,
                    price: o.price,
                    volume: o.volume_24h,
                })
                .collect(),
            volume: dome_response.volume_24h,
            liquidity: dome_response.liquidity,
        })
    }

    fn extract_identifier(&self, url: &str) -> Result<String> {
        let parsed = Url::parse(url)
            .map_err(|e| AppError::Validation(format!("Invalid URL: {}", e)))?;

        // Extract slug from Polymarket URL: https://polymarket.com/event/...
        if parsed.host_str().unwrap_or("").contains("polymarket") {
            let path = parsed.path();
            if let Some(slug) = path.strip_prefix("/event/") {
                return Ok(slug.to_string());
            }
        }

        // Extract ticker from Kalshi URL: https://kalshi.com/trade/...
        if parsed.host_str().unwrap_or("").contains("kalshi") {
            let path = parsed.path();
            if let Some(ticker) = path.strip_prefix("/trade/") {
                return Ok(ticker.to_string());
            }
        }

        Err(AppError::Validation(format!(
            "Could not extract identifier from URL: {}",
            url
        )))
    }

    fn detect_platform(&self, url: &str) -> Result<Platform> {
        let parsed = Url::parse(url)
            .map_err(|e| AppError::Validation(format!("Invalid URL: {}", e)))?;

        let host = parsed.host_str().unwrap_or("").to_lowercase();

        if host.contains("polymarket") {
            Ok(Platform::Polymarket)
        } else if host.contains("kalshi") {
            Ok(Platform::Kalshi)
        } else {
            Err(AppError::Validation(format!(
                "Unsupported platform in URL: {}",
                url
            )))
        }
    }
}

