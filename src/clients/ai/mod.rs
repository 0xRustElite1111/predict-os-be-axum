pub mod grok;
pub mod openai;
pub mod prompts;

pub use grok::GrokClient;
pub use openai::OpenAiClient;

use crate::types::AiAnalysis;
use crate::Result;
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub enum AiProvider {
    Grok,
    OpenAi,
}

#[async_trait]
pub trait AiClient: Send + Sync {
    async fn analyze_markets(&self, prompt: String) -> Result<AiAnalysis>;
    fn provider_name(&self) -> &'static str;
}

pub fn create_ai_client(provider: AiProvider) -> Result<Box<dyn AiClient>> {
    match provider {
        AiProvider::Grok => Ok(Box::new(GrokClient::new()?)),
        AiProvider::OpenAi => Ok(Box::new(OpenAiClient::new()?)),
    }
}

