//! Tauri-safe command and event DTOs for forgetag.

use forgetag_core::LibraryStatus;
use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct CreateLibraryRequest {
    pub name: String,
    pub roots: Vec<CreateLibraryRootRequest>,
    pub content_mode: String,
    pub managed_root_path: Option<String>,
    pub storage_mode: String,
    pub start_indexing: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct CreateLibraryRootRequest {
    pub path: String,
    pub symlink_policy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct LibraryDto {
    pub id: String,
    pub name: String,
    pub status: String,
    pub content_mode: String,
    pub managed_root_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ImportMetadataInput {
    pub title: Option<String>,
    pub description: Option<String>,
    pub tag_ids: Vec<String>,
    pub date: Option<String>,
    pub project: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct AddFileImportRequest {
    pub library_id: String,
    pub source_path: String,
    pub import_mode: String,
    pub conflict_policy: String,
    pub metadata: ImportMetadataInput,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct AddFolderImportRequest {
    pub library_id: String,
    pub source_path: String,
    pub import_mode: String,
    pub conflict_policy: String,
    pub apply_metadata_to_children: bool,
    pub metadata: ImportMetadataInput,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct AddArchiveImportRequest {
    pub library_id: String,
    pub source_path: String,
    pub import_mode: String,
    pub archive_mode: String,
    pub preserve_original: bool,
    pub conflict_policy: String,
    pub metadata: ImportMetadataInput,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ImportBatchDto {
    pub id: String,
    pub library_id: String,
    pub import_kind: String,
    pub status: String,
    pub queued_jobs: u32,
}

pub fn library_status_to_wire(value: LibraryStatus) -> &'static str {
    match value {
        LibraryStatus::Mounted => "mounted",
        LibraryStatus::Unmounted => "unmounted",
        LibraryStatus::Offline => "offline",
        LibraryStatus::Error => "error",
    }
}
