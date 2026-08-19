//! `cadrion assembly`

use std::fs;
use std::path::{Path, PathBuf};

use cadrion_parts::{assembly_kinematics, assembly_to_robot_json, validate_assembly, AssemblySpec};
use serde_json::json;

use crate::cli::{AssemblyArgs, AssemblyCmd, AssemblyEmitArgs, Cli};
use crate::output::{emit, ExitCode};

pub fn run(cli: &Cli, args: &AssemblyArgs) -> ExitCode {
    match &args.cmd {
        AssemblyCmd::Validate(a) => validate(cli, &a.target),
        AssemblyCmd::EmitKinematics(a) => emit_kinematics(cli, a),
        AssemblyCmd::EmitRobot(a) => emit_robot(cli, a),
    }
}

fn load_spec(cli: &Cli, path: &Path) -> Result<AssemblySpec, ExitCode> {
    let text = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) => {
            emit(
                cli.json,
                &json!({"ok": false, "error": format!("read {}: {e}", path.display())}),
                false,
            );
            return Err(ExitCode::Io);
        }
    };
    match serde_json::from_str(&text) {
        Ok(s) => Ok(s),
        Err(e) => {
            emit(
                cli.json,
                &json!({"ok": false, "error": format!("parse assembly: {e}")}),
                false,
            );
            Err(ExitCode::Validation)
        }
    }
}

fn validate(cli: &Cli, path: &Path) -> ExitCode {
    let spec = match load_spec(cli, path) {
        Ok(s) => s,
        Err(c) => return c,
    };
    let report = validate_assembly(&spec);
    emit(
        cli.json,
        &json!({
            "ok": report.ok,
            "path": path,
            "report": report,
        }),
        report.ok,
    );
    if report.ok {
        ExitCode::Ok
    } else {
        ExitCode::Validation
    }
}

fn default_out(target: &Path, suffix: &str) -> PathBuf {
    let stem = target
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("assembly");
    let stem = stem.strip_suffix(".assy").unwrap_or(stem);
    target
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(format!("{stem}.{suffix}"))
}

fn emit_kinematics(cli: &Cli, a: &AssemblyEmitArgs) -> ExitCode {
    let spec = match load_spec(cli, &a.target) {
        Ok(s) => s,
        Err(c) => return c,
    };
    let kin = match assembly_kinematics(&spec) {
        Ok(k) => k,
        Err(errs) => {
            emit(
                cli.json,
                &json!({"ok": false, "errors": errs, "note": "validate assembly first"}),
                false,
            );
            return ExitCode::Validation;
        }
    };
    let out = a
        .out
        .clone()
        .unwrap_or_else(|| default_out(&a.target, "kinematics.json"));
    if let Err(e) = fs::write(&out, serde_json::to_string_pretty(&kin).unwrap_or_default()) {
        emit(
            cli.json,
            &json!({"ok": false, "error": format!("write {}: {e}", out.display())}),
            false,
        );
        return ExitCode::Io;
    }
    emit(
        cli.json,
        &json!({
            "ok": true,
            "out": out,
            "kinematics": kin,
            "note": "H3-4 OQ-4 partial — not AP242 STEP kinematics",
        }),
        true,
    );
    ExitCode::Ok
}

fn emit_robot(cli: &Cli, a: &AssemblyEmitArgs) -> ExitCode {
    let spec = match load_spec(cli, &a.target) {
        Ok(s) => s,
        Err(c) => return c,
    };
    let robot = match assembly_to_robot_json(&spec) {
        Ok(v) => v,
        Err(errs) => {
            emit(cli.json, &json!({"ok": false, "errors": errs}), false);
            return ExitCode::Validation;
        }
    };
    let out = a
        .out
        .clone()
        .unwrap_or_else(|| default_out(&a.target, "robot.json"));
    if let Err(e) = fs::write(
        &out,
        serde_json::to_string_pretty(&robot).unwrap_or_default(),
    ) {
        emit(
            cli.json,
            &json!({"ok": false, "error": format!("write {}: {e}", out.display())}),
            false,
        );
        return ExitCode::Io;
    }
    emit(
        cli.json,
        &json!({
            "ok": true,
            "out": out,
            "robot": robot,
            "note": "placeholder geometry — feed to `cadrion robot gen` for URDF/SRDF/SDF",
        }),
        true,
    );
    ExitCode::Ok
}
