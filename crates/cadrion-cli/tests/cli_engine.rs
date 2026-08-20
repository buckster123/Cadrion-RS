//! H4-2: `cadrion engine info|install` is honest about compile-time kernels.

use assert_cmd::cargo::cargo_bin_cmd;

#[test]
fn engine_info_json() {
    let out = cargo_bin_cmd!("cadrion")
        .args(["--json", "engine", "info"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["ok"], true);
    assert_eq!(v["compiled"]["mock"], true);
    assert_eq!(v["prebuilt_fetch"], false);
    assert!(v["install"]["occt"]["method"]
        .as_str()
        .unwrap()
        .contains("features occt"));
}

#[test]
fn engine_install_matches_compiled_occt() {
    let info_out = cargo_bin_cmd!("cadrion")
        .args(["--json", "engine", "info"])
        .output()
        .unwrap();
    let info: serde_json::Value = serde_json::from_slice(&info_out.stdout).unwrap();
    let occt = info["compiled"]["occt"].as_bool().unwrap();

    let inst = cargo_bin_cmd!("cadrion")
        .args(["--json", "engine", "install", "--backend", "occt"])
        .output()
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&inst.stdout).unwrap();
    if occt {
        assert!(inst.status.success());
        assert_eq!(v["ok"], true);
        assert_eq!(v["already_present"], true);
    } else {
        assert!(!inst.status.success());
        assert_eq!(v["ok"], false);
        assert_eq!(v["diagnostics"][0]["code"], "CADRION-E-ENGINE-MISSING");
    }
}

#[test]
fn engine_install_truck_brep_default_ci_missing() {
    let info_out = cargo_bin_cmd!("cadrion")
        .args(["--json", "engine", "info"])
        .output()
        .unwrap();
    let info: serde_json::Value = serde_json::from_slice(&info_out.stdout).unwrap();
    let compiled = info["compiled"]["truck_brep"].as_bool().unwrap();

    let inst = cargo_bin_cmd!("cadrion")
        .args(["--json", "engine", "install", "--backend", "truck-brep"])
        .output()
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&inst.stdout).unwrap();
    if compiled {
        assert_eq!(v["already_present"], true);
    } else {
        assert_eq!(v["diagnostics"][0]["code"], "CADRION-E-ENGINE-MISSING");
    }
}
