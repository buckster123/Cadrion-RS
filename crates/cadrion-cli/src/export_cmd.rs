//! `cadrion export`

use std::fs;
use std::path::PathBuf;

use cadrion_kernel::{GeomKernel, StepWriteOpts, TessTol};
use cadrion_lang::{evaluate, execute_ir, EvalOptions};
use cadrion_render::{write_gltf_json, write_stl_ascii};
use serde_json::json;

use crate::build_cmd::parse_sets;
use crate::cli::{Cli, ExportArgs, ExportFormat};
use crate::kernel_pick::open_kernel;
use crate::output::{emit, ExitCode};

pub fn run(cli: &Cli, args: &ExportArgs) -> ExitCode {
    let mut kernel = match open_kernel(cli.kernel) {
        Ok(k) => k,
        Err((c, v)) => {
            emit(cli.json, &v, false);
            return c;
        }
    };

    let shape = match load_shape(args, kernel.as_mut()) {
        Ok(s) => s,
        Err((c, v)) => {
            emit(cli.json, &v, false);
            return c;
        }
    };

    let out = args.output.clone().unwrap_or_else(|| {
        let stem = strip_export_stem(&args.target);
        match args.format {
            ExportFormat::Step => PathBuf::from(format!("{}.step", stem.display())),
            ExportFormat::Stl => PathBuf::from(format!("{}.stl", stem.display())),
            ExportFormat::Glb => PathBuf::from(format!("{}.glb", stem.display())),
        }
    });

    match args.format {
        ExportFormat::Step => {
            match kernel
                .as_mut()
                .write_step(shape, &out, &StepWriteOpts::default())
            {
                Ok(()) => {
                    let body = json!({"ok": true, "path": out, "format": "step"});
                    emit(cli.json, &body, true);
                    ExitCode::Ok
                }
                Err(e) => {
                    let body = json!({"ok": false, "diagnostics": [{"code": e.code(), "message": e.to_string()}]});
                    emit(cli.json, &body, false);
                    ExitCode::Kernel
                }
            }
        }
        ExportFormat::Stl => match kernel.as_mut().tessellate(shape, TessTol::default()) {
            Ok(mesh) => match write_stl_ascii(&out, &mesh) {
                Ok(()) => {
                    let body = json!({"ok": true, "path": out, "format": "stl", "triangles": mesh.triangle_count()});
                    emit(cli.json, &body, true);
                    ExitCode::Ok
                }
                Err(e) => {
                    let body = json!({"ok": false, "diagnostics": [{"code": "CADRION-E-IO", "message": e.to_string()}]});
                    emit(cli.json, &body, false);
                    ExitCode::Io
                }
            },
            Err(e) => {
                let body = json!({
                    "ok": false,
                    "diagnostics": [{
                        "code": e.code(),
                        "message": e.to_string(),
                        "hint": "STL needs tessellation; use --kernel occt (features occt)"
                    }]
                });
                emit(cli.json, &body, false);
                ExitCode::Kernel
            }
        },
        ExportFormat::Glb => {
            // Minimal glTF 2.0 JSON (no Draco) with positions only — not a full binary GLB container.
            // Honest: we write `.gltf` companion if user asked glb without binary pack, or fail.
            // For S5: write JSON glTF with embedded buffer base64 as .gltf; if extension is .glb warn.
            match kernel.as_mut().tessellate(shape, TessTol::default()) {
                Ok(mesh) => match write_gltf_json(&out, &mesh) {
                    Ok(path) => {
                        let body = json!({
                            "ok": true,
                            "path": path,
                            "format": "gltf",
                            "note": "S5 writes JSON glTF (embedded buffer); binary .glb container is follow-up",
                            "triangles": mesh.triangle_count(),
                        });
                        emit(cli.json, &body, true);
                        ExitCode::Ok
                    }
                    Err(e) => {
                        let body = json!({"ok": false, "diagnostics": [{"code": "CADRION-E-IO", "message": e.to_string()}]});
                        emit(cli.json, &body, false);
                        ExitCode::Io
                    }
                },
                Err(e) => {
                    let body = json!({
                        "ok": false,
                        "diagnostics": [{
                            "code": e.code(),
                            "message": e.to_string(),
                            "hint": "GLB/glTF needs tessellation; use --kernel occt"
                        }]
                    });
                    emit(cli.json, &body, false);
                    ExitCode::Kernel
                }
            }
        }
    }
}

fn load_shape(
    args: &ExportArgs,
    kernel: &mut dyn GeomKernel,
) -> Result<cadrion_kernel::ShapeId, (ExitCode, serde_json::Value)> {
    let ext = args
        .target
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    if ext == "step" || ext == "stp" {
        return kernel
            .read_step(&args.target, &Default::default())
            .map_err(|e| {
                (
                    ExitCode::Kernel,
                    json!({"ok": false, "diagnostics": [{"code": e.code(), "message": e.to_string()}]}),
                )
            });
    }

    let source = fs::read_to_string(&args.target).map_err(|e| {
        (
            ExitCode::Io,
            json!({"ok": false, "diagnostics": [{"code": "CADRION-E-IO", "message": e.to_string()}]}),
        )
    })?;
    let overrides = parse_sets(&args.set).map_err(|m| {
        (
            ExitCode::Usage,
            json!({"ok": false, "diagnostics": [{"code": "CADRION-E-USAGE", "message": m}]}),
        )
    })?;
    let name = args
        .target
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("model.cad.star")
        .to_string();
    let mut opts = EvalOptions::new(name);
    opts.overrides = overrides;
    let eval = evaluate(&source, &opts);
    if !eval.ok {
        return Err((
            ExitCode::Eval,
            json!({"ok": false, "diagnostics": eval.diagnostics}),
        ));
    }
    let ir = eval.ir.unwrap();
    execute_ir(kernel, &ir).map_err(|e| {
        (
            ExitCode::Kernel,
            json!({"ok": false, "diagnostics": [{"code": e.code(), "message": e.to_string()}]}),
        )
    })
}

fn strip_export_stem(path: &std::path::Path) -> PathBuf {
    let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("model");
    let stem = if let Some(s) = name.strip_suffix(".cad.star") {
        s
    } else if let Some(s) = name.strip_suffix(".star") {
        s
    } else {
        path.file_stem().and_then(|s| s.to_str()).unwrap_or(name)
    };
    path.with_file_name(stem)
}
