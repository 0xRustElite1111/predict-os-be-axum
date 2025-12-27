use axum::{extract::State, Json};
use chrono::Utc;
use std::sync::Arc;
use std::time::Instant;

use crate::api::AppState;
use crate::types::{
    LimitOrderBotRequest, LimitOrderBotResponse, OrderMode,
    ResponseMetadata,
};
use crate::Result;

pub async fn handler(
    State(state): State<Arc<AppState>>,
    Json(request): Json<LimitOrderBotRequest>,
) -> Result<Json<LimitOrderBotResponse>> {
    let start = Instant::now();
    let mut logs = Vec::new();

    // Validate request
    if request.wallet_private_key.is_empty() {
        return Err(crate::AppError::Validation(
            "Wallet private key is required".to_string(),
        ));
    }

    if request.bankroll_usd <= 0.0 {
        return Err(crate::AppError::Validation(
            "Bankroll must be greater than 0".to_string(),
        ));
    }

    // Calculate next 15-min market timestamp
    let market_timestamp = state.polymarket_client.calculate_next_15min_market_timestamp();
    let market_slug = request.market_slug.unwrap_or_else(|| {
        format!("15min-up-down-{}", market_timestamp.format("%Y%m%d-%H%M"))
    });

    logs.push(format!("Target market: {}", market_slug));

    // Fetch market data
    let market = state.polymarket_client.get_market_by_slug(&market_slug).await?;
    logs.push(format!("Fetched market: {}", market.question));

    // Extract token IDs (Up/Down)
    let token_ids: Vec<String> = market.outcomes.iter().map(|o| o.id.clone()).collect();

    if token_ids.len() < 2 {
        return Err(crate::AppError::Validation(
            "Market must have at least 2 outcomes".to_string(),
        ));
    }

    let up_token_id = &token_ids[0];
    let down_token_id = &token_ids[1];

    logs.push(format!("Up token: {}, Down token: {}", up_token_id, down_token_id));

    // Place orders based on mode
    let mut orders = Vec::new();

    match request.mode {
        OrderMode::Simple => {
            // Straddle: buy both Up and Down at current prices
            logs.push("Mode: Simple (straddle)".to_string());

            let up_price = market.outcomes[0].price;
            let down_price = market.outcomes[1].price;
            let allocation_per_side = request.bankroll_usd / 2.0;

            let up_shares = (allocation_per_side / up_price).max(5.0);
            let down_shares = (allocation_per_side / down_price).max(5.0);

            logs.push(format!(
                "Placing Up order: {} shares @ ${:.4}",
                up_shares, up_price
            ));
            logs.push(format!(
                "Placing Down order: {} shares @ ${:.4}",
                down_shares, down_price
            ));

            let up_order = state
                .polymarket_client
                .place_order(
                    &request.wallet_private_key,
                    up_token_id,
                    "buy",
                    up_price,
                    up_shares,
                )
                .await?;

            let down_order = state
                .polymarket_client
                .place_order(
                    &request.wallet_private_key,
                    down_token_id,
                    "buy",
                    down_price,
                    down_shares,
                )
                .await?;

            orders.push(up_order);
            orders.push(down_order);
        }
        OrderMode::Ladder => {
            // Ladder: multiple price levels with exponential taper
            logs.push("Mode: Ladder (exponential taper)".to_string());

            let price_levels = request.price_levels.unwrap_or(5);
            let min_price = 0.01;
            let max_price = 0.99;

            let up_ladder = state.polymarket_client.calculate_ladder_orders(
                request.bankroll_usd / 2.0,
                price_levels,
                min_price,
                max_price,
            );

            let down_ladder = state.polymarket_client.calculate_ladder_orders(
                request.bankroll_usd / 2.0,
                price_levels,
                min_price,
                max_price,
            );

            logs.push(format!("Calculated {} price levels per side", price_levels));

            for (price, shares) in up_ladder {
                logs.push(format!("Up ladder: {} shares @ ${:.4}", shares, price));
                let order = state
                    .polymarket_client
                    .place_order(
                        &request.wallet_private_key,
                        up_token_id,
                        "buy",
                        price,
                        shares,
                    )
                    .await?;
                orders.push(order);
            }

            for (price, shares) in down_ladder {
                logs.push(format!("Down ladder: {} shares @ ${:.4}", shares, price));
                let order = state
                    .polymarket_client
                    .place_order(
                        &request.wallet_private_key,
                        down_token_id,
                        "buy",
                        price,
                        shares,
                    )
                    .await?;
                orders.push(order);
            }
        }
    }

    let execution_time = start.elapsed().as_millis() as u64;

    logs.push(format!("Completed in {}ms", execution_time));

    Ok(Json(LimitOrderBotResponse {
        orders,
        market,
        logs,
        metadata: ResponseMetadata {
            timestamp: Utc::now().to_rfc3339(),
            execution_time_ms: execution_time,
            model_used: None,
            retries: 0,
        },
    }))
}

