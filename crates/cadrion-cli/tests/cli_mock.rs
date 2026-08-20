//! CLI integration tests (mock kernel — no OCCT).

use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use std::fs;
use tempfile::tempdir;

const BOX_STAR: &str = r#"
P = params(w=100.0, d=60.0, h=20.0)
def gen_step():
    return solid(box(P.w, P.d, P.h, at=CENTER), label="block")
"#;

#[test]
fn build_writes_ir_json() {
    let dir = tempdir().unwrap();
    let star = dir.path().join("block.cad.star");
    fs::write(&star, BOX_STAR).unwrap();

    cargo_bin_cmd!("cadrion")
        .current_dir(dir.path())
        .args(["--json", "build", "block.cad.star"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ok\": true"))
        .stdout(predicate::str::contains("ir"));

    let ir = dir.path().join("block.ir.json");
    assert!(ir.is_file(), "expected {}", ir.display());
    let text = fs::read_to_string(ir).unwrap();
    assert!(text.contains("\"op\": \"box\"") || text.contains("\"op\":\"box\""));
}

#[test]
fn build_refuses_directory() {
    let dir = tempdir().unwrap();
    cargo_bin_cmd!("cadrion")
        .current_dir(dir.path())
        .args(["--json", "build", "."])
        .assert()
        .failure()
        .code(2)
        .stdout(predicate::str::contains("CADRION-E-EXPLICIT-TARGET"));
}

#[test]
fn inspect_refs_json() {
    let dir = tempdir().unwrap();
    let star = dir.path().join("block.cad.star");
    fs::write(&star, BOX_STAR).unwrap();

    cargo_bin_cmd!("cadrion")
        .current_dir(dir.path())
        .args(["--json", "inspect", "refs", "block.cad.star", "--facts"])
        .assert()
        .success()
        .stdout(predicate::str::contains("#o1"))
        .stdout(predicate::str::contains("\"faces\": 6"));
}

#[test]
fn inspect_measure_thickness() {
    let dir = tempdir().unwrap();
    let star = dir.path().join("block.cad.star");
    fs::write(&star, BOX_STAR).unwrap();

    // First get face selectors from refs
    let out = cargo_bin_cmd!("cadrion")
        .current_dir(dir.path())
        .args(["--json", "inspect", "refs", "block.cad.star"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let refs = v["refs"].as_array().unwrap();
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
    let a = top["selector"].as_str().unwrap();
    let b = bot["selector"].as_str().unwrap();

    cargo_bin_cmd!("cadrion")
        .current_dir(dir.path())
        .args([
            "--json",
            "inspect",
            "measure",
            "block.cad.star",
            a,
            b,
            "--kind",
            "thickness",
        ])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("\"value\": 20.0")
                .or(predicate::str::contains("\"value\":20.0")),
        );
}

#[test]
fn truck_export_step_is_unsupported_and_writes_nothing() {
    let dir = tempdir().unwrap();
    let star = dir.path().join("block.cad.star");
    fs::write(&star, BOX_STAR).unwrap();
    let step = dir.path().join("block.step");
    cargo_bin_cmd!("cadrion")
        .current_dir(dir.path())
        .args([
            "--json",
            "--kernel",
            "truck",
            "export",
            "block.cad.star",
            "--format",
            "step",
            "-o",
            "block.step",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("CADRION-E-UNSUPPORTED"));
    assert!(!step.exists(), "truck refuse must not write STEP");
}

#[test]
fn version_json() {
    cargo_bin_cmd!("cadrion")
        .args(["--json", "version"])
        .assert()
        .success()
        .stdout(predicate::str::contains("cadrion"));
}

#[test]
fn parts_search_fetch_lock_local_catalog() {
    use std::path::PathBuf;
    let dir = tempdir().unwrap();
    let parts = dir.path().join("parts");
    fs::create_dir_all(&parts).unwrap();
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/assembly/parts/m6_bolt.step");
    fs::copy(&src, parts.join("m6_bolt.step")).unwrap();

    cargo_bin_cmd!("cadrion")
        .current_dir(dir.path())
        .args(["--json", "parts", "search", "m6"])
        .assert()
        .success()
        .stdout(predicate::str::contains("m6_bolt"))
        .stdout(predicate::str::contains("\"storefront\": false"));

    cargo_bin_cmd!("cadrion")
        .current_dir(dir.path())
        .args(["--json", "parts", "show", "m6_bolt"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"downloaded\": false"));

    cargo_bin_cmd!("cadrion")
        .current_dir(dir.path())
        .args(["--json", "parts", "lock", "m6_bolt"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"verified\": true"));
    assert!(dir.path().join("parts.lock").is_file());

    cargo_bin_cmd!("cadrion")
        .current_dir(dir.path())
        .args(["--json", "parts", "fetch", "nope"])
        .assert()
        .failure()
        .code(4)
        .stdout(predicate::str::contains("CADRION-E-PARTS-NOT-FOUND"));
}
