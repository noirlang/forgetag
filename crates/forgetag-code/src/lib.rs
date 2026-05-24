//! Code intelligence contracts for forgetag.

use forgetag_core::ItemId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSymbol {
    pub item_id: ItemId,
    pub language: String,
    pub symbol_kind: String,
    pub name: String,
    pub qualified_name: Option<String>,
    pub start_line: u32,
    pub end_line: u32,
}
