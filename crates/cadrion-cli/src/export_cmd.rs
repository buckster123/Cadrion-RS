//! `cadrion export`

use std::fs;
use std::path::PathBuf;

use cadrion_kernel::{GeomKernel, Mesh, StepWriteOpts, TessTol};
use cadrion_lang::{evaluate, execute_ir, EvalOptions};
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

fn write_stl_ascii(path: &std::path::Path, mesh: &Mesh) -> std::io::Result<()> {
    use std::io::Write;
    let mut f = fs::File::create(path)?;
    writeln!(f, "solid cadrion")?;
    let p = &mesh.positions;
    for tri in mesh.indices.chunks_exact(3) {
        let i0 = tri[0] as usize * 3;
        let i1 = tri[1] as usize * 3;
        let i2 = tri[2] as usize * 3;
        let ax = p[i0];
        let ay = p[i0 + 1];
        let az = p[i0 + 2];
        let bx = p[i1];
        let by = p[i1 + 1];
        let bz = p[i1 + 2];
        let cx = p[i2];
        let cy = p[i2 + 1];
        let cz = p[i2 + 2];
        // normal
        let ux = bx - ax;
        let uy = by - ay;
        let uz = bz - az;
        let vx = cx - ax;
        let vy = cy - ay;
        let vz = cz - az;
        let nx = uy * vz - uz * vy;
        let ny = uz * vx - ux * vz;
        let nz = ux * vy - uy * vx;
        writeln!(f, "  facet normal {nx} {ny} {nz}")?;
        writeln!(f, "    outer loop")?;
        writeln!(f, "      vertex {ax} {ay} {az}")?;
        writeln!(f, "      vertex {bx} {by} {bz}")?;
        writeln!(f, "      vertex {cx} {cy} {cz}")?;
        writeln!(f, "    endloop")?;
        writeln!(f, "  endfacet")?;
    }
    writeln!(f, "endsolid cadrion")?;
    Ok(())
}

fn write_gltf_json(path: &std::path::Path, mesh: &Mesh) -> std::io::Result<PathBuf> {
    // Force .gltf extension for honesty when binary glb not implemented.
    let out = if path.extension().and_then(|e| e.to_str()) == Some("glb") {
        path.with_extension("gltf")
    } else {
        path.to_path_buf()
    };
    // interleaved f32 positions
    let mut bin = Vec::with_capacity(mesh.positions.len() * 4);
    for f in &mesh.positions {
        bin.extend_from_slice(&f.to_le_bytes());
    }
    let b64 = base64_encode(&bin);
    let n_verts = mesh.positions.len() / 3;
    let indices: Vec<u32> = mesh.indices.clone();
    let mut idx_bin = Vec::with_capacity(indices.len() * 4);
    for i in &indices {
        idx_bin.extend_from_slice(&i.to_le_bytes());
    }
    let idx_b64 = base64_encode(&idx_bin);

    // Single buffer with positions then indices is more complex; two data URIs via two buffers.
    let gltf = json!({
        "asset": {"version": "2.0", "generator": "cadrion-cli"},
        "buffers": [
            {"byteLength": bin.len(), "uri": format!("data:application/octet-stream;base64,{b64}")},
            {"byteLength": idx_bin.len(), "uri": format!("data:application/octet-stream;base64,{idx_b64}")},
        ],
        "bufferViews": [
            {"buffer": 0, "byteOffset": 0, "byteLength": bin.len(), "target": 34962},
            {"buffer": 1, "byteOffset": 0, "byteLength": idx_bin.len(), "target": 34963},
        ],
        "accessors": [
            {
                "bufferView": 0,
                "componentType": 5126,
                "count": n_verts,
                "type": "VEC3",
            },
            {
                "bufferView": 1,
                "componentType": 5125,
                "count": indices.len(),
                "type": "SCALAR",
            }
        ],
        "meshes": [{
            "primitives": [{
                "attributes": {"POSITION": 0},
                "indices": 1,
                "mode": 4
            }]
        }],
        "nodes": [{"mesh": 0}],
        "scenes": [{"nodes": [0]}],
        "scene": 0
    });
    fs::write(&out, serde_json::to_vec_pretty(&gltf).unwrap())?;
    Ok(out)
}

fn base64_encode(data: &[u8]) -> String {
    // Minimal base64 without extra dep
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    let mut i = 0;
    while i + 3 <= data.len() {
        let n = ((data[i] as u32) << 16) | ((data[i + 1] as u32) << 8) | (data[i + 2] as u32);
        out.push(T[((n >> 18) & 63) as usize] as char);
        out.push(T[((n >> 12) & 63) as usize] as char);
        out.push(T[((n >> 6) & 63) as usize] as char);
        out.push(T[(n & 63) as usize] as char);
        i += 3;
    }
    match data.len() - i {
        1 => {
            let n = (data[i] as u32) << 16;
            out.push(T[((n >> 18) & 63) as usize] as char);
            out.push(T[((n >> 12) & 63) as usize] as char);
            out.push('=');
            out.push('=');
        }
        2 => {
            let n = ((data[i] as u32) << 16) | ((data[i + 1] as u32) << 8);
            out.push(T[((n >> 18) & 63) as usize] as char);
            out.push(T[((n >> 12) & 63) as usize] as char);
            out.push(T[((n >> 6) & 63) as usize] as char);
            out.push('=');
        }
        _ => {}
    }
    out
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
