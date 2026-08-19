//! Snapshot / view CLI tests.

use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use std::fs;
use tempfile::tempdir;

const BOX: &str = r#"
def gen_step():
    return solid(box(30.0, 20.0, 10.0, at=CENTER), label="snapbox")
"#;

#[test]
fn snapshot_writes_packet() {
    let dir = tempdir().unwrap();
    let star = dir.path().join("box.cad.star");
    fs::write(&star, BOX).unwrap();

    cargo_bin_cmd!("cadrion")
        .current_dir(dir.path())
        .args([
            "--json",
            "snapshot",
            "box.cad.star",
            "--size",
            "64",
            "--gif-frames",
            "6",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ok\": true"))
        .stdout(predicate::str::contains("orbit.gif"));

    let snap = dir.path().join("box.snap");
    assert!(snap.join("iso.png").is_file());
    assert!(snap.join("front.png").is_file());
    assert!(snap.join("orbit.gif").is_file());
    assert!(snap.join("manifest.json").is_file());
}

#[test]
fn view_once_prepares_snap() {
    let dir = tempdir().unwrap();
    let star = dir.path().join("box.cad.star");
    fs::write(&star, BOX).unwrap();

    cargo_bin_cmd!("cadrion")
        .current_dir(dir.path())
        .args(["--json", "view", "box.cad.star", "--once"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"once\": true"));

    assert!(dir.path().join("box.snap/iso.png").is_file());
    let mesh = dir.path().join("box.snap/mesh.json");
    assert!(mesh.is_file(), "H2-6 mesh.json missing");
    let t = fs::read_to_string(mesh).unwrap();
    assert!(t.contains("positions"));
    assert!(t.contains("triangle_count"));
    let drawing = dir.path().join("box.snap/drawing.json");
    assert!(drawing.is_file(), "H3-5 drawing.json missing");
    let d = fs::read_to_string(drawing).unwrap();
    assert!(d.contains("cadrion.drawing_packet") || d.contains("dims"));
}

#[test]
fn view_once_gcode_writes_path_json() {
    let dir = tempdir().unwrap();
    let g = dir.path().join("sample.gcode");
    fs::write(
        &g,
        "G28\nG1 Z0.2 F3000\nG1 X10 Y10 F1200\nG1 X40 Y10 E1.2\nG1 Z0.4\nG1 X10 Y40 E1.2\n",
    )
    .unwrap();

    cargo_bin_cmd!("cadrion")
        .current_dir(dir.path())
        .args(["--json", "view", "sample.gcode", "--once"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"kind\": \"gcode\""))
        .stdout(predicate::str::contains("path.json"));

    let path_json = dir.path().join("sample.view/path.json");
    assert!(path_json.is_file());
    let text = fs::read_to_string(path_json).unwrap();
    assert!(text.contains("layers"));
    assert!(text.contains("move_count"));
}

#[test]
fn view_once_robot_writes_jog_json() {
    let dir = tempdir().unwrap();
    // copy simple arm fixture
    let src = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/robots/simple_arm.robot.json"
    );
    let dest = dir.path().join("arm.robot.json");
    fs::copy(src, &dest).unwrap();

    cargo_bin_cmd!("cadrion")
        .current_dir(dir.path())
        .args(["--json", "view", "arm.robot.json", "--once"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"kind\": \"robot\""));

    let meta = dir.path().join("arm.robot.view/robot.json");
    // stem is arm.robot -> arm.robot.view
    let meta2 = dir.path().join("arm.view/robot.json");
    let meta = if meta.is_file() {
        meta
    } else {
        // file_stem of arm.robot.json is arm.robot
        let alt = dir.path().join("arm.robot.view/robot.json");
        if alt.is_file() {
            alt
        } else {
            meta2
        }
    };
    assert!(meta.is_file(), "missing robot.json under view dir");
    let text = fs::read_to_string(meta).unwrap();
    assert!(text.contains("joint1"));
    assert!(text.contains("simple_arm") || text.contains("joints"));
}
