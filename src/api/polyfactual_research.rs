use axum::{extract::State, Json};
use std::sync::Arc;

use crate::api::AppState;
use crate::types::PolyfactualResearchRequest;
use crate::Result;

pub async fn handler(
    State(state): State<Arc<AppState>>,
    Json(request): Json<PolyfactualResearchRequest>,
) -> Result<Json<crate::types::PolyfactualResearchResponse>> {
    // Validate request
    if request.query.is_empty() {
        return Err(crate::AppError::Validation("Query is required".to_string()));
    }

    // Call Polyfactual API
    let response = state.polyfactual_client.research(request.query).await?;

    Ok(Json(response))
}

