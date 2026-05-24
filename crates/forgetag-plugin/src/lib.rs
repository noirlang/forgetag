//! Plugin manifest and capability model for forgetag.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub version: String,
    pub runtime: PluginRuntime,
    pub capabilities: Vec<PluginCapability>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginRuntime {
    Builtin,
    Wasm,
    Native,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginCapability {
    pub name: String,
    pub scope: serde_json::Value,
}
