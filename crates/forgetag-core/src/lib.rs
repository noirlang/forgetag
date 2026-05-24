//! Core domain model for forgetag.
//!
//! This crate must stay free of Tauri, SQLx, Tantivy, Tree-sitter, and provider
//! SDK dependencies. It defines stable domain types shared by backend services.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LibraryId(pub Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ItemId(pub Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TagId(pub Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RelationshipId(pub Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct JobId(pub Uuid);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LibraryStatus {
    Mounted,
    Unmounted,
    Offline,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LibraryContentMode {
    Managed,
    Indexed,
    Hybrid,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportMode {
    Copy,
    Move,
    IndexInPlace,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportKind {
    File,
    Folder,
    Archive,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveImportMode {
    KeepOnly,
    Extract,
    IndexInPlace,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictPolicy {
    Rename,
    Skip,
    Replace,
    Deduplicate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemSource {
    Filesystem,
    Email,
    Bookmark,
    Terminal,
    Plugin,
    Virtual,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtractionState {
    Pending,
    Partial,
    Complete,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Library {
    pub id: LibraryId,
    pub name: String,
    pub status: LibraryStatus,
    pub content_mode: LibraryContentMode,
    pub managed_root_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub id: ItemId,
    pub library_id: LibraryId,
    pub kind: String,
    pub title: Option<String>,
    pub canonical_uri: String,
    pub source: ItemSource,
    pub extraction_state: ExtractionState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub id: TagId,
    pub library_id: LibraryId,
    pub namespace: Option<String>,
    pub slug: String,
    pub label: String,
    pub parent_tag_id: Option<TagId>,
}

#[derive(Debug, thiserror::Error)]
pub enum ForgetagError {
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("permission denied: {0}")]
    PermissionDenied(String),
    #[error("library is offline: {0}")]
    LibraryOffline(String),
    #[error("internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, ForgetagError>;
