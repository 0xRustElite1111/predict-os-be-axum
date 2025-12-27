use predict_os_be::api;
use predict_os_be::clients::{PolyfactualClient, PolymarketClient};
use predict_os_be::api::analyze_event_markets::Clients;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tracing_subscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "predict_os_be=debug,tower_http=info".into()),
        )
        .init();

    // Load environment variables
    dotenvy::dotenv().ok();

    // Initialize clients
    let dome_clients = Arc::new(Clients::new().map_err(|e| anyhow::anyhow!("{}", e))?);
    let polyfactual_client = Arc::new(
        PolyfactualClient::new().map_err(|e| anyhow::anyhow!("{}", e))?
    );
    let polymarket_client = Arc::new(PolymarketClient::new());

    // Create app state
    let app_state = Arc::new(api::AppState {
        dome_clients,
        polyfactual_client,
        polymarket_client,
    });

    // Create router with state
    let app = api::create_router()
        .layer(CorsLayer::permissive())
        .with_state(app_state.clone());

    // Start server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    tracing::info!("Server listening on http://0.0.0.0:3000");

    axum::serve(listener, app).await?;

    Ok(())
}
