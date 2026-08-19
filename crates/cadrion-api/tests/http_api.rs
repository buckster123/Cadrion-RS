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
