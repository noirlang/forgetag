//! Database boundary for forgetag.
//!
//! SQLite is the first implementation target. Keep public traits generic enough
//! to support PostgreSQL later.

use async_trait::async_trait;
use forgetag_core::{Library, LibraryId, Result};

#[async_trait]
pub trait LibraryRepository: Send + Sync {
    async fn list_libraries(&self) -> Result<Vec<Library>>;
    async fn get_library(&self, id: LibraryId) -> Result<Option<Library>>;
    async fn upsert_library(&self, library: &Library) -> Result<()>;
}

pub trait TransactionManager: Send + Sync {}
