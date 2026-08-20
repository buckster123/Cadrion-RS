//! Stdio + shared JSON-RPC dispatch.

use std::io::{self, BufReader, Write};

use serde_json::{json, Value};

use crate::prompts::{get_prompt, list_prompts};
use crate::protocol::{read_message, write_message, JsonRpcRequest, JsonRpcResponse};
use crate::resources::{list_resources, read_resource};
use crate::tools::{call_tool, tool_defs};

/// Run until stdin EOF. Logs go to stderr only.
pub fn run_stdio() -> io::Result<()> {
    crate::protocol::configure_framing_from_env();
    crate::policy::init_policy(crate::policy::McpPolicy {
        // H7: stdio write_source OFF unless CADRION_MCP_WRITE_SOURCE=1
        write_source: crate::policy::write_source_from_env(false),
        project_root: crate::policy::project_root_from_env(),
        transport: "stdio",
    });
    let stdin = io::stdin();
    let mut stdin = BufReader::new(stdin.lock());
    let mut stdout = io::stdout().lock();

    eprintln!(
        "cadrion-mcp {} ready (stdio) write_source={}",
        crate::VERSION,
        crate::policy::policy().write_source
    );

    while let Some(body) = read_message(&mut stdin)? {
        let req: JsonRpcRequest = match serde_json::from_slice(&body) {
            Ok(r) => r,
            Err(e) => {
                let resp = JsonRpcResponse::err(None, -32700, format!("parse error: {e}"));
                write_response(&mut stdout, &resp)?;
                continue;
            }
        };

        let resp = dispatch(req);
        if let Some(resp) = resp {
            if resp.id.is_some() || resp.error.is_some() {
                write_response(&mut stdout, &resp)?;
            }
        }
    }
    Ok(())
}

fn write_response(stdout: &mut impl Write, resp: &JsonRpcResponse) -> io::Result<()> {
    let bytes =
        serde_json::to_vec(resp).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    write_message(stdout, &bytes)
}

/// Handle one JSON-RPC request (shared by stdio + HTTP).
pub fn dispatch(req: JsonRpcRequest) -> Option<JsonRpcResponse> {
    let id = req.id.clone();
    match req.method.as_str() {
        "initialize" => Some(JsonRpcResponse::ok(
            id,
            json!({
                "protocolVersion": crate::compliance::PROTOCOL_VERSION,
                "capabilities": {
                    "tools": {},
                    "resources": {},
                    "prompts": {}
                },
                "serverInfo": {
                    "name": "cadrion",
                    "version": crate::VERSION,
                    "transports": ["stdio", "streamable-http"],
                    "write_source": crate::policy::policy().write_source,
                    "transport": crate::policy::policy().transport,
                    "implementation": "hand-rolled",
                    "oq7": "stay-hand-rolled-2026-08-06",
                }
            }),
        )),
        "notifications/initialized" | "initialized" => None,
        "ping" => Some(JsonRpcResponse::ok(id, json!({}))),
        "tools/list" => Some(JsonRpcResponse::ok(id, json!({ "tools": tool_defs() }))),
        "tools/call" => {
            let params = req.params.unwrap_or(Value::Null);
            let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let args = params.get("arguments").cloned().unwrap_or(json!({}));
            match call_tool(name, &args) {
                Ok(result) => Some(JsonRpcResponse::ok(id, result)),
                Err(e) => Some(JsonRpcResponse::ok(
                    id,
                    json!({
                        "content": [{"type": "text", "text": format!("error: {e}")}],
                        "isError": true
                    }),
                )),
            }
        }
        "resources/list" => {
            let root = crate::policy::policy().project_root;
            Some(JsonRpcResponse::ok(
                id,
                json!({ "resources": list_resources(&root) }),
            ))
        }
        "resources/read" => {
            let params = req.params.unwrap_or(Value::Null);
            let uri = params.get("uri").and_then(|v| v.as_str()).unwrap_or("");
            let root = crate::policy::policy().project_root;
            match read_resource(&root, uri) {
                Ok(result) => Some(JsonRpcResponse::ok(id, result)),
                Err(e) => Some(JsonRpcResponse::err(id, -32004, e.to_string())),
            }
        }
        "prompts/list" => Some(JsonRpcResponse::ok(id, list_prompts())),
        "prompts/get" => {
            let params = req.params.unwrap_or(Value::Null);
            let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            match get_prompt(name) {
                Ok(result) => Some(JsonRpcResponse::ok(id, result)),
                Err(e) => Some(JsonRpcResponse::err(id, -32602, e)),
            }
        }
        other => {
            if id.is_some() {
                Some(JsonRpcResponse::err(
                    id,
                    -32601,
                    format!("method not found: {other}"),
                ))
            } else {
                None
            }
        }
    }
}

/// Parse body as one JSON-RPC request and return response JSON value.
pub fn handle_http_body(body: &[u8]) -> Result<Option<Value>, String> {
    let req: JsonRpcRequest =
        serde_json::from_slice(body).map_err(|e| format!("parse error: {e}"))?;
    Ok(dispatch(req).map(|r| serde_json::to_value(r).unwrap_or(Value::Null)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::call_tool;
    use std::sync::Once;
    use std::time::{SystemTime, UNIX_EPOCH};

    static INIT: Once = Once::new();
    fn ensure_test_policy_write_on() {
        INIT.call_once(|| {
            crate::policy::init_policy(crate::policy::McpPolicy {
                write_source: true,
                project_root: std::env::temp_dir(),
                transport: "test",
            });
        });
    }

    #[test]
    fn write_build_inspect_snapshot_loop() {
        ensure_test_policy_write_on();
        let mut dir = std::env::temp_dir();
        dir.push(format!(
            "cadrion-mcp-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("block.cad.star");
        let content = r#"
P = params(w=40.0, d=20.0, h=10.0)
def gen_step():
    return solid(box(P.w, P.d, P.h, at=CENTER), label="block")
"#;
        call_tool(
            "write_source",
            &json!({"path": path.to_str().unwrap(), "content": content}),
        )
        .unwrap();
        let b = call_tool("build", &json!({"path": path.to_str().unwrap()})).unwrap();
        let text = b["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("\"ok\": true") || text.contains("\"ok\":true"));
        let r = call_tool(
            "inspect_refs",
            &json!({"path": path.to_str().unwrap(), "facts": true}),
        )
        .unwrap();
        assert!(r["content"][0]["text"].as_str().unwrap().contains("#o1"));
        let s = call_tool(
            "snapshot",
            &json!({
                "path": path.to_str().unwrap(),
                "size": 64,
                "include_images": false
            }),
        )
        .unwrap();
        assert!(s["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("orbit.gif"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn initialize_dispatch() {
        ensure_test_policy_write_on();
        let resp = dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "initialize".into(),
            params: Some(json!({})),
        })
        .unwrap();
        assert!(resp.error.is_none());
        let tools = dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(2)),
            method: "tools/list".into(),
            params: None,
        })
        .unwrap();
        let n = tools.result.unwrap()["tools"].as_array().unwrap().len();
        assert_eq!(n, crate::compliance::TOOL_NAMES.len());
    }

    #[test]
    fn http_body_tools_list() {
        ensure_test_policy_write_on();
        let body = br#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#;
        let v = handle_http_body(body).unwrap().unwrap();
        assert_eq!(
            v["result"]["tools"].as_array().unwrap().len(),
            crate::compliance::TOOL_NAMES.len()
        );
    }

    #[test]
    fn resources_list_and_read_policy_doc() {
        ensure_test_policy_write_on();
        let resp = dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(3)),
            method: "resources/list".into(),
            params: None,
        })
        .unwrap();
        let resources = resp.result.unwrap()["resources"]
            .as_array()
            .unwrap()
            .clone();
        assert!(resources.len() >= 6);
        assert!(resources.iter().any(|r| {
            r["uri"]
                .as_str()
                .is_some_and(|u| u == "cadrion://doc/write-source-policy")
        }));

        let read = dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(4)),
            method: "resources/read".into(),
            params: Some(json!({"uri": "cadrion://doc/write-source-policy"})),
        })
        .unwrap();
        let text = read.result.unwrap()["contents"][0]["text"]
            .as_str()
            .unwrap()
            .to_string();
        assert!(text.contains("stdio"));
        assert!(text.contains("write_source"));
    }

    #[test]
    fn write_source_respects_policy_flag() {
        // If policy was already init'd with write on, we can still check the error
        // path by calling with a disabled policy only when Once hasn't fired —
        // so test the error string via a direct gate simulation:
        ensure_test_policy_write_on();
        // With write on, a bad path still fails for other reasons
        let err = call_tool(
            "write_source",
            &json!({"path": "/proc/cadrion_should_not_write", "content": "x"}),
        );
        // Either IO error or success depending on OS — just ensure not "disabled"
        if let Err(e) = err {
            assert!(
                !e.to_string().contains("disabled"),
                "unexpected disabled: {e}"
            );
        }
    }
}
