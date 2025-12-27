use axum::{extract::State, Json};
use chrono::Utc;
use std::sync::Arc;
use std::time::Instant;

use crate::api::AppState;
use crate::types::{
    PairStatus, Position, PositionTrackerRequest, PositionTrackerResponse, ResponseMetadata,
};
use crate::Result;

pub async fn handler(
    State(state): State<Arc<AppState>>,
    Json(request): Json<PositionTrackerRequest>,
) -> Result<Json<PositionTrackerResponse>> {
    let start = Instant::now();

    // Validate request
    if request.wallet_address.is_empty() {
        return Err(crate::AppError::Validation(
            "Wallet address is required".to_string(),
        ));
    }

    // Determine current 15-min market
    let market_timestamp = state.polymarket_client.calculate_15min_market_timestamp();
    let market_slug = request.market_slug.unwrap_or_else(|| {
        format!("15min-up-down-{}", market_timestamp.format("%Y%m%d-%H%M"))
    });

    // Fetch market data
    let market = state.polymarket_client.get_market_by_slug(&market_slug).await?;

    // Extract token IDs (Up/Down)
    let token_ids: Vec<String> = market.outcomes.iter().map(|o| o.id.clone()).collect();

    if token_ids.len() < 2 {
        return Err(crate::AppError::Validation(
            "Market must have at least 2 outcomes".to_string(),
        ));
    }

    // Fetch positions
    let position_data = state
        .polymarket_client
        .get_market_position(&request.wallet_address, &token_ids)
        .await?;

    // Calculate positions and pair status
    let positions: Vec<Position> = position_data
        .iter()
        .map(|p| {
            let outcome = market
                .outcomes
                .iter()
                .find(|o| o.id == p.token_id)
                .map(|o| o.name.clone())
                .unwrap_or_else(|| "Unknown".to_string());

            Position {
                token_id: p.token_id.clone(),
                outcome,
                shares: p.shares,
                avg_price: p.avg_price,
                current_price: p.current_price,
                unrealized_pnl: (p.current_price - p.avg_price) * p.shares,
            }
        })
        .collect();

    // Calculate pair status
    let (pair_status, profit_lock, break_even) = calculate_pair_status(&positions);

    let execution_time = start.elapsed().as_millis() as u64;

    Ok(Json(PositionTrackerResponse {
        market,
        positions,
        pair_status,
        profit_lock,
        break_even,
        metadata: ResponseMetadata {
            timestamp: Utc::now().to_rfc3339(),
            execution_time_ms: execution_time,
            model_used: None,
            retries: 0,
        },
    }))
}

fn calculate_pair_status(positions: &[Position]) -> (PairStatus, Option<f64>, Option<f64>) {
    if positions.len() < 2 {
        return (PairStatus::NoPosition, None, None);
    }

    let up_position = positions.iter().find(|p| p.outcome.contains("Up"));
    let down_position = positions.iter().find(|p| p.outcome.contains("Down"));

    match (up_position, down_position) {
        (Some(up), Some(down)) => {
            let up_pnl = up.unrealized_pnl;
            let down_pnl = down.unrealized_pnl;
            let total_pnl = up_pnl + down_pnl;

            if total_pnl > 0.0 {
                // Profit locked
                (PairStatus::ProfitLocked, Some(total_pnl), None)
            } else if total_pnl == 0.0 {
                // Break even
                (PairStatus::BreakEven, None, Some(0.0))
            } else {
                // At risk
                let break_even_price = (up.avg_price + down.avg_price) / 2.0;
                (PairStatus::AtRisk, None, Some(break_even_price))
            }
        }
        _ => (PairStatus::NoPosition, None, None),
    }
}

