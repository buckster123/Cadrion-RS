//! Cadrion MCP server — stdio + streamable HTTP JSON-RPC.

#![deny(unsafe_code)]

mod compliance;
mod engine;
mod http;
mod policy;
mod prompts;
mod protocol;
mod resources;
mod schema;
mod server;
mod tools;

pub use compliance::{PROTOCOL_VERSION, SUPPORTED_METHODS, TOOL_NAMES, UNSUPPORTED_BUT_DOCUMENTED};
pub use engine::{info_json as engine_info, install as engine_install};
pub use http::{serve_http, HttpMcpConfig};
pub use policy::{init_policy, policy, project_root_from_env, write_source_from_env, McpPolicy};
pub use prompts::{get_prompt, list_prompts, PROMPTS};
pub use schema::{dump as schema_dump, errors_schema, mcp_schema, SCHEMA_HONESTY};
pub use server::{dispatch, handle_http_body, run_stdio};
pub use tools::{call_tool, tool_defs, ToolError};

/// Crate version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Shared entry for HTTP API (returns full MCP tool result JSON).
pub fn tools_call_for_api(
    name: &str,
    args: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    call_tool(name, args).map_err(|e| e.to_string())
}
