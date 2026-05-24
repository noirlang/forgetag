//! Optional local AI provider contracts for forgetag.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingRequest {
    pub model: String,
    pub input: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingResponse {
    pub model: String,
    pub vector: Vec<f32>,
}

#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    async fn embed(&self, request: EmbeddingRequest) -> anyhow::Result<EmbeddingResponse>;
}
