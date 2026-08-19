//! `cadrion serve`

use std::path::PathBuf;

use serde_json::json;

use crate::cli::{Cli, ServeArgs, ServeCmd};
use crate::output::{emit, ExitCode};

pub fn run(cli: &Cli, args: &ServeArgs) -> ExitCode {
    match &args.cmd {
        ServeCmd::Api(a) => serve_api(
            cli,
            a.port,
            a.host.clone(),
            a.token.clone(),
            a.project.clone(),
        ),
        ServeCmd::Mcp(a) => serve_mcp(cli, a.port, a.host.clone(), a.token.clone()),
    }
}

fn serve_api(
    cli: &Cli,
    port: u16,
    host: String,
    token: Option<String>,
    project: Option<PathBuf>,
) -> ExitCode {
    let bind = format!("{host}:{port}");
    let project_root = project
        .or_else(|| cli.project.clone())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let cfg = cadrion_api::AppConfig {
        bind: bind.clone(),
        token: token.clone(),
        project_root: project_root.clone(),
    };

    if cli.json {
        let body = json!({
            "ok": true,
            "bind": bind,
            "project_root": project_root,
            "auth": token.is_some(),
            "openapi": format!("http://{bind}/v1/openapi.json"),
        });
        emit(true, &body, true);
    } else if !cli.quiet {
        eprintln!("cadrion serve api on http://{bind}");
        eprintln!("  openapi: http://{bind}/v1/openapi.json");
        if token.is_some() {
            eprintln!("  auth: bearer token required");
        }
    }

    let rt = match tokio::runtime::Runtime::new() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("tokio runtime: {e}");
            return ExitCode::Internal;
        }
    };
    match rt.block_on(cadrion_api::serve(cfg)) {
        Ok(()) => ExitCode::Ok,
        Err(e) => {
            eprintln!("serve error: {e}");
            ExitCode::Io
        }
    }
}

fn serve_mcp(cli: &Cli, port: u16, host: String, token: Option<String>) -> ExitCode {
    let bind = format!("{host}:{port}");
    let cfg = cadrion_mcp::HttpMcpConfig {
        bind: bind.clone(),
        token: token.clone(),
    };

    if cli.json {
        emit(
            true,
            &json!({
                "ok": true,
                "bind": bind,
                "transport": "streamable-http",
                "rpc": format!("http://{bind}/mcp"),
                "sse": format!("http://{bind}/mcp"),
                "health": format!("http://{bind}/health"),
                "auth": token.is_some(),
            }),
            true,
        );
    } else if !cli.quiet {
        eprintln!("cadrion serve mcp on http://{bind}");
        eprintln!("  POST http://{bind}/mcp   — JSON-RPC (tools/list, tools/call, …)");
        eprintln!("  GET  http://{bind}/mcp   — SSE heartbeats");
        eprintln!("  GET  http://{bind}/health");
        if token.is_some() {
            eprintln!("  auth: bearer token required");
        }
    }

    let rt = match tokio::runtime::Runtime::new() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("tokio runtime: {e}");
            return ExitCode::Internal;
        }
    };
    match rt.block_on(cadrion_mcp::serve_http(cfg)) {
        Ok(()) => ExitCode::Ok,
        Err(e) => {
            eprintln!("mcp serve error: {e}");
            ExitCode::Io
        }
    }
}
