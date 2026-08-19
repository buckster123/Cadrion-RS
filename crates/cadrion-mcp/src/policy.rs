//! Global MCP policy (write_source gating, project root).

use std::path::PathBuf;
use std::sync::OnceLock;

#[derive(Debug, Clone)]
pub struct McpPolicy {
    /// When false, `write_source` tool returns a structured error.
    pub write_source: bool,
    /// Project root for resource scans / path policy.
    pub project_root: PathBuf,
    /// Transport label for diagnostics: "stdio" | "http".
    pub transport: &'static str,
}

static POLICY: OnceLock<McpPolicy> = OnceLock::new();

/// Install policy once at process start (stdio or HTTP entry).
pub fn init_policy(policy: McpPolicy) {
    let _ = POLICY.set(policy);
}

pub fn policy() -> McpPolicy {
    POLICY.get().cloned().unwrap_or_else(|| McpPolicy {
        write_source: false,
        project_root: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        transport: "unknown",
    })
}

/// Resolve write_source from env with transport default.
/// - unset → `default_on`
/// - `1`/`true`/`on`/`yes` → true
/// - `0`/`false`/`off`/`no` → false
pub fn write_source_from_env(default_on: bool) -> bool {
    match cadrion_kernel::env_var("CADRION_MCP_WRITE_SOURCE") {
        Ok(v) => {
            let l = v.to_ascii_lowercase();
            matches!(l.as_str(), "1" | "true" | "on" | "yes")
        }
        Err(_) => default_on,
    }
}

pub fn project_root_from_env() -> PathBuf {
    if let Ok(p) = cadrion_kernel::env_var("CADRION_PROJECT") {
        return PathBuf::from(p);
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}
