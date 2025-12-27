pub mod analyze_event_markets;
pub mod limit_order_bot;
pub mod polyfactual_research;
pub mod position_tracker;

use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;

use crate::clients::{PolyfactualClient, PolymarketClient};
use crate::api::analyze_event_markets::Clients;

#[derive(Clone)]
pub struct AppState {
    pub dome_clients: Arc<Clients>,
    pub polyfactual_client: Arc<PolyfactualClient>,
    pub polymarket_client: Arc<PolymarketClient>,
}

pub fn create_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/analyze-event-markets", post(analyze_event_markets::handler))
        .route("/api/polyfactual-research", post(polyfactual_research::handler))
        .route("/api/position-tracker", post(position_tracker::handler))
        .route("/api/limit-order-bot", post(limit_order_bot::handler))
        .route("/health", get(health_check))
}

async fn health_check() -> &'static str {
    "OK"
}
