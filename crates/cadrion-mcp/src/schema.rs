//! Live MCP + error surface dump (H5-5 / D13).
//!
//! Honesty: not a generated JSON Schema of every response body. The clap CLI
//! tree lives in `cadrion-cli`; OpenAPI lives in `cadrion-api`. This crate
//! dumps what it owns. Long catalogs: `cadrion://doc/schema`.

use serde_json::{json, Value};

use cadrion_kernel::ERROR_CATALOG;

use crate::compliance::{PROTOCOL_VERSION, TOOL_NAMES};
use crate::tools::tool_defs;

pub const SCHEMA_HONESTY: &str = "live-surfaces: cadrion_mcp::tool_defs + ERROR_CATALOG — clap CLI and OpenAPI are other faces (cadrion schema / /v1/schema)";

pub fn mcp_schema() -> Value {
    json!({
        "implementation": "hand-rolled",
        "protocol": PROTOCOL_VERSION,
        "tools": tool_defs(),
        "tool_names": TOOL_NAMES,
    })
}

pub fn errors_schema() -> Value {
    json!({
        "codes": ERROR_CATALOG
            .iter()
            .map(|c| json!({"code": c.code, "meaning": c.meaning}))
            .collect::<Vec<_>>(),
        "count": ERROR_CATALOG.len(),
    })
}

pub fn dump(face: &str) -> Result<Value, String> {
    match face {
        "mcp" | "" => Ok(wrap("mcp", mcp_schema())),
        "errors" => Ok(wrap("errors", errors_schema())),
        other => Err(format!(
            "unknown schema face {other:?} (mcp|errors). cli/api: `cadrion schema` or POST /v1/schema"
        )),
    }
}

fn wrap(key: &str, value: Value) -> Value {
    json!({
        "ok": true,
        "cadrion": env!("CARGO_PKG_VERSION"),
        "source": "live-surfaces",
        "honesty": SCHEMA_HONESTY,
        key: value,
    })
}
