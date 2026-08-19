//! Local slicer CLI discovery + gated execute.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SlicerKind {
    PrusaSlicer,
    OrcaSlicer,
    BambuStudio,
    SuperSlicer,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlicerInfo {
    pub kind: SlicerKind,
    pub name: String,
    pub path: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

/// Second consent string for live slicer exec (mirrors printer `START`).
pub const CONFIRM_SLICE: &str = "SLICE";

const CANDIDATES: &[(&str, SlicerKind)] = &[
    ("prusa-slicer", SlicerKind::PrusaSlicer),
    ("prusa-slicer-console", SlicerKind::PrusaSlicer),
    ("PrusaSlicer", SlicerKind::PrusaSlicer),
    ("orca-slicer", SlicerKind::OrcaSlicer),
    ("OrcaSlicer", SlicerKind::OrcaSlicer),
    ("bambu-studio", SlicerKind::BambuStudio),
    ("BambuStudio", SlicerKind::BambuStudio),
    ("superslicer", SlicerKind::SuperSlicer),
];

/// Discover slicer binaries on PATH (and a few common install dirs).
pub fn discover_slicers() -> Vec<SlicerInfo> {
    let mut out = Vec::new();
    let mut seen = std::collections::BTreeSet::new();

    for (bin, kind) in CANDIDATES {
        if let Some(path) = which(bin) {
            let key = path.display().to_string();
            if seen.insert(key) {
                let version = probe_version(&path);
                out.push(SlicerInfo {
                    kind: *kind,
                    name: bin.to_string(),
                    path,
                    version,
                });
            }
        }
    }

    let extras = [
        "/usr/bin/prusa-slicer",
        "/usr/local/bin/prusa-slicer",
        "/usr/bin/orca-slicer",
        "/opt/bambu-studio/bambu-studio",
    ];
    for p in extras {
        let path = PathBuf::from(p);
        if path.is_file() {
            let key = path.display().to_string();
            if seen.insert(key) {
                let kind = kind_from_name(p);
                let version = probe_version(&path);
                out.push(SlicerInfo {
                    kind,
                    name: path
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("slicer")
                        .into(),
                    path,
                    version,
                });
            }
        }
    }
    out
}

fn kind_from_name(p: &str) -> SlicerKind {
    let l = p.to_ascii_lowercase();
    if l.contains("prusa") {
        SlicerKind::PrusaSlicer
    } else if l.contains("orca") {
        SlicerKind::OrcaSlicer
    } else if l.contains("bambu") {
        SlicerKind::BambuStudio
    } else if l.contains("super") {
        SlicerKind::SuperSlicer
    } else {
        SlicerKind::Unknown
    }
}

fn which(bin: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(bin);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn probe_version(path: &Path) -> Option<String> {
    let output = Command::new(path).arg("--version").output().ok()?;
    let s = String::from_utf8_lossy(&output.stdout);
    let line = s.lines().next().unwrap_or("").trim();
    if line.is_empty() {
        let e = String::from_utf8_lossy(&output.stderr);
        let line = e.lines().next().unwrap_or("").trim();
        if line.is_empty() {
            None
        } else {
            Some(line.to_string())
        }
    } else {
        Some(line.to_string())
    }
}

/// Build argv for a slicer (does not execute).
pub fn slice_argv(
    slicer: &SlicerInfo,
    mesh: &Path,
    out_gcode: &Path,
    printer_profile: Option<&str>,
) -> Vec<String> {
    match slicer.kind {
        SlicerKind::PrusaSlicer | SlicerKind::SuperSlicer | SlicerKind::OrcaSlicer => {
            let mut args = vec![
                slicer.path.display().to_string(),
                "--export-gcode".into(),
                "--output".into(),
                out_gcode.display().to_string(),
                mesh.display().to_string(),
            ];
            if let Some(p) = printer_profile {
                args.push("--load".into());
                args.push(p.into());
            }
            args
        }
        SlicerKind::BambuStudio => {
            vec![
                slicer.path.display().to_string(),
                "--slice".into(),
                mesh.display().to_string(),
            ]
        }
        SlicerKind::Unknown => {
            // Generic: pass mesh + -o out (works for stub scripts in tests).
            vec![
                slicer.path.display().to_string(),
                mesh.display().to_string(),
                "-o".into(),
                out_gcode.display().to_string(),
            ]
        }
    }
}

/// Build a dry-run command line for documentation (does not execute).
pub fn slice_command_preview(
    slicer: &SlicerInfo,
    mesh: &Path,
    out_gcode: &Path,
    printer_profile: Option<&str>,
) -> String {
    slice_argv(slicer, mesh, out_gcode, printer_profile).join(" ")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SliceGate {
    pub name: String,
    pub ok: bool,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SliceRequest {
    pub mesh: PathBuf,
    pub out: PathBuf,
    /// Must be exactly [`CONFIRM_SLICE`] when `execute` is true.
    pub confirm: Option<String>,
    pub execute: bool,
    /// Optional allowlist of slicer binary basenames or absolute paths.
    pub allowlist: Vec<String>,
    pub profile: Option<String>,
    /// Explicit slicer binary path (skips discovery). Used in tests.
    pub slicer_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SliceReport {
    pub ok: bool,
    pub preview: bool,
    pub executed: bool,
    pub command: String,
    pub gates: Vec<SliceGate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stdout_tail: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stderr_tail: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wall_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub out_bytes: Option<u64>,
    pub note: String,
}

/// Evaluate gates; if `execute` and all pass, run the slicer.
pub fn run_slice(slicer: &SlicerInfo, req: &SliceRequest) -> SliceReport {
    let argv = slice_argv(slicer, &req.mesh, &req.out, req.profile.as_deref());
    let command = argv.join(" ");
    let mut gates = Vec::new();

    // mesh exists
    let mesh_ok = req.mesh.is_file();
    gates.push(SliceGate {
        name: "mesh_exists".into(),
        ok: mesh_ok,
        detail: if mesh_ok {
            format!("{}", req.mesh.display())
        } else {
            format!("missing mesh {}", req.mesh.display())
        },
    });

    // allowlist (if provided)
    if req.allowlist.is_empty() {
        gates.push(SliceGate {
            name: "allowlist".into(),
            ok: true,
            detail: "empty allowlist = any discovered slicer".into(),
        });
    } else {
        let path_s = slicer.path.display().to_string();
        let base = slicer
            .path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        let ok = req.allowlist.iter().any(|a| {
            a == &path_s || a.eq_ignore_ascii_case(base) || a.eq_ignore_ascii_case(&slicer.name)
        });
        gates.push(SliceGate {
            name: "allowlist".into(),
            ok,
            detail: if ok {
                format!("slicer {path_s} allowed")
            } else {
                format!("slicer {path_s} not in allowlist {:?}", req.allowlist)
            },
        });
    }

    if req.execute {
        let conf_ok = req.confirm.as_deref() == Some(CONFIRM_SLICE);
        gates.push(SliceGate {
            name: "confirm".into(),
            ok: conf_ok,
            detail: if conf_ok {
                format!("confirm={CONFIRM_SLICE}")
            } else {
                format!("need --confirm {CONFIRM_SLICE} (got {:?})", req.confirm)
            },
        });
    } else {
        gates.push(SliceGate {
            name: "confirm".into(),
            ok: true,
            detail: "preview mode — confirm not required".into(),
        });
    }

    let gates_ok = gates.iter().all(|g| g.ok);

    if !req.execute {
        return SliceReport {
            ok: gates_ok && mesh_ok,
            preview: true,
            executed: false,
            command,
            gates,
            exit_code: None,
            stdout_tail: None,
            stderr_tail: None,
            wall_ms: None,
            out_bytes: None,
            note: "preview only — pass --execute --confirm SLICE to run host slicer".into(),
        };
    }

    if !gates_ok {
        return SliceReport {
            ok: false,
            preview: false,
            executed: false,
            command,
            gates,
            exit_code: None,
            stdout_tail: None,
            stderr_tail: None,
            wall_ms: None,
            out_bytes: None,
            note: "execute blocked by gates — no process spawned".into(),
        };
    }

    // Execute
    let started = Instant::now();
    let mut cmd = Command::new(&argv[0]);
    if argv.len() > 1 {
        cmd.args(&argv[1..]);
    }
    let output = cmd.output();
    let wall_ms = started.elapsed().as_millis() as u64;

    match output {
        Ok(out) => {
            let code = out.status.code().unwrap_or(-1);
            let stdout_tail = tail_str(&String::from_utf8_lossy(&out.stdout), 400);
            let stderr_tail = tail_str(&String::from_utf8_lossy(&out.stderr), 400);
            let out_bytes = std::fs::metadata(&req.out).ok().map(|m| m.len());
            let ok = out.status.success() && req.out.is_file();
            SliceReport {
                ok,
                preview: false,
                executed: true,
                command,
                gates,
                exit_code: Some(code),
                stdout_tail: Some(stdout_tail),
                stderr_tail: Some(stderr_tail),
                wall_ms: Some(wall_ms),
                out_bytes,
                note: if ok {
                    "slicer executed; gcode written".into()
                } else {
                    "slicer ran but failed or missing output — see exit_code/stderr".into()
                },
            }
        }
        Err(e) => SliceReport {
            ok: false,
            preview: false,
            executed: false,
            command,
            gates,
            exit_code: None,
            stdout_tail: None,
            stderr_tail: Some(e.to_string()),
            wall_ms: Some(wall_ms),
            out_bytes: None,
            note: "failed to spawn slicer".into(),
        },
    }
}

fn tail_str(s: &str, max: usize) -> String {
    let t = s.trim();
    if t.len() <= max {
        t.to_string()
    } else {
        format!("…{}", &t[t.len() - max..])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn preview_without_execute() {
        let dir = tempfile::tempdir().unwrap();
        let mesh = dir.path().join("a.stl");
        std::fs::write(&mesh, b"solid x\nendsolid x\n").unwrap();
        let out = dir.path().join("a.gcode");
        let slicer = SlicerInfo {
            kind: SlicerKind::Unknown,
            name: "stub".into(),
            path: PathBuf::from("/bin/true"),
            version: None,
        };
        let rep = run_slice(
            &slicer,
            &SliceRequest {
                mesh,
                out,
                confirm: None,
                execute: false,
                allowlist: vec![],
                profile: None,
                slicer_path: None,
            },
        );
        assert!(rep.preview);
        assert!(!rep.executed);
        assert!(rep.ok);
    }

    #[test]
    fn execute_without_confirm_blocked() {
        let dir = tempfile::tempdir().unwrap();
        let mesh = dir.path().join("a.stl");
        std::fs::write(&mesh, b"solid\n").unwrap();
        let slicer = SlicerInfo {
            kind: SlicerKind::Unknown,
            name: "stub".into(),
            path: PathBuf::from("/bin/true"),
            version: None,
        };
        let rep = run_slice(
            &slicer,
            &SliceRequest {
                mesh,
                out: dir.path().join("a.gcode"),
                confirm: None,
                execute: true,
                allowlist: vec![],
                profile: None,
                slicer_path: None,
            },
        );
        assert!(!rep.ok);
        assert!(!rep.executed);
        assert!(rep.gates.iter().any(|g| g.name == "confirm" && !g.ok));
    }

    #[test]
    #[cfg(unix)]
    fn execute_with_confirm_runs_stub() {
        let dir = tempfile::tempdir().unwrap();
        let mesh = dir.path().join("a.stl");
        std::fs::write(&mesh, b"solid\n").unwrap();
        let out = dir.path().join("a.gcode");
        let stub = dir.path().join("fake-slicer.sh");
        let mut f = std::fs::File::create(&stub).unwrap();
        writeln!(f, "#!/bin/sh").unwrap();
        // Unknown kind argv: [bin, mesh, -o, out]
        writeln!(f, "out=\"$4\"").unwrap();
        writeln!(f, "if [ -z \"$out\" ]; then out=\"$3\"; fi").unwrap();
        writeln!(f, "echo '; stub gcode' > \"$out\"").unwrap();
        drop(f);
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&stub).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&stub, perms).unwrap();
        }
        let slicer = SlicerInfo {
            kind: SlicerKind::Unknown,
            name: "fake-slicer".into(),
            path: stub,
            version: Some("test".into()),
        };
        let rep = run_slice(
            &slicer,
            &SliceRequest {
                mesh,
                out: out.clone(),
                confirm: Some(CONFIRM_SLICE.into()),
                execute: true,
                allowlist: vec!["fake-slicer.sh".into()],
                profile: None,
                slicer_path: None,
            },
        );
        assert!(rep.executed, "{rep:?}");
        assert!(rep.ok, "{rep:?}");
        assert!(out.is_file());
        assert!(std::fs::read_to_string(out).unwrap().contains("stub"));
    }

    #[test]
    #[cfg(windows)]
    fn execute_with_confirm_runs_stub() {
        let dir = tempfile::tempdir().unwrap();
        let mesh = dir.path().join("a.stl");
        std::fs::write(&mesh, b"solid\n").unwrap();
        let out = dir.path().join("a.gcode");
        let stub = dir.path().join("fake-slicer.bat");
        // %1=mesh %2=-o %3=out when invoked as bat with mesh -o out
        std::fs::write(&stub, "@echo off\r\necho ; stub gcode > \"%~3\"\r\n").unwrap();
        let slicer = SlicerInfo {
            kind: SlicerKind::Unknown,
            name: "fake-slicer".into(),
            path: stub,
            version: Some("test".into()),
        };
        let rep = run_slice(
            &slicer,
            &SliceRequest {
                mesh,
                out: out.clone(),
                confirm: Some(CONFIRM_SLICE.into()),
                execute: true,
                allowlist: vec!["fake-slicer.bat".into()],
                profile: None,
                slicer_path: None,
            },
        );
        assert!(rep.executed, "{rep:?}");
        assert!(rep.ok, "{rep:?}");
        assert!(out.is_file());
    }
}
