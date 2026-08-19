//! MCP method surface — single source for compliance docs + tests (H2-2 / OQ-7).

/// Protocol version we advertise in `initialize`.
pub const PROTOCOL_VERSION: &str = "2024-11-05";

/// Methods Cadrion implements (hand-rolled). Keep in sync with `server::dispatch`.
pub const SUPPORTED_METHODS: &[&str] = &[
    "initialize",
    "notifications/initialized",
    "initialized", // alias some clients send
    "ping",
    "tools/list",
    "tools/call",
    "resources/list",
    "resources/read",
    "prompts/list",
];

/// Methods we deliberately do **not** implement (return method-not-found).
/// Documented so agents don't assume SDK parity.
pub const UNSUPPORTED_BUT_DOCUMENTED: &[&str] = &[
    "resources/subscribe",
    "resources/unsubscribe",
    "resources/templates/list",
    "completion/complete",
    "logging/setLevel",
    "sampling/createMessage",
    "roots/list",
];

/// Tool names exposed via tools/list (order stable for tests).
pub const TOOL_NAMES: &[&str] = &[
    "build",
    "write_source",
    "read_source",
    "inspect_refs",
    "measure",
    "snapshot",
    "inspect_dims",
    "assembly_validate",
    "sdf_sample",
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::JsonRpcRequest;
    use crate::server::dispatch;
    use crate::tools::tool_defs;
    use serde_json::json;
    use std::sync::Once;

    static INIT: Once = Once::new();
    fn policy() {
        INIT.call_once(|| {
            crate::policy::init_policy(crate::policy::McpPolicy {
                write_source: true,
                project_root: std::env::temp_dir(),
                transport: "test",
            });
        });
    }

    #[test]
    fn tool_defs_match_tool_names() {
        let defs = tool_defs();
        let arr = defs.as_array().expect("tools array");
        assert_eq!(arr.len(), TOOL_NAMES.len());
        for (i, name) in TOOL_NAMES.iter().enumerate() {
            assert_eq!(arr[i]["name"], *name, "tool order drift at {i}");
        }
    }

    #[test]
    fn initialize_advertises_protocol_and_caps() {
        policy();
        let resp = dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "initialize".into(),
            params: Some(json!({})),
        })
        .unwrap();
        let r = resp.result.unwrap();
        assert_eq!(r["protocolVersion"], PROTOCOL_VERSION);
        assert!(r["capabilities"]["tools"].is_object());
        assert!(r["capabilities"]["resources"].is_object());
        assert_eq!(r["serverInfo"]["name"], "cadrion");
        assert!(r["serverInfo"]["transports"]
            .as_array()
            .unwrap()
            .iter()
            .any(|t| t == "stdio"));
    }

    #[test]
    fn unsupported_methods_are_method_not_found() {
        policy();
        for m in UNSUPPORTED_BUT_DOCUMENTED {
            let resp = dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(42)),
                method: (*m).into(),
                params: None,
            })
            .unwrap();
            let err = resp.error.expect("expected error");
            assert_eq!(err.code, -32601, "{m}");
            assert!(
                err.message.contains("method not found"),
                "{m}: {}",
                err.message
            );
        }
    }

    #[test]
    fn supported_request_methods_do_not_404() {
        policy();
        // Skip notifications (no id / no response required) and tools/call (needs args).
        let need_ok = [
            "initialize",
            "ping",
            "tools/list",
            "resources/list",
            "prompts/list",
        ];
        for m in need_ok {
            let resp = dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(7)),
                method: m.into(),
                params: Some(json!({})),
            })
            .unwrap();
            assert!(resp.error.is_none(), "{m} should succeed: {:?}", resp.error);
        }
    }

    #[test]
    fn resources_read_policy_doc() {
        policy();
        let resp = dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(9)),
            method: "resources/read".into(),
            params: Some(json!({"uri": "cadrion://doc/write-source-policy"})),
        })
        .unwrap();
        let text = resp.result.unwrap()["contents"][0]["text"]
            .as_str()
            .unwrap()
            .to_string();
        assert!(text.contains("write_source"));
    }
}
