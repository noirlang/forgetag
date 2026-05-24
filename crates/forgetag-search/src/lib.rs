//! Search query model and backend abstraction for forgetag.

use forgetag_core::{ItemId, LibraryId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRequest {
    pub query: String,
    pub library_ids: Vec<LibraryId>,
    pub limit: u32,
    pub offset: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    pub item_id: ItemId,
    pub library_id: LibraryId,
    pub title: String,
    pub score: f32,
}

#[derive(Debug, thiserror::Error)]
pub enum QueryError {
    #[error("failed to parse query: {0}")]
    Parse(String),
}

pub trait SearchBackend: Send + Sync {
    fn search(&self, request: SearchRequest) -> Result<Vec<SearchHit>, QueryError>;
}
