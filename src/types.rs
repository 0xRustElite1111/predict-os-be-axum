use serde::{Deserialize, Serialize};

// AI Response Types
#[derive(Debug, Serialize, Deserialize)]
pub struct AiAnalysis {
    pub recommendation: Recommendation,
    pub confidence: f64,
    pub reasoning: String,
    pub key_factors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Recommendation {
    BuyYes,
    BuyNo,
    NoTrade,
}

// Market Types
#[derive(Debug, Serialize, Deserialize)]
pub struct MarketData {
    pub id: String,
    pub question: String,
    pub slug: Option<String>,
    pub ticker: Option<String>,
    pub platform: Platform,
    pub outcomes: Vec<Outcome>,
    pub volume: Option<f64>,
    pub liquidity: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Polymarket,
    Kalshi,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Outcome {
    pub id: String,
    pub name: String,
    pub price: f64,
    pub volume: Option<f64>,
}

// Request Types
#[derive(Debug, Deserialize)]
pub struct AnalyzeEventMarketsRequest {
    pub url: String,
    pub question: Option<String>,
    pub model: Option<String>, // "grok" or "openai"
}

#[derive(Debug, Deserialize)]
pub struct PolyfactualResearchRequest {
    pub query: String,
}

#[derive(Debug, Deserialize)]
pub struct PositionTrackerRequest {
    pub wallet_address: String,
    pub market_slug: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LimitOrderBotRequest {
    pub wallet_private_key: String,
    pub market_slug: Option<String>,
    pub mode: OrderMode,
    pub bankroll_usd: f64,
    pub price_levels: Option<usize>, // For ladder mode
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OrderMode {
    Simple,
    Ladder,
}

// Response Types
#[derive(Debug, Serialize)]
pub struct AnalyzeEventMarketsResponse {
    pub recommendation: Recommendation,
    pub analysis: AiAnalysis,
    pub market_data: MarketData,
    pub metadata: ResponseMetadata,
}

#[derive(Debug, Serialize)]
pub struct PolyfactualResearchResponse {
    pub answer: String,
    pub citations: Vec<Citation>,
    pub metadata: ResponseMetadata,
}

#[derive(Debug, Serialize)]
pub struct Citation {
    pub source: String,
    pub url: Option<String>,
    pub relevance: f64,
}

#[derive(Debug, Serialize)]
pub struct PositionTrackerResponse {
    pub market: MarketData,
    pub positions: Vec<Position>,
    pub pair_status: PairStatus,
    pub profit_lock: Option<f64>,
    pub break_even: Option<f64>,
    pub metadata: ResponseMetadata,
}

#[derive(Debug, Serialize)]
pub struct Position {
    pub token_id: String,
    pub outcome: String,
    pub shares: f64,
    pub avg_price: f64,
    pub current_price: f64,
    pub unrealized_pnl: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PairStatus {
    ProfitLocked,
    BreakEven,
    AtRisk,
    NoPosition,
}

#[derive(Debug, Serialize)]
pub struct LimitOrderBotResponse {
    pub orders: Vec<OrderResult>,
    pub market: MarketData,
    pub logs: Vec<String>,
    pub metadata: ResponseMetadata,
}

#[derive(Debug, Serialize)]
pub struct OrderResult {
    pub token_id: String,
    pub outcome: String,
    pub side: String, // "buy" or "sell"
    pub price: f64,
    pub size: f64,
    pub order_id: Option<String>,
    pub status: OrderStatus,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OrderStatus {
    Pending,
    Filled,
    Cancelled,
    Failed,
}

#[derive(Debug, Serialize)]
pub struct ResponseMetadata {
    pub timestamp: String,
    pub execution_time_ms: u64,
    pub model_used: Option<String>,
    pub retries: u32,
}

