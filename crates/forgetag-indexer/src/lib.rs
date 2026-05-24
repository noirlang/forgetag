//! Filesystem indexing pipeline for forgetag.

use forgetag_core::{ItemId, JobId, LibraryId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IndexJobType {
    DiscoverPath,
    StatPath,
    ExtractMetadata,
    ExtractContent,
    ExtractCodeSymbols,
    GeneratePreview,
    UpdateSearchIndex,
    ComputeEmbedding,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexJob {
    pub id: JobId,
    pub library_id: LibraryId,
    pub item_id: Option<ItemId>,
    pub job_type: IndexJobType,
    pub priority: i32,
}
