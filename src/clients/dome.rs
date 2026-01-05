use crate::types::{MarketData, Outcome, Platform};
use crate::{AppError, Result};
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;
use url::Url;

const DOME_API_BASE: &str = "https://api.domeapi.io/v1";

#[derive(Debug, Deserialize)]
struct DomeMarketsResponse {
    markets: Vec<DomeMarket>,
    #[allow(dead_code)]
    pagination: DomePagination,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct DomePagination {
    limit: u32,
    offset: u32,
    total: u32,
    has_more: bool,
}

#[derive(Debug, Deserialize)]
struct DomeMarket {
    market_slug: String,
    title: String,
    condition_id: String,
    side_a: DomeSide,
    side_b: DomeSide,
    volume_total: Option<f64>,
    #[allow(dead_code)]
    volume_1_week: Option<f64>,
    #[allow(dead_code)]
    image: Option<String>,
    #[allow(dead_code)]
    tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct DomeSide {
    id: String,
    label: String,
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
            .map_err(|e| {
                AppError::Internal(anyhow::anyhow!("Failed to create HTTP client: {}", e))
            })?;

        Ok(Self { client, api_key })
    }

    pub async fn get_market_by_url(&self, url: &str) -> Result<MarketData> {
        // Extract identifier from URL
        let identifier = self.extract_identifier(url)?;
        let platform = self.detect_platform(url)?;
        let endpoint = match platform {
            Platform::Polymarket => format!("{}/polymarket/markets?event_slug={}", DOME_API_BASE, identifier),
            Platform::Kalshi => format!("{}/markets/kalshi/{}", DOME_API_BASE, identifier),
        };
        tracing::info!("endpoint -----------> {:?}", endpoint);
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
        let dome_response: DomeMarketsResponse = response
            .json()
            .await
            .map_err(|e| AppError::ExternalApi(format!("Failed to parse Dome response: {}", e)))?;

        // Get the first market from the response
        let market = dome_response
            .markets
            .first()
            .ok_or_else(|| AppError::ExternalApi("No markets found in Dome API response".to_string()))?;

        // Convert sides to outcomes
        // Note: Dome API doesn't provide prices directly, so we set them to 0.0
        // You may need to fetch prices from a separate endpoint or calculate them
        let mut outcomes = Vec::new();
        outcomes.push(Outcome {
            id: market.side_a.id.clone(),
            name: market.side_a.label.clone(),
            price: 0.0, // Price not available in this response
            volume: None,
        });
        outcomes.push(Outcome {
            id: market.side_b.id.clone(),
            name: market.side_b.label.clone(),
            price: 0.0, // Price not available in this response
            volume: None,
        });

        Ok(MarketData {
            id: market.condition_id.clone(),
            question: market.title.clone(),
            slug: Some(market.market_slug.clone()),
            ticker: None,
            platform,
            outcomes,
            volume: market.volume_total,
            liquidity: None, // Liquidity not available in this response
        })
    }

    fn extract_identifier(&self, url: &str) -> Result<String> {
        let parsed =
            Url::parse(url).map_err(|e| AppError::Validation(format!("Invalid URL: {}", e)))?;

        // Extract slug from Polymarket URL: https://polymarket.com/event/...
        if parsed.host_str().unwrap_or("").contains("polymarket") {
            let path = parsed.path();
            if let Some(slug) = path.strip_prefix("/event/") {
                println!("slug ---------> {:?}", slug.to_string());
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
        let parsed =
            Url::parse(url).map_err(|e| AppError::Validation(format!("Invalid URL: {}", e)))?;

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
