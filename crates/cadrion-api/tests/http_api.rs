//! HTTP API integration tests.

use std::path::PathBuf;
use std::time::Duration;

use axum::Router;
use axum_test::TestServer;
use cadrion_api::{router, AppConfig, AppState};
use serde_json::json;

fn example_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/assembly")
}

fn app() -> Router {
    let cfg = AppConfig {
        bind: "127.0.0.1:0".into(),
        token: Some("test-token".into()),
        project_root: example_root(),
    };
    router(AppState::new(cfg))
}

#[tokio::test]
async fn health_and_openapi() {
    let server = TestServer::new(app()).unwrap();
    // health without auth
    let r = server.get("/v1/health").await;
    r.assert_status_ok();
    let o = server.get("/v1/openapi.json").await;
    o.assert_status_ok();
    let v: serde_json::Value = o.json();
    assert_eq!(v["openapi"], "3.1.0");
    assert!(v["paths"]["/v1/inspect/measure"].is_object());
    assert!(v["paths"]["/v1/inspect/dims"].is_object());
    assert!(v["paths"]["/v1/inspect/align"].is_object());
    assert!(v["paths"]["/v1/inspect/frame"].is_object());
    assert!(v["paths"]["/v1/inspect/diff"].is_object());
    assert!(v["paths"]["/v1/sdf/sample"].is_object());
    assert!(v["paths"]["/v1/export"].is_object());
    assert!(v["paths"]["/v1/fab/check"].is_object());
}

fn mcp_payload(v: &serde_json::Value) -> serde_json::Value {
    let text = v["content"][0]["text"].as_str().expect("mcp text content");
    serde_json::from_str(text).expect("mcp payload json")
}

#[tokio::test]
async fn auth_required_on_build() {
    let server = TestServer::new(app()).unwrap();
    let r = server
        .post("/v1/build")
        .json(&json!({"path": "cad/plate.cad.star"}))
        .await;
    r.assert_status_unauthorized();

    let r = server
        .post("/v1/build")
        .add_header("Authorization", "Bearer test-token")
        .json(&json!({"path": "cad/plate.cad.star"}))
        .await;
    r.assert_status_ok();
    let v: serde_json::Value = r.json();
    assert!(v["content"][0]["text"].as_str().unwrap().contains("ok"));
}

#[tokio::test]
async fn assembly_validate_lock() {
    let server = TestServer::new(app()).unwrap();
    let r = server
        .post("/v1/assembly/validate")
        .add_header("Authorization", "Bearer test-token")
        .json(&json!({"path": "plate_bolt.assy.json"}))
        .await;
    r.assert_status_ok();
    let v: serde_json::Value = r.json();
    assert_eq!(v["ok"], true);
    assert!(v["lock_verified"]
        .as_array()
        .unwrap()
        .iter()
        .any(|x| x == "m6_bolt"));
}

#[tokio::test]
async fn job_build_completes() {
    let server = TestServer::new(app()).unwrap();
    let r = server
        .post("/v1/jobs")
        .add_header("Authorization", "Bearer test-token")
        .json(&json!({
            "kind": "build",
            "payload": {"path": "cad/plate.cad.star"}
        }))
        .await;
    r.assert_status_ok();
    let v: serde_json::Value = r.json();
    let id = v["job"]["id"].as_str().unwrap().to_string();

    let mut done = false;
    for _ in 0..50 {
        tokio::time::sleep(Duration::from_millis(20)).await;
        let g = server
            .get(&format!("/v1/jobs/{id}"))
            .add_header("Authorization", "Bearer test-token")
            .await;
        g.assert_status_ok();
        let j: serde_json::Value = g.json();
        let st = j["job"]["status"].as_str().unwrap();
        if st == "completed" {
            done = true;
            break;
        }
        if st == "failed" {
            panic!("job failed: {j}");
        }
    }
    assert!(done, "job did not complete");
}

#[tokio::test]
async fn parts_search() {
    let server = TestServer::new(app()).unwrap();
    let r = server
        .post("/v1/parts/search")
        .add_header("Authorization", "Bearer test-token")
        .json(&json!({"path": ".", "query": "m6", "parts_root": example_root().join("parts").display().to_string()}))
        .await;
    r.assert_status_ok();
    let v: serde_json::Value = r.json();
    assert!(v["results"]
        .as_array()
        .unwrap()
        .iter()
        .any(|c| c["id"] == "m6_bolt"));
}

#[tokio::test]
async fn inspect_measure_plate_thickness() {
    let server = TestServer::new(app()).unwrap();
    let refs = server
        .post("/v1/inspect/refs")
        .add_header("Authorization", "Bearer test-token")
        .json(&json!({"path": "cad/plate.cad.star"}))
        .await;
    refs.assert_status_ok();
    let report = mcp_payload(&refs.json());
    let refs = report["refs"].as_array().unwrap();
    let top = refs
        .iter()
        .find(|r| {
            r["kind"] == "face"
                && r["normal"]
                    .as_object()
                    .map(|n| n.get("z").and_then(|z| z.as_f64()) == Some(1.0))
                    .unwrap_or(false)
        })
        .unwrap();
    let bot = refs
        .iter()
        .find(|r| {
            r["kind"] == "face"
                && r["normal"]
                    .as_object()
                    .map(|n| n.get("z").and_then(|z| z.as_f64()) == Some(-1.0))
                    .unwrap_or(false)
        })
        .unwrap();
    let r = server
        .post("/v1/inspect/measure")
        .add_header("Authorization", "Bearer test-token")
        .json(&json!({
            "path": "cad/plate.cad.star",
            "a": top["selector"],
            "b": bot["selector"],
            "kind": "thickness"
        }))
        .await;
    r.assert_status_ok();
    let m = mcp_payload(&r.json());
    let val = m["value"].as_f64().unwrap_or(0.0);
    assert!((val - 5.0).abs() < 1e-6, "thickness {m}");

    let a = server
        .post("/v1/inspect/align")
        .add_header("Authorization", "Bearer test-token")
        .json(&json!({
            "path": "cad/plate.cad.star",
            "a": top["selector"],
            "b": bot["selector"],
            "expect": "coaxial"
        }))
        .await;
    a.assert_status_ok();
    let ar = mcp_payload(&a.json());
    assert_eq!(ar["ok"], true, "{ar}");

    let f = server
        .post("/v1/inspect/frame")
        .add_header("Authorization", "Bearer test-token")
        .json(&json!({
            "path": "cad/plate.cad.star",
            "selector": top["selector"]
        }))
        .await;
    f.assert_status_ok();
    let fr = mcp_payload(&f.json());
    assert_eq!(fr["kind"], "face", "{fr}");

    let d = server
        .post("/v1/inspect/diff")
        .add_header("Authorization", "Bearer test-token")
        .json(&json!({
            "old": "cad/plate.cad.star",
            "new": "cad/plate.cad.star"
        }))
        .await;
    d.assert_status_ok();
    let dr = mcp_payload(&d.json());
    assert_eq!(dr["ok"], true);
    assert_eq!(dr["diff"]["volume_delta_mm3"], 0.0);
}

#[tokio::test]
async fn inspect_dims_writes_packet() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("plate.drawing.json");
    let server = TestServer::new(app()).unwrap();
    let r = server
        .post("/v1/inspect/dims")
        .add_header("Authorization", "Bearer test-token")
        .json(&json!({
            "path": "cad/plate.cad.star",
            "out": out.display().to_string()
        }))
        .await;
    r.assert_status_ok();
    let p = mcp_payload(&r.json());
    assert_eq!(p["ok"], true);
    assert!(out.is_file(), "expected {}", out.display());
}

#[tokio::test]
async fn sdf_sample_box() {
    let dir = tempfile::tempdir().unwrap();
    let server = TestServer::new(app()).unwrap();
    let r = server
        .post("/v1/sdf/sample")
        .add_header("Authorization", "Bearer test-token")
        .json(&json!({
            "prim": "box",
            "a": 10.0,
            "b": 8.0,
            "c": 6.0,
            "res": 16,
            "out": dir.path().display().to_string(),
            "stem": "http_box"
        }))
        .await;
    r.assert_status_ok();
    let p = mcp_payload(&r.json());
    assert_eq!(p["ok"], true);
    assert_eq!(p["secondary"], true);
}

#[tokio::test]
async fn export_stl_and_refuse_step() {
    let dir = tempfile::tempdir().unwrap();
    let stl = dir.path().join("plate.stl");
    let step = dir.path().join("plate.step");
    let server = TestServer::new(app()).unwrap();
    let ok = server
        .post("/v1/export")
        .add_header("Authorization", "Bearer test-token")
        .json(&json!({
            "path": "cad/plate.cad.star",
            "format": "stl",
            "out": stl.display().to_string()
        }))
        .await;
    ok.assert_status_ok();
    let p = mcp_payload(&ok.json());
    assert_eq!(p["ok"], true);
    assert_eq!(p["mesh"], "ir-analytic-preview");
    assert!(stl.is_file());

    let bad = server
        .post("/v1/export")
        .add_header("Authorization", "Bearer test-token")
        .json(&json!({
            "path": "cad/plate.cad.star",
            "format": "step",
            "out": step.display().to_string()
        }))
        .await;
    assert_eq!(bad.status_code().as_u16(), 400);
    let body = format!("{}", bad.text());
    assert!(body.contains("CADRION-E-UNSUPPORTED"), "{body}");
    assert!(!step.exists(), "mock must not write STEP");
}

fn plate_flat_json() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/fab/plate.flat.json")
}

#[tokio::test]
async fn fab_check_plate_and_unknown_profile() {
    let server = TestServer::new(app()).unwrap();
    let ok = server
        .post("/v1/fab/check")
        .add_header("Authorization", "Bearer test-token")
        .json(&json!({
            "path": plate_flat_json().display().to_string()
        }))
        .await;
    ok.assert_status_ok();
    let p = mcp_payload(&ok.json());
    assert_eq!(p["ok"], true);
    assert_eq!(p["printer_start"], false);
    assert_eq!(p["report"]["profile_id"], "sendcutsend.laser");
    assert_eq!(p["report"]["profile_version"], "1.0.0");

    let bad = server
        .post("/v1/fab/check")
        .add_header("Authorization", "Bearer test-token")
        .json(&json!({
            "path": plate_flat_json().display().to_string(),
            "profile": "not-a-vendor"
        }))
        .await;
    assert_eq!(bad.status_code().as_u16(), 400);
    let body = format!("{}", bad.text());
    assert!(body.contains("unknown profile"), "{body}");
}
