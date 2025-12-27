# PredictOS Backend

A Rust-based backend for PredictOS, a prediction market trading platform with AI-powered analysis.

## Architecture

This backend uses **Axum** as the web framework and provides a serverless-like architecture with:

- **API Routes**: RESTful endpoints that proxy and validate requests
- **Shared Clients**: Reusable clients for external services
- **Error Handling**: Comprehensive error handling with retry logic
- **Type Safety**: Full TypeScript-like type safety with Rust

## Features

### API Endpoints

1. **`POST /api/analyze-event-markets`** - Analyze prediction markets with AI
   - Supports Polymarket and Kalshi
   - AI providers: Grok (default) or OpenAI
   - Returns trading recommendations (BUY_YES, BUY_NO, NO_TRADE)

2. **`POST /api/polyfactual-research`** - Deep research with citations
   - Query validation (max 1000 chars)
   - Returns answers with source citations

3. **`POST /api/position-tracker`** - Track positions in Polymarket 15-min markets
   - Auto-detects current market
   - Calculates profit lock, break-even, and pair status

4. **`POST /api/limit-order-bot`** - Automated limit order bot
   - Simple mode: Straddle orders (buy both Up/Down)
   - Ladder mode: Multiple price levels with exponential taper

5. **`GET /health`** - Health check endpoint

### Shared Clients

- **AI Clients** (`src/clients/ai/`): Grok and OpenAI integration with retry logic
- **Dome Client** (`src/clients/dome.rs`): Unified API for Polymarket and Kalshi
- **Polymarket Client** (`src/clients/polymarket.rs`): Market data, positions, and order placement
- **Polyfactual Client** (`src/clients/polyfactual.rs`): Research API integration

## Setup

1. **Install Rust** (if not already installed):
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **Clone and navigate to the project**:
   ```bash
   cd predict-os-be
   ```

3. **Copy environment variables**:
   ```bash
   cp .env.example .env
   ```

4. **Edit `.env`** with your API keys:
   - `GROK_API_KEY` - Grok API key (from x.ai)
   - `OPENAI_API_KEY` - OpenAI API key (optional, for fallback)
   - `DOME_API_KEY` - Dome API key for unified market data
   - `POLYMARKET_GAMMA_API_KEY` - Polymarket Gamma API key (optional)
   - `POLYFACTUAL_API_KEY` - Polyfactual API key

5. **Build and run**:
   ```bash
   cargo build --release
   cargo run
   ```

   Or for development:
   ```bash
   cargo run
   ```

The server will start on `http://0.0.0.0:3000`

## API Usage Examples

### Analyze Event Markets

```bash
curl -X POST http://localhost:3000/api/analyze-event-markets \
  -H "Content-Type: application/json" \
  -d '{
    "url": "https://polymarket.com/event/will-bitcoin-reach-100k",
    "question": "Should I buy YES or NO?",
    "model": "grok"
  }'
```

### Polyfactual Research

```bash
curl -X POST http://localhost:3000/api/polyfactual-research \
  -H "Content-Type: application/json" \
  -d '{
    "query": "What are the latest developments in prediction markets?"
  }'
```

### Position Tracker

```bash
curl -X POST http://localhost:3000/api/position-tracker \
  -H "Content-Type: application/json" \
  -d '{
    "wallet_address": "0x...",
    "market_slug": "15min-up-down-20240101-1200"
  }'
```

### Limit Order Bot

```bash
curl -X POST http://localhost:3000/api/limit-order-bot \
  -H "Content-Type: application/json" \
  -d '{
    "wallet_private_key": "0x...",
    "bankroll_usd": 100.0,
    "mode": "simple",
    "price_levels": 5
  }'
```

## Project Structure

```
src/
├── main.rs                 # Server entry point
├── lib.rs                  # Library root
├── error.rs                # Error types and handling
├── types.rs                # Shared type definitions
├── api/                    # API route handlers
│   ├── mod.rs
│   ├── analyze_event_markets.rs
│   ├── polyfactual_research.rs
│   ├── position_tracker.rs
│   └── limit_order_bot.rs
└── clients/                # External service clients
    ├── mod.rs
    ├── ai/
    │   ├── mod.rs
    │   ├── grok.rs
    │   ├── openai.rs
    │   └── prompts.rs
    ├── dome.rs
    ├── polyfactual.rs
    └── polymarket.rs
```

## Technical Details

### Error Handling
- Retry logic with exponential backoff (max 3 attempts)
- Structured error responses with metadata
- Comprehensive logging at all levels

### Security
- API keys stored in environment variables
- Wallet private keys never exposed in responses
- CORS headers configured

### Performance
- Parallel operations where possible
- Request timeouts (2 min for AI, 5 min for research)
- Efficient HTTP client reuse

### Type Safety
- TypeScript-like type definitions
- Strict JSON schema validation
- Compile-time error checking

## Development

### Running Tests
```bash
cargo test
```

### Code Formatting
```bash
cargo fmt
```

### Linting
```bash
cargo clippy
```

## Dependencies

- **axum**: Web framework
- **tokio**: Async runtime
- **reqwest**: HTTP client
- **serde**: Serialization
- **chrono**: Date/time handling
- **tower-http**: Middleware (CORS, tracing)
- **tracing**: Logging

## Notes

- The Polymarket CLOB integration is currently a placeholder and requires full implementation with ethers.js equivalent
- Some API endpoints may require additional authentication in production
- Consider adding rate limiting and caching for production use

## License

[Your License Here]

