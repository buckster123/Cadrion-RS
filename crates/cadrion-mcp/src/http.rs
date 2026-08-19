//! Streamable HTTP transport for MCP (JSON-RPC over POST + optional SSE).

use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::extract::State;
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{json, Value};
use tokio_stream::wrappers::IntervalStream;
use tokio_stream::StreamExt;

use crate::server::handle_http_body;

#[derive(Clone)]
pub struct HttpMcpConfig {
    pub bind: String,
    pub token: Option<String>,
}

#[derive(Clone)]
struct AppState {
    token: Option<String>,
}

/// Block until the server exits (never, unless bind fails).
pub async fn serve_http(cfg: HttpMcpConfig) -> Result<(), String> {
    crate::policy::init_policy(crate::policy::McpPolicy {
        // H7: HTTP write_source ON unless CADRION_MCP_WRITE_SOURCE=0
        write_source: crate::policy::write_source_from_env(true),
        project_root: crate::policy::project_root_from_env(),
        transport: "http",
    });
    let addr: SocketAddr = cfg
        .bind
        .parse()
        .map_err(|e| format!("bad bind {}: {e}", cfg.bind))?;
    let state = AppState {
        token: cfg.token.clone(),
    };
    let app = Router::new()
        .route("/health", get(health))
        .route("/mcp", post(mcp_post).get(mcp_sse))
        .route("/mcp/", post(mcp_post).get(mcp_sse))
        .with_state(Arc::new(state));

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| format!("bind {addr}: {e}"))?;
    eprintln!(
        "cadrion-mcp {} http on {} write_source={}",
        crate::VERSION,
        cfg.bind,
        crate::policy::policy().write_source
    );
    axum::serve(listener, app)
        .await
        .map_err(|e| format!("serve: {e}"))
}

async fn health(State(st): State<Arc<AppState>>) -> Json<Value> {
    Json(json!({
        "ok": true,
        "service": "cadrion-mcp",
        "version": crate::VERSION,
        "transport": "streamable-http",
        "auth": st.token.is_some(),
        "endpoints": {
            "rpc": "POST /mcp",
            "sse": "GET /mcp",
            "health": "GET /health"
        }
    }))
}

fn auth_ok(st: &AppState, headers: &HeaderMap) -> bool {
    let Some(expected) = &st.token else {
        return true;
    };
    if let Some(h) = headers.get(header::AUTHORIZATION) {
        if let Ok(s) = h.to_str() {
            if let Some(rest) = s.strip_prefix("Bearer ") {
                return rest == expected;
            }
            if s == expected {
                return true;
            }
        }
    }
    if let Some(h) = headers.get("x-cadrion-token") {
        if let Ok(s) = h.to_str() {
            return s == expected;
        }
    }
    false
}

async fn mcp_post(State(st): State<Arc<AppState>>, headers: HeaderMap, body: Body) -> Response {
    if !auth_ok(&st, &headers) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "jsonrpc": "2.0",
                "error": {"code": -32001, "message": "unauthorized"},
                "id": null
            })),
        )
            .into_response();
    }

    let bytes = match axum::body::to_bytes(body, 8 * 1024 * 1024).await {
        Ok(b) => b,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "jsonrpc": "2.0",
                    "error": {"code": -32700, "message": format!("body read: {e}")},
                    "id": null
                })),
            )
                .into_response();
        }
    };

    // Accept naked JSON-RPC or { "jsonrpc": ... } already.
    // Also accept MCP-ish wrapper { "method": ..., "params": ... } without jsonrpc.
    let body = if bytes.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "jsonrpc": "2.0",
                "error": {"code": -32600, "message": "empty body"},
                "id": null
            })),
        )
            .into_response();
    } else {
        bytes
    };

    match handle_http_body(&body) {
        Ok(Some(v)) => (StatusCode::OK, Json(v)).into_response(),
        Ok(None) => {
            // notification — 202 Accepted, empty body (streamable HTTP style)
            StatusCode::ACCEPTED.into_response()
        }
        Err(e) => (
            StatusCode::OK, // JSON-RPC errors still 200 per many MCP clients
            Json(json!({
                "jsonrpc": "2.0",
                "error": {"code": -32700, "message": e},
                "id": null
            })),
        )
            .into_response(),
    }
}

/// Minimal SSE stream for clients that open GET /mcp (heartbeats + hello).
async fn mcp_sse(
    State(st): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>>, StatusCode> {
    if !auth_ok(&st, &headers) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let stream = IntervalStream::new(tokio::time::interval(Duration::from_secs(15))).map(|_| {
        Ok(Event::default()
            .event("ping")
            .data(format!(r#"{{"ok":true,"ts":{}}}"#, unix_ts())))
    });
    // First event is immediate hello via keep-alive comment + first ping shortly
    Ok(Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(20))
            .text("cadrion-mcp"),
    ))
}

fn unix_ts() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    fn app(token: Option<&str>) -> Router {
        let state = Arc::new(AppState {
            token: token.map(|s| s.to_string()),
        });
        Router::new()
            .route("/health", get(health))
            .route("/mcp", post(mcp_post).get(mcp_sse))
            .with_state(state)
    }

    #[tokio::test]
    async fn health_and_tools_list() {
        let app = app(None);
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let res = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/mcp")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(res.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let v: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            v["result"]["tools"].as_array().unwrap().len(),
            crate::compliance::TOOL_NAMES.len()
        );
    }

    #[tokio::test]
    async fn auth_required() {
        let app = app(Some("secret"));
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/mcp")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

        let res = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/mcp")
                    .header("content-type", "application/json")
                    .header("authorization", "Bearer secret")
                    .body(Body::from(r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }
}
