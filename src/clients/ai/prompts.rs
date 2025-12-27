use crate::types::MarketData;

pub fn build_analysis_prompt(market_data: &MarketData, question: Option<&String>) -> String {
    let base_question = question
        .map(|q| q.as_str())
        .unwrap_or("Should I buy YES or NO on this prediction market?");

    format!(
        r#"You are an expert prediction market analyst. Analyze the following market data and provide a recommendation.

Market Question: {}
Platform: {:?}
Volume: {:?}
Liquidity: {:?}

Outcomes:
{}

User Question: {}

Provide your analysis in the following JSON format:
{{
  "recommendation": "BUY_YES" | "BUY_NO" | "NO_TRADE",
  "confidence": 0.0-1.0,
  "reasoning": "Detailed explanation of your analysis",
  "key_factors": ["factor1", "factor2", ...]
}}

Be concise but thorough. Focus on market dynamics, liquidity, and value opportunities."#,
        market_data.question,
        market_data.platform,
        market_data.volume,
        market_data.liquidity,
        market_data
            .outcomes
            .iter()
            .map(|o| format!("  - {}: ${:.4} (volume: {:?})", o.name, o.price, o.volume))
            .collect::<Vec<_>>()
            .join("\n"),
        base_question
    )
}

