//! `cadrion snapshot`

use std::fs;
use std::path::PathBuf;

use cadrion_lang::{evaluate, EvalOptions};
use cadrion_render::{mesh_from_ir, write_snapshot_packet, SnapshotOptions, ViewName};
use serde_json::json;

use crate::build_cmd::parse_sets;
use crate::cli::{Cli, SnapshotArgs};
use crate::output::{emit, ExitCode};

pub fn run(cli: &Cli, args: &SnapshotArgs) -> ExitCode {
    let target = &args.target;
    if !target.exists() {
        let v = json!({"ok": false, "diagnostics": [{"code": "CADRION-E-IO", "message": format!("not found: {}", target.display())}]});
        emit(cli.json, &v, false);
        return ExitCode::Io;
    }

    let source = match fs::read_to_string(target) {
        Ok(s) => s,
        Err(e) => {
            let v = json!({"ok": false, "diagnostics": [{"code": "CADRION-E-IO", "message": e.to_string()}]});
            emit(cli.json, &v, false);
            return ExitCode::Io;
        }
    };

    let overrides = match parse_sets(&args.set) {
        Ok(m) => m,
        Err(msg) => {
            let v =
                json!({"ok": false, "diagnostics": [{"code": "CADRION-E-USAGE", "message": msg}]});
            emit(cli.json, &v, false);
            return ExitCode::Usage;
        }
    };

    let name = target
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("part.cad.star")
        .to_string();
    let mut opts = EvalOptions::new(name);
    opts.overrides = overrides;
    let eval = evaluate(&source, &opts);
    if !eval.ok {
        let v = json!({"ok": false, "diagnostics": eval.diagnostics});
        emit(cli.json, &v, false);
        return ExitCode::Eval;
    }
    let ir = eval.ir.unwrap();

    let (mesh, notes) = match mesh_from_ir(&ir) {
        Ok(m) => m,
        Err(e) => {
            let v =
                json!({"ok": false, "diagnostics": [{"code": "CADRION-E-RENDER", "message": e}]});
            emit(cli.json, &v, false);
            return ExitCode::Internal;
        }
    };

    let out = args.out.clone().unwrap_or_else(|| snap_dir_for(target));
    let views = ViewName::parse_list(&args.views);
    if views.is_empty() {
        let v = json!({"ok": false, "diagnostics": [{"code": "CADRION-E-USAGE", "message": "no valid --views"}]});
        emit(cli.json, &v, false);
        return ExitCode::Usage;
    }

    let snap_opts = SnapshotOptions {
        views,
        width: args.size,
        height: args.size,
        gif: !args.no_gif,
        gif_frames: args.gif_frames,
        gif_delay_cs: 6,
        notes,
    };

    match write_snapshot_packet(&mesh, &out, &snap_opts) {
        Ok(res) => {
            let body = json!({
                "ok": true,
                "out_dir": res.manifest.out_dir,
                "views": res.manifest.views,
                "gif": res.manifest.gif,
                "triangles": res.manifest.triangles,
                "preview_mesh": res.manifest.preview_mesh,
                "notes": res.manifest.notes,
                "wall_ms": res.manifest.wall_ms,
                "renderer": res.manifest.renderer,
                "meta": {"source": target},
            });
            emit(cli.json, &body, true);
            ExitCode::Ok
        }
        Err(e) => {
            let v =
                json!({"ok": false, "diagnostics": [{"code": "CADRION-E-RENDER", "message": e}]});
            emit(cli.json, &v, false);
            ExitCode::Internal
        }
    }
}

fn snap_dir_for(target: &std::path::Path) -> PathBuf {
    let name = target
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("part");
    let stem = name
        .strip_suffix(".cad.star")
        .or_else(|| name.strip_suffix(".star"))
        .unwrap_or(name);
    target.with_file_name(format!("{stem}.snap"))
}
