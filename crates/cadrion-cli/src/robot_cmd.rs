//! `cadrion robot`

use std::fs;
use std::path::{Path, PathBuf};

use cadrion_robot::{
    emit_and_validate, parse_urdf_xml, srdf_from_robot, validate_sdf_xml, validate_urdf_xml,
    write_sdf, write_srdf, RobotSpec, ValidationReport,
};
use serde_json::json;

use crate::cli::{Cli, RobotArgs, RobotCmd};
use crate::output::{emit, ExitCode};

pub fn run(cli: &Cli, args: &RobotArgs) -> ExitCode {
    match &args.cmd {
        RobotCmd::Validate(a) => validate(cli, &a.target, a.emit_dir.as_ref()),
        RobotCmd::Gen(a) => gen(cli, &a.spec, a.out.as_ref(), a.srdf, a.sdf),
    }
}

fn load_spec(path: &Path) -> Result<RobotSpec, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let jd = &mut serde_json::Deserializer::from_str(&text);
    match serde_path_to_error::deserialize(jd) {
        Ok(v) => Ok(v),
        Err(err) => Err(format!(
            "robot json ({} bytes from {}): path={} err={}",
            text.len(),
            path.display(),
            err.path(),
            err.inner()
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_example_arm() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/robots/simple_arm.robot.json");
        let r = load_spec(&path).expect("load");
        assert_eq!(r.name, "simple_arm");
    }
}

fn gen(cli: &Cli, spec: &Path, out: Option<&PathBuf>, with_srdf: bool, with_sdf: bool) -> ExitCode {
    let robot = match load_spec(spec) {
        Ok(r) => r,
        Err(e) => {
            emit(
                cli.json,
                &json!({"ok": false, "diagnostics": [{"code": "CADRION-E-ROBOT", "message": e}]}),
                false,
            );
            return ExitCode::Io;
        }
    };
    let (urdf, report) = emit_and_validate(&robot);
    if !report.ok {
        emit(cli.json, &json!({"ok": false, "report": report}), false);
        return ExitCode::Validation;
    }

    let out_dir = out
        .cloned()
        .unwrap_or_else(|| spec.parent().unwrap_or(Path::new(".")).to_path_buf());
    let _ = fs::create_dir_all(&out_dir);
    let urdf_path = out_dir.join(format!("{}.urdf", robot.name));
    if let Err(e) = fs::write(&urdf_path, &urdf) {
        emit(
            cli.json,
            &json!({"ok": false, "diagnostics": [{"code": "CADRION-E-IO", "message": e.to_string()}]}),
            false,
        );
        return ExitCode::Io;
    }

    let mut files = vec![urdf_path.display().to_string()];
    if with_srdf {
        let srdf = srdf_from_robot(&robot, "arm");
        let p = out_dir.join(format!("{}.srdf", robot.name));
        let _ = fs::write(&p, write_srdf(&srdf));
        files.push(p.display().to_string());
    }
    if with_sdf {
        let p = out_dir.join(format!("{}.sdf", robot.name));
        let _ = fs::write(&p, write_sdf(&robot));
        files.push(p.display().to_string());
    }

    emit(
        cli.json,
        &json!({"ok": true, "report": report, "files": files}),
        true,
    );
    ExitCode::Ok
}

fn validate(cli: &Cli, target: &Path, emit_dir: Option<&PathBuf>) -> ExitCode {
    let name = target
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    if name.ends_with(".json") {
        return gen(cli, target, emit_dir, true, true);
    }

    let text = match fs::read_to_string(target) {
        Ok(t) => t,
        Err(e) => {
            emit(
                cli.json,
                &json!({"ok": false, "diagnostics": [{"code": "CADRION-E-IO", "message": e.to_string()}]}),
                false,
            );
            return ExitCode::Io;
        }
    };

    let report: ValidationReport = if name.ends_with(".urdf") {
        let mut r = validate_urdf_xml(&text);
        if let Err(e) = parse_urdf_xml(&text) {
            r.errors.push(e);
            r.ok = false;
        }
        r
    } else if name.ends_with(".srdf") {
        let mut r = ValidationReport {
            ok: true,
            kind: "srdf_xml".into(),
            errors: vec![],
            warnings: vec![],
        };
        if !text.contains("<robot") || !text.contains("<group") {
            r.errors.push("SRDF missing robot/group".into());
            r.ok = false;
        }
        r
    } else if name.ends_with(".sdf") {
        validate_sdf_xml(&text)
    } else {
        emit(
            cli.json,
            &json!({"ok": false, "diagnostics": [{"code": "CADRION-E-USAGE", "message": "expected .json/.urdf/.srdf/.sdf"}]}),
            false,
        );
        return ExitCode::Usage;
    };

    let ok = report.ok;
    emit(
        cli.json,
        &json!({"ok": ok, "report": report, "target": target}),
        ok,
    );
    if ok {
        ExitCode::Ok
    } else {
        ExitCode::Validation
    }
}
