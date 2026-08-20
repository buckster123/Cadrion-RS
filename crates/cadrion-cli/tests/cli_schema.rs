//! H4-1: `cadrion schema` dumps live faces and stays aligned with MCP/API sources.

use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;

#[test]
fn schema_all_json() {
    let out = cargo_bin_cmd!("cadrion")
        .args(["--json", "schema"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["ok"], true);
    assert_eq!(v["source"], "live-surfaces");
    assert!(v["cli"]["commands"]
        .as_array()
        .unwrap()
        .iter()
        .any(|c| c["name"] == "schema"));
    assert!(v["mcp"]["tool_names"]
        .as_array()
        .unwrap()
        .iter()
        .any(|n| n == "sdf_sample"));
    assert_eq!(v["api"]["openapi"], "3.1.0");
    assert!(v["api"]["paths"]["/v1/build"].is_object());
    assert!(v["api"]["paths"]["/v1/inspect/align"].is_object());
    assert!(v["api"]["paths"]["/v1/inspect/frame"].is_object());
    assert!(v["api"]["paths"]["/v1/inspect/diff"].is_object());
    assert!(v["api"]["paths"]["/v1/export"].is_object());
    assert!(v["api"]["paths"]["/v1/fab/check"].is_object());
    assert!(v["api"]["paths"]["/v1/engine"].is_object());
    assert!(v["api"]["paths"]["/v1/schema"].is_object());
    assert!(v["api"]["paths"]["/v1/robot/gen"].is_object());
    assert!(v["api"]["paths"]["/v1/robot/validate"].is_object());
    let codes = v["errors"]["codes"].as_array().unwrap();
    assert!(codes.iter().any(|c| c["code"] == "CADRION-E-HERMETIC-LOAD"));
    assert!(codes
        .iter()
        .any(|c| c["code"] == "CADRION-E-EXPLICIT-TARGET"));
}

#[test]
fn schema_errors_face() {
    cargo_bin_cmd!("cadrion")
        .args(["--json", "schema", "errors"])
        .assert()
        .success()
        .stdout(predicate::str::contains("CADRION-E-UNSUPPORTED"))
        .stdout(predicate::str::contains("\"cli\"").not());
}

#[test]
fn schema_mcp_matches_tool_names() {
    let out = cargo_bin_cmd!("cadrion")
        .args(["--json", "schema", "mcp"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let names: Vec<&str> = v["mcp"]["tool_names"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n.as_str().unwrap())
        .collect();
    let defs: Vec<&str> = v["mcp"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, defs);
    assert!(names.contains(&"inspect_dims"));
    assert!(names.contains(&"assembly_validate"));
    assert!(names.contains(&"align_check"));
    assert!(names.contains(&"frame"));
    assert!(names.contains(&"diff"));
    assert!(names.contains(&"export"));
    assert!(names.contains(&"fab_check"));
    assert!(names.contains(&"engine"));
    assert!(names.contains(&"schema"));
    assert!(names.contains(&"robot"));
}
