//! `cadrion schema` — live surface dump (D13).
//!
//! Honesty: this is **not** a generated JSON Schema of every response body.
//! CLI comes from clap, MCP from `tool_defs`, API from `openapi_doc`, errors
//! from `ERROR_CATALOG`. Drift tests pin those four sources together.

use clap::{Command, CommandFactory};
use serde_json::{json, Value};

use cadrion_kernel::ERROR_CATALOG;

use crate::cli::{Cli, SchemaArgs, SchemaFace};
use crate::output::{emit, ExitCode};

const HONESTY: &str = "live-surfaces: clap + cadrion_mcp::tool_defs + cadrion_api::openapi_doc + ERROR_CATALOG — not a generated JSON Schema of every response body";

pub fn run(cli: &Cli, args: &SchemaArgs) -> ExitCode {
    let face = args.face.unwrap_or(SchemaFace::All);
    let body = dump(face);
    emit(cli.json, &body, true);
    ExitCode::Ok
}

pub fn dump(face: SchemaFace) -> Value {
    match face {
        SchemaFace::All => json!({
            "ok": true,
            "cadrion": env!("CARGO_PKG_VERSION"),
            "source": "live-surfaces",
            "honesty": HONESTY,
            "cli": cli_schema(),
            "mcp": mcp_schema(),
            "api": api_schema(),
            "errors": errors_schema(),
        }),
        SchemaFace::Cli => wrap("cli", cli_schema()),
        SchemaFace::Mcp => wrap("mcp", mcp_schema()),
        SchemaFace::Api => wrap("api", api_schema()),
        SchemaFace::Errors => wrap("errors", errors_schema()),
    }
}

fn wrap(key: &str, value: Value) -> Value {
    json!({
        "ok": true,
        "cadrion": env!("CARGO_PKG_VERSION"),
        "source": "live-surfaces",
        "honesty": HONESTY,
        key: value,
    })
}

fn cli_schema() -> Value {
    let cmd = Cli::command();
    json!({
        "name": cmd.get_name(),
        "about": cmd.get_about().map(|s| s.to_string()),
        "args": args_of(&cmd),
        "commands": subcommands_of(&cmd),
    })
}

fn subcommands_of(cmd: &Command) -> Vec<Value> {
    let mut out: Vec<Value> = cmd
        .get_subcommands()
        .filter(|s| !s.is_hide_set())
        .map(|s| {
            json!({
                "name": s.get_name(),
                "about": s.get_about().map(|a| a.to_string()),
                "args": args_of(s),
                "commands": subcommands_of(s),
            })
        })
        .collect();
    out.sort_by(|a, b| {
        a["name"]
            .as_str()
            .unwrap_or("")
            .cmp(b["name"].as_str().unwrap_or(""))
    });
    out
}

fn args_of(cmd: &Command) -> Vec<Value> {
    cmd.get_arguments()
        .filter(|a| !a.is_hide_set())
        .filter(|a| {
            let id = a.get_id().as_str();
            id != "help" && id != "version"
        })
        .map(|a| {
            json!({
                "id": a.get_id().to_string(),
                "long": a.get_long(),
                "short": a.get_short().map(|c| c.to_string()),
                "required": a.is_required_set(),
                "help": a.get_help().map(|h| h.to_string()),
            })
        })
        .collect()
}

fn mcp_schema() -> Value {
    let defs = cadrion_mcp::tool_defs();
    json!({
        "implementation": "hand-rolled",
        "protocol": cadrion_mcp::PROTOCOL_VERSION,
        "tools": defs,
        "tool_names": cadrion_mcp::TOOL_NAMES,
    })
}

fn api_schema() -> Value {
    cadrion_api::openapi_doc()
}

fn errors_schema() -> Value {
    json!({
        "codes": ERROR_CATALOG
            .iter()
            .map(|c| json!({"code": c.code, "meaning": c.meaning}))
            .collect::<Vec<_>>(),
        "count": ERROR_CATALOG.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadrion_mcp::TOOL_NAMES;

    #[test]
    fn dump_all_has_four_faces() {
        let v = dump(SchemaFace::All);
        assert_eq!(v["ok"], true);
        assert!(v["cli"]["commands"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["name"] == "schema"));
        assert!(v["mcp"]["tool_names"]
            .as_array()
            .unwrap()
            .iter()
            .any(|n| n == "inspect_dims"));
        assert_eq!(v["api"]["openapi"], "3.1.0");
        assert!(v["errors"]["count"].as_u64().unwrap() >= 20);
    }

    #[test]
    fn mcp_names_match_compliance() {
        let v = dump(SchemaFace::Mcp);
        let names: Vec<&str> = v["mcp"]["tool_names"]
            .as_array()
            .unwrap()
            .iter()
            .map(|n| n.as_str().unwrap())
            .collect();
        assert_eq!(names, TOOL_NAMES);
        let defs = v["mcp"]["tools"].as_array().unwrap();
        let def_names: Vec<&str> = defs.iter().map(|t| t["name"].as_str().unwrap()).collect();
        assert_eq!(def_names, TOOL_NAMES);
    }
}
