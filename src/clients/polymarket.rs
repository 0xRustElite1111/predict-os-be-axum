use crate::types::{MarketData, OrderResult, OrderStatus, Outcome, Platform};
use crate::{AppError, Result};
use chrono::{DateTime, Timelike, Utc};
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;

const GAMMA_API_BASE: &str = "https://gamma-api.polymarket.com";
const DATA_API_BASE: &str = "https://data-api.polymarket.com";

#[derive(Debug, Deserialize)]
struct GammaMarketResponse {
    id: String,
    question: String,
    slug: String,
    outcomes: Vec<GammaOutcome>,
    volume: Option<f64>,
    liquidity: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct GammaOutcome {
    id: String,
    name: String,
    price: f64,
    volume: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct PositionResponse {
    positions: Vec<PositionData>,
}

#[derive(Debug, Deserialize)]
pub struct PositionData {
    pub token_id: String,
    pub outcome: String,
    pub shares: f64,
    pub avg_price: f64,
    pub current_price: f64,
}

pub struct PolymarketClient {
    client: Client,
    gamma_api_key: Option<String>,
}

impl PolymarketClient {
    pub fn new() -> Self {
        let gamma_api_key = std::env::var("POLYMARKET_GAMMA_API_KEY").ok();

        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            client,
            gamma_api_key,
        }
    }

    pub async fn get_market_by_slug(&self, slug: &str) -> Result<MarketData> {
        let url = format!("{}/markets/{}", GAMMA_API_BASE, slug);

        let mut request = self.client.get(&url);

        if let Some(ref key) = self.gamma_api_key {
            request = request.header("Authorization", format!("Bearer {}", key));
        }

        let response = request
            .send()
            .await
            .map_err(|e| AppError::ExternalApi(format!("Gamma API request failed: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(AppError::ExternalApi(format!(
                "Gamma API returned {}: {}",
                status, error_text
            )));
        }

        let gamma_response: GammaMarketResponse = response
            .json()
            .await
            .map_err(|e| AppError::ExternalApi(format!("Failed to parse Gamma response: {}", e)))?;

        Ok(MarketData {
            id: gamma_response.id,
            question: gamma_response.question,
            slug: Some(gamma_response.slug),
            ticker: None,
            platform: Platform::Polymarket,
            outcomes: gamma_response
                .outcomes
                .into_iter()
                .map(|o| Outcome {
                    id: o.id,
                    name: o.name,
                    price: o.price,
                    volume: o.volume,
                })
                .collect(),
            volume: gamma_response.volume,
            liquidity: gamma_response.liquidity,
        })
    }

    pub async fn get_market_position(
        &self,
        wallet_address: &str,
        token_ids: &[String],
    ) -> Result<Vec<PositionData>> {
        let url = format!("{}/positions", DATA_API_BASE);

        let response = self
            .client
            .get(&url)
            .query(&[("user", wallet_address)])
            .send()
            .await
            .map_err(|e| AppError::ExternalApi(format!("Data API request failed: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(AppError::ExternalApi(format!(
                "Data API returned {}: {}",
                status, error_text
            )));
        }

        let position_response: PositionResponse = response
            .json()
            .await
            .map_err(|e| AppError::ExternalApi(format!("Failed to parse position response: {}", e)))?;

        // Filter positions by token IDs
        let filtered: Vec<PositionData> = position_response
            .positions
            .into_iter()
            .filter(|p| token_ids.contains(&p.token_id))
            .collect();

        Ok(filtered)
    }

    pub fn calculate_15min_market_timestamp(&self) -> DateTime<Utc> {
        let now = Utc::now();
        let minutes = now.minute();
        let rounded_minutes = (minutes / 15) * 15;
        now.with_minute(rounded_minutes)
            .and_then(|dt| dt.with_second(0))
            .and_then(|dt| dt.with_nanosecond(0))
            .unwrap_or(now)
    }

    pub fn calculate_next_15min_market_timestamp(&self) -> DateTime<Utc> {
        let current = self.calculate_15min_market_timestamp();
        current + chrono::Duration::minutes(15)
    }

    // Placeholder for CLOB order placement
    // In a real implementation, this would use @polymarket/clob-client
    pub async fn place_order(
        &self,
        _private_key: &str,
        token_id: &str,
        side: &str,
        price: f64,
        size: f64,
    ) -> Result<OrderResult> {
        // This is a placeholder - real implementation would use ethers and CLOB client
        tracing::warn!("CLOB order placement not fully implemented - requires ethers integration");

        Ok(OrderResult {
            token_id: token_id.to_string(),
            outcome: "Unknown".to_string(),
            side: side.to_string(),
            price,
            size,
            order_id: None,
            status: OrderStatus::Pending,
        })
    }

    pub fn calculate_ladder_orders(
        &self,
        bankroll_usd: f64,
        price_levels: usize,
        min_price: f64,
        max_price: f64,
    ) -> Vec<(f64, f64)> {
        // Exponential taper: more allocation at lower prices
        let mut orders = Vec::new();
        let total_allocation = bankroll_usd;
        let min_shares = 5.0; // Polymarket minimum

        for i in 0..price_levels {
            let price = min_price + (max_price - min_price) * (i as f64 / (price_levels - 1) as f64);
            // Exponential taper: 2^(levels-i) / sum(2^j for j in 0..levels)
            let weight = 2_f64.powi((price_levels - i) as i32);
            let allocation = total_allocation * weight / (2_f64.powi(price_levels as i32) - 1.0);
            let shares = (allocation / price).max(min_shares);

            orders.push((price, shares));
        }

        orders
    }
}

