//! MCP tool implementations (thin wrappers over lang/inspect/render + fs).

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use cadrion_fab::{
    apply_override, check_dfm, check_gcode, hex_sha256, load_override_json, load_profile_json,
    resolve_bundled_profile, FlatPart, PrinterVolume,
};
use cadrion_inspect::{
    align_refs, build_drawing_packet, diff_snapshots, frame_of, inspect_refs, measure, AlignExpect,
    DimSpec, MeasureKind, MeasureRequest,
};
use cadrion_kernel::{GeomKernel, MockKernel, StepWriteOpts};
use cadrion_lang::{evaluate, execute_ir, EvalOptions};
use cadrion_parts::{
    upsert_lock_entry, validate_assembly, verify_lock_entry, AssemblySpec, LocalFsProvider,
    PartProvider, PartsLockEntry,
};
use cadrion_render::{
    mesh_from_ir, write_gltf_json, write_snapshot_packet, write_stl_ascii, SnapshotOptions,
    ViewName,
};
use cadrion_robot::{
    emit_and_validate, parse_urdf_xml, srdf_from_robot, validate_sdf_xml, validate_urdf_xml,
    write_sdf, write_srdf, RobotSpec, ValidationReport,
};
use cadrion_sdf::{grid_for_prim, sample_analytic, write_nrrd, write_raw, SdfPrim};
use serde_json::{json, Value};

#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("{0}")]
    Msg(String),
}

impl ToolError {
    pub(crate) fn msg(s: impl Into<String>) -> Self {
        Self::Msg(s.into())
    }
}

/// Short tool definitions for tools/list (keep under token budget).
pub fn tool_defs() -> Value {
    json!([
        {
            "name": "build",
            "description": "Evaluate .cad.star → IR (mock). Writes companion .ir.json. Returns facts JSON.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Path to .cad.star"},
                    "set": {"type": "object", "additionalProperties": {"type": "number"}, "description": "param overrides"}
                },
                "required": ["path"]
            }
        },
        {
            "name": "write_source",
            "description": "Write a .cad.star file (creates parents). Stdio: OFF by default (CADRION_MCP_WRITE_SOURCE=1). HTTP: ON by default.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {"type": "string"},
                    "content": {"type": "string"}
                },
                "required": ["path", "content"]
            }
        },
        {
            "name": "read_source",
            "description": "Read a text source file.",
            "inputSchema": {
                "type": "object",
                "properties": {"path": {"type": "string"}},
                "required": ["path"]
            }
        },
        {
            "name": "inspect_refs",
            "description": "List stable #o… selectors (+ optional facts) from .cad.star IR topology.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {"type": "string"},
                    "facts": {"type": "boolean", "default": true}
                },
                "required": ["path"]
            }
        },
        {
            "name": "measure",
            "description": "Measure distance|angle|diameter|thickness between selectors.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {"type": "string"},
                    "a": {"type": "string"},
                    "b": {"type": "string"},
                    "kind": {"type": "string", "enum": ["distance","angle","diameter","thickness"]}
                },
                "required": ["path", "a", "kind"]
            }
        },
        {
            "name": "snapshot",
            "description": "Render multi-view PNG + orbit GIF packet. Returns paths; PNG/GIF as base64 image content when include_images=true.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {"type": "string"},
                    "views": {"type": "string", "default": "iso,front,top,right"},
                    "size": {"type": "integer", "default": 256},
                    "include_images": {"type": "boolean", "default": true}
                },
                "required": ["path"]
            }
        },
        {
            "name": "inspect_dims",
            "description": "PMI alpha: linear dim facts → drawing packet JSON (not a drafting package). Auto opposite faces or optional dims array.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": ".cad.star"},
                    "out": {"type": "string", "description": "optional output .drawing.json path"},
                    "dims": {
                        "type": "array",
                        "description": "optional DimSpec list {a,b?,kind,label?}",
                        "items": {"type": "object"}
                    }
                },
                "required": ["path"]
            }
        },
        {
            "name": "assembly_validate",
            "description": "Validate assembly .assy.json joints/components (fail-closed limits).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "path to .assy.json"}
                },
                "required": ["path"]
            }
        },
        {
            "name": "sdf_sample",
            "description": "Secondary SDF sample (analytic box/cyl → raw+NRRD). Never a modeling path.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "prim": {"type": "string", "enum": ["box", "cylinder"]},
                    "a": {"type": "number", "description": "box dx or cylinder r"},
                    "b": {"type": "number", "description": "box dy or cylinder h"},
                    "c": {"type": "number", "description": "box dz (required for box)"},
                    "res": {"type": "integer", "default": 24},
                    "pad": {"type": "number", "default": 2.0},
                    "out": {"type": "string", "description": "output dir"},
                    "stem": {"type": "string"}
                },
                "required": ["prim", "a", "b"]
            }
        },
        {
            "name": "align_check",
            "description": "Align two selectors (coplanar|coaxial|distance).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {"type": "string"},
                    "a": {"type": "string"},
                    "b": {"type": "string"},
                    "expect": {"type": "string", "enum": ["coplanar","coaxial","distance"], "default": "distance"},
                    "distance": {"type": "number"},
                    "tol": {"type": "number", "default": 0.1},
                    "tol_deg": {"type": "number", "default": 1.0}
                },
                "required": ["path", "a", "b"]
            }
        },
        {
            "name": "frame",
            "description": "Local frame (origin + axes) for a selector.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {"type": "string"},
                    "selector": {"type": "string"}
                },
                "required": ["path", "selector"]
            }
        },
        {
            "name": "diff",
            "description": "Diff two .cad.star builds (volume/faces + selector remap).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "old": {"type": "string"},
                    "new": {"type": "string"}
                },
                "required": ["old", "new"]
            }
        },
        {
            "name": "export",
            "description": "Export stl/gltf preview mesh, or STEP when the kernel writes it. Mock STEP is Unsupported.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {"type": "string"},
                    "format": {"type": "string", "enum": ["step", "stl", "gltf", "glb"]},
                    "out": {"type": "string"}
                },
                "required": ["path", "format"]
            }
        },
        {
            "name": "fab_check",
            "description": "DFM preflight on a FlatPart JSON + bundled profile. No printer start.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "FlatPart JSON"},
                    "profile": {"type": "string", "default": "sendcutsend.laser"},
                    "profile_file": {"type": "string"},
                    "override_file": {"type": "string"}
                },
                "required": ["path"]
            }
        },
        {
            "name": "engine",
            "description": "Kernel inventory. install is fail-closed (no tarball). See cadrion://doc/schema.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "action": {"type": "string", "enum": ["info", "install"], "default": "info"},
                    "backend": {"type": "string", "enum": ["occt", "truck-brep"]}
                }
            }
        },
        {
            "name": "schema",
            "description": "Live MCP/error surface. cli/api faces: cadrion schema or /v1/schema.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "face": {"type": "string", "enum": ["mcp", "errors"], "default": "mcp"}
                }
            }
        },
        {
            "name": "robot",
            "description": "URDF gen/validate from .robot.json. Inertials must be in the spec.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "op": {"type": "string", "enum": ["gen", "validate"]},
                    "path": {"type": "string"},
                    "out": {"type": "string"},
                    "srdf": {"type": "boolean", "default": true},
                    "sdf": {"type": "boolean", "default": true}
                },
                "required": ["op", "path"]
            }
        },
        {
            "name": "parts",
            "description": "Local STEP catalog: search/fetch/lock. Not a storefront. See cadrion://doc/status.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "op": {"type": "string", "enum": ["search", "fetch", "lock"]},
                    "query": {"type": "string"},
                    "id": {"type": "string"},
                    "parts_root": {"type": "string"},
                    "lock": {"type": "string"},
                    "key": {"type": "string"},
                    "project": {"type": "string"}
                },
                "required": ["op"]
            }
        },
        {
            "name": "viewer_open",
            "description": "Loopback viewer links (--once). Does not start a server or claim wgpu CAD.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {"type": "string"},
                    "paths": {"type": "array", "items": {"type": "string"}},
                    "once": {"type": "boolean", "default": true}
                }
            }
        },
        {
            "name": "gcode_check",
            "description": "Static G-code bbox/temp/flavor. Not a slicer and not a print.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {"type": "string"},
                    "bed_x": {"type": "number"},
                    "bed_y": {"type": "number"},
                    "bed_z": {"type": "number"},
                    "max_hotend": {"type": "number"},
                    "max_bed": {"type": "number"}
                },
                "required": ["path"]
            }
        }
    ])
}

pub fn call_tool(name: &str, args: &Value) -> Result<Value, ToolError> {
    match name {
        "build" => tool_build(args),
        "write_source" => tool_write_source(args),
        "read_source" => tool_read_source(args),
        "inspect_refs" => tool_inspect_refs(args),
        "measure" => tool_measure(args),
        "snapshot" => tool_snapshot(args),
        "inspect_dims" => tool_inspect_dims(args),
        "assembly_validate" => tool_assembly_validate(args),
        "sdf_sample" => tool_sdf_sample(args),
        "align_check" => tool_align_check(args),
        "frame" => tool_frame(args),
        "diff" => tool_diff(args),
        "export" => tool_export(args),
        "fab_check" => tool_fab_check(args),
        "engine" => tool_engine(args),
        "schema" => tool_schema(args),
        "robot" => tool_robot(args),
        "parts" => tool_parts(args),
        "viewer_open" => tool_viewer_open(args),
        "gcode_check" => tool_gcode_check(args),
        other => Err(ToolError::msg(format!("unknown tool: {other}"))),
    }
}

fn str_arg<'a>(args: &'a Value, key: &str) -> Result<&'a str, ToolError> {
    args.get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| ToolError::msg(format!("missing string arg '{key}'")))
}

fn tool_write_source(args: &Value) -> Result<Value, ToolError> {
    let pol = crate::policy::policy();
    if !pol.write_source {
        return Err(ToolError::msg(format!(
            "write_source disabled on {} transport (set CADRION_MCP_WRITE_SOURCE=1 to enable; see cadrion://doc/write-source-policy)",
            pol.transport
        )));
    }
    let path = PathBuf::from(str_arg(args, "path")?);
    let content = str_arg(args, "content")?;
    if path.is_dir() {
        return Err(ToolError::msg("path is a directory"));
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| ToolError::msg(e.to_string()))?;
    }
    fs::write(&path, content).map_err(|e| ToolError::msg(e.to_string()))?;
    Ok(json!({
        "content": [{"type": "text", "text": serde_json::to_string_pretty(&json!({
            "ok": true,
            "path": path,
            "bytes": content.len(),
            "transport": pol.transport,
        })).unwrap()}]
    }))
}

fn tool_read_source(args: &Value) -> Result<Value, ToolError> {
    let path = PathBuf::from(str_arg(args, "path")?);
    let text = fs::read_to_string(&path).map_err(|e| ToolError::msg(e.to_string()))?;
    Ok(json!({
        "content": [{"type": "text", "text": text}]
    }))
}

fn eval_path(path: &Path, set: Option<&Value>) -> Result<cadrion_lang::FeatureIr, ToolError> {
    let source = fs::read_to_string(path).map_err(|e| ToolError::msg(e.to_string()))?;
    let mut opts = EvalOptions::new(
        path.file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("part.cad.star"),
    );
    if let Some(Value::Object(map)) = set {
        let mut o = BTreeMap::new();
        for (k, v) in map {
            let n = v
                .as_f64()
                .ok_or_else(|| ToolError::msg(format!("set.{k} must be number")))?;
            o.insert(k.clone(), n);
        }
        opts.overrides = o;
    }
    let r = evaluate(&source, &opts);
    if !r.ok {
        return Err(ToolError::msg(format!("eval failed: {:?}", r.diagnostics)));
    }
    Ok(r.ir.unwrap())
}

fn tool_build(args: &Value) -> Result<Value, ToolError> {
    let path = PathBuf::from(str_arg(args, "path")?);
    if path.is_dir() {
        return Err(ToolError::msg("directory builds refused"));
    }
    let ir = eval_path(&path, args.get("set"))?;
    let ir_path = {
        let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("part");
        let stem = name
            .strip_suffix(".cad.star")
            .or_else(|| name.strip_suffix(".star"))
            .unwrap_or(name);
        path.with_file_name(format!("{stem}.ir.json"))
    };
    let ir_json = serde_json::to_string_pretty(&ir).map_err(|e| ToolError::msg(e.to_string()))?;
    fs::write(&ir_path, &ir_json).map_err(|e| ToolError::msg(e.to_string()))?;

    // execute on mock for facts
    use cadrion_kernel::{GeomKernel, MockKernel};
    use cadrion_lang::execute_ir;
    let mut k = MockKernel::new();
    let shape = execute_ir(&mut k, &ir).map_err(|e| ToolError::msg(e.to_string()))?;
    let facts = k.facts(shape).map_err(|e| ToolError::msg(e.to_string()))?;

    let payload = json!({
        "ok": true,
        "ir_path": ir_path,
        "label": ir.label,
        "params": ir.params,
        "node_count": ir.node_count(),
        "facts": facts,
        "kernel": "mock",
        "note": "STEP requires --kernel occt CLI; MCP build is IR+facts on mock"
    });
    Ok(json!({
        "content": [{"type": "text", "text": serde_json::to_string_pretty(&payload).unwrap()}]
    }))
}

fn tool_inspect_refs(args: &Value) -> Result<Value, ToolError> {
    let path = PathBuf::from(str_arg(args, "path")?);
    let facts = args.get("facts").and_then(|v| v.as_bool()).unwrap_or(true);
    let ir = eval_path(&path, None)?;
    let snap = topo_from_ir(&ir)?;
    let report = inspect_refs(&snap, facts);
    Ok(json!({
        "content": [{"type": "text", "text": serde_json::to_string_pretty(&report).unwrap()}]
    }))
}

fn tool_measure(args: &Value) -> Result<Value, ToolError> {
    let path = PathBuf::from(str_arg(args, "path")?);
    let a = str_arg(args, "a")?.to_string();
    let b = args
        .get("b")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let kind = match str_arg(args, "kind")? {
        "distance" => MeasureKind::Distance,
        "angle" => MeasureKind::Angle,
        "diameter" => MeasureKind::Diameter,
        "thickness" => MeasureKind::Thickness,
        other => return Err(ToolError::msg(format!("bad kind {other}"))),
    };
    let ir = eval_path(&path, None)?;
    let snap = topo_from_ir(&ir)?;
    let r = measure(&snap, &MeasureRequest { a, b, kind })
        .map_err(|e| ToolError::msg(e.to_string()))?;
    Ok(json!({
        "content": [{"type": "text", "text": serde_json::to_string_pretty(&r).unwrap()}]
    }))
}

fn tool_snapshot(args: &Value) -> Result<Value, ToolError> {
    let path = PathBuf::from(str_arg(args, "path")?);
    let views_s = args
        .get("views")
        .and_then(|v| v.as_str())
        .unwrap_or("iso,front,top,right");
    let size = args.get("size").and_then(|v| v.as_u64()).unwrap_or(256) as u32;
    let include = args
        .get("include_images")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let ir = eval_path(&path, None)?;
    let (mesh, notes) = mesh_from_ir(&ir).map_err(ToolError::msg)?;
    let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("part");
    let stem = name
        .strip_suffix(".cad.star")
        .or_else(|| name.strip_suffix(".star"))
        .unwrap_or(name);
    let out = path.with_file_name(format!("{stem}.snap"));
    let opts = SnapshotOptions {
        views: ViewName::parse_list(views_s),
        width: size,
        height: size,
        gif: true,
        gif_frames: 12,
        gif_delay_cs: 6,
        notes,
    };
    let res = write_snapshot_packet(&mesh, &out, &opts).map_err(ToolError::msg)?;

    let mut content = vec![json!({
        "type": "text",
        "text": serde_json::to_string_pretty(&json!({
            "ok": true,
            "out_dir": res.manifest.out_dir,
            "views": res.manifest.views,
            "gif": res.manifest.gif,
            "notes": res.manifest.notes,
            "preview_mesh": true
        })).unwrap()
    })];

    if include {
        for v in &res.manifest.views {
            if v.name == "iso" || v.name == "front" {
                if let Ok(bytes) = fs::read(&v.path) {
                    use base64::Engine;
                    let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
                    content.push(json!({
                        "type": "image",
                        "data": b64,
                        "mimeType": "image/png"
                    }));
                }
            }
        }
    }

    Ok(json!({ "content": content }))
}

fn tool_inspect_dims(args: &Value) -> Result<Value, ToolError> {
    let path = PathBuf::from(str_arg(args, "path")?);
    let ir = eval_path(&path, None)?;
    let snap = topo_from_ir(&ir)?;
    let mut specs: Vec<DimSpec> = Vec::new();
    if let Some(arr) = args.get("dims").and_then(|v| v.as_array()) {
        for (i, v) in arr.iter().enumerate() {
            let s: DimSpec = serde_json::from_value(v.clone())
                .map_err(|e| ToolError::msg(format!("dims[{i}]: {e}")))?;
            specs.push(s);
        }
    }
    let source = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("part")
        .to_string();
    let packet = build_drawing_packet(&snap, &source, "ir-analytic-mcp", &specs);
    let out = args
        .get("out")
        .and_then(|v| v.as_str())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("part");
            let stem = stem.strip_suffix(".cad").unwrap_or(stem);
            path.parent()
                .unwrap_or_else(|| Path::new("."))
                .join(format!("{stem}.drawing.json"))
        });
    fs::write(
        &out,
        serde_json::to_string_pretty(&packet).map_err(|e| ToolError::msg(e.to_string()))?,
    )
    .map_err(|e| ToolError::msg(e.to_string()))?;
    let payload = json!({
        "ok": packet.ok,
        "out": out,
        "packet": packet,
        "note": "not a drafting package — H2-8/H3-3 PMI alpha"
    });
    Ok(json!({
        "content": [{"type": "text", "text": serde_json::to_string_pretty(&payload).unwrap()}]
    }))
}

fn tool_assembly_validate(args: &Value) -> Result<Value, ToolError> {
    let path = PathBuf::from(str_arg(args, "path")?);
    let text = fs::read_to_string(&path).map_err(|e| ToolError::msg(e.to_string()))?;
    let spec: AssemblySpec =
        serde_json::from_str(&text).map_err(|e| ToolError::msg(format!("assy json: {e}")))?;
    let report = validate_assembly(&spec);
    let payload = json!({
        "ok": report.ok,
        "path": path,
        "report": report,
    });
    Ok(json!({
        "content": [{"type": "text", "text": serde_json::to_string_pretty(&payload).unwrap()}]
    }))
}

fn tool_sdf_sample(args: &Value) -> Result<Value, ToolError> {
    let prim_s = str_arg(args, "prim")?;
    let a = args
        .get("a")
        .and_then(|v| v.as_f64())
        .ok_or_else(|| ToolError::msg("a number required"))?;
    let b = args
        .get("b")
        .and_then(|v| v.as_f64())
        .ok_or_else(|| ToolError::msg("b number required"))?;
    if a <= 0.0 || b <= 0.0 {
        return Err(ToolError::msg("a and b must be > 0"));
    }
    let prim = match prim_s {
        "box" => {
            let c = args
                .get("c")
                .and_then(|v| v.as_f64())
                .filter(|c| *c > 0.0)
                .ok_or_else(|| ToolError::msg("box requires c > 0 (dz)"))?;
            SdfPrim::Box {
                dx: a,
                dy: b,
                dz: c,
            }
        }
        "cylinder" => SdfPrim::Cylinder { r: a, h: b },
        other => {
            return Err(ToolError::msg(format!(
                "prim must be box|cylinder, got {other}"
            )))
        }
    };
    let res = args.get("res").and_then(|v| v.as_u64()).unwrap_or(24) as usize;
    let pad = args.get("pad").and_then(|v| v.as_f64()).unwrap_or(2.0);
    let out = args
        .get("out")
        .and_then(|v| v.as_str())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("sdf_out"));
    let stem = args
        .get("stem")
        .and_then(|v| v.as_str())
        .unwrap_or(prim_s)
        .to_string();
    let grid = grid_for_prim(prim, res, pad);
    let vol = sample_analytic(prim, &grid).map_err(|e| ToolError::msg(e.to_string()))?;
    let (raw, meta) = write_raw(&vol, &out, &stem).map_err(|e| ToolError::msg(e.to_string()))?;
    let (nrrd, nraw) = write_nrrd(&vol, &out, &stem).map_err(|e| ToolError::msg(e.to_string()))?;
    let payload = json!({
        "ok": true,
        "secondary": true,
        "note": "experimental SDF — not a modeling path; STEP remains primary",
        "voxel_count": vol.values.len(),
        "raw_f32": raw,
        "meta": meta,
        "nrrd": nrrd,
        "nrrd_raw": nraw,
    });
    Ok(json!({
        "content": [{"type": "text", "text": serde_json::to_string_pretty(&payload).unwrap()}]
    }))
}

fn tool_align_check(args: &Value) -> Result<Value, ToolError> {
    let path = PathBuf::from(str_arg(args, "path")?);
    let a = str_arg(args, "a")?.to_string();
    let b = str_arg(args, "b")?.to_string();
    let expect = match args
        .get("expect")
        .and_then(|v| v.as_str())
        .unwrap_or("distance")
    {
        "coplanar" => AlignExpect::Coplanar,
        "coaxial" => AlignExpect::Coaxial,
        "distance" => AlignExpect::Distance,
        other => return Err(ToolError::msg(format!("bad expect {other}"))),
    };
    let distance = args.get("distance").and_then(|v| v.as_f64());
    let tol = args.get("tol").and_then(|v| v.as_f64()).unwrap_or(0.1);
    let tol_deg = args.get("tol_deg").and_then(|v| v.as_f64()).unwrap_or(1.0);
    let ir = eval_path(&path, None)?;
    let snap = topo_from_ir(&ir)?;
    let r = align_refs(&snap, &a, &b, expect, distance, tol, tol_deg)
        .map_err(|e| ToolError::msg(e.to_string()))?;
    Ok(json!({
        "content": [{"type": "text", "text": serde_json::to_string_pretty(&r).unwrap()}]
    }))
}

fn tool_frame(args: &Value) -> Result<Value, ToolError> {
    let path = PathBuf::from(str_arg(args, "path")?);
    let selector = str_arg(args, "selector")?.to_string();
    let ir = eval_path(&path, None)?;
    let snap = topo_from_ir(&ir)?;
    let r = frame_of(&snap, &selector).map_err(|e| ToolError::msg(e.to_string()))?;
    Ok(json!({
        "content": [{"type": "text", "text": serde_json::to_string_pretty(&r).unwrap()}]
    }))
}

fn tool_diff(args: &Value) -> Result<Value, ToolError> {
    let old = PathBuf::from(str_arg(args, "old")?);
    let new = PathBuf::from(str_arg(args, "new")?);
    let ir_old = eval_path(&old, None)?;
    let ir_new = eval_path(&new, None)?;
    let snap_old = topo_from_ir(&ir_old)?;
    let snap_new = topo_from_ir(&ir_new)?;
    let r = diff_snapshots(&snap_old, &snap_new);
    let payload = json!({
        "ok": true,
        "diff": r,
        "old": old,
        "new": new,
    });
    Ok(json!({
        "content": [{"type": "text", "text": serde_json::to_string_pretty(&payload).unwrap()}]
    }))
}

fn tool_export(args: &Value) -> Result<Value, ToolError> {
    let path = PathBuf::from(str_arg(args, "path")?);
    let format = str_arg(args, "format")?;
    let ir = eval_path(&path, None)?;
    let stem = export_stem(&path);
    match format {
        "step" => {
            let out = args
                .get("out")
                .and_then(|v| v.as_str())
                .map(PathBuf::from)
                .unwrap_or_else(|| path.with_file_name(format!("{stem}.step")));
            let mut kernel = MockKernel::new();
            let shape = execute_ir(&mut kernel, &ir).map_err(|e| ToolError::msg(e.to_string()))?;
            match kernel.write_step(shape, &out, &StepWriteOpts::default()) {
                Ok(()) => Ok(json!({
                    "content": [{"type": "text", "text": serde_json::to_string_pretty(&json!({
                        "ok": true,
                        "path": out,
                        "format": "step"
                    })).unwrap()}]
                })),
                Err(e) => {
                    if out.exists() {
                        let _ = fs::remove_file(&out);
                    }
                    Err(ToolError::msg(format!("{}: {e}", e.code())))
                }
            }
        }
        "stl" => {
            let out = args
                .get("out")
                .and_then(|v| v.as_str())
                .map(PathBuf::from)
                .unwrap_or_else(|| path.with_file_name(format!("{stem}.stl")));
            let (mesh, notes) = mesh_from_ir(&ir).map_err(ToolError::msg)?;
            write_stl_ascii(&out, &mesh).map_err(|e| ToolError::msg(e.to_string()))?;
            Ok(json!({
                "content": [{"type": "text", "text": serde_json::to_string_pretty(&json!({
                    "ok": true,
                    "path": out,
                    "format": "stl",
                    "triangles": mesh.triangle_count(),
                    "mesh": "ir-analytic-preview",
                    "notes": notes
                })).unwrap()}]
            }))
        }
        "gltf" | "glb" => {
            let requested = args
                .get("out")
                .and_then(|v| v.as_str())
                .map(PathBuf::from)
                .unwrap_or_else(|| path.with_file_name(format!("{stem}.{format}")));
            let (mesh, notes) = mesh_from_ir(&ir).map_err(ToolError::msg)?;
            let out =
                write_gltf_json(&requested, &mesh).map_err(|e| ToolError::msg(e.to_string()))?;
            Ok(json!({
                "content": [{"type": "text", "text": serde_json::to_string_pretty(&json!({
                    "ok": true,
                    "path": out,
                    "format": "gltf",
                    "note": "JSON glTF (embedded buffer); binary .glb container is follow-up",
                    "triangles": mesh.triangle_count(),
                    "mesh": "ir-analytic-preview",
                    "notes": notes
                })).unwrap()}]
            }))
        }
        other => Err(ToolError::msg(format!(
            "format must be step|stl|gltf|glb, got {other}"
        ))),
    }
}

fn export_stem(path: &Path) -> String {
    let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("part");
    name.strip_suffix(".cad.star")
        .or_else(|| name.strip_suffix(".star"))
        .unwrap_or_else(|| path.file_stem().and_then(|s| s.to_str()).unwrap_or("part"))
        .to_string()
}

fn tool_fab_check(args: &Value) -> Result<Value, ToolError> {
    let path = PathBuf::from(str_arg(args, "path")?);
    let text = fs::read_to_string(&path).map_err(|e| ToolError::msg(e.to_string()))?;
    let part: FlatPart =
        serde_json::from_str(&text).map_err(|e| ToolError::msg(format!("FlatPart json: {e}")))?;
    let mut profile = if let Some(pf) = args.get("profile_file").and_then(|v| v.as_str()) {
        let t = fs::read_to_string(pf).map_err(|e| ToolError::msg(e.to_string()))?;
        load_profile_json(&t).map_err(ToolError::msg)?
    } else {
        let id = args
            .get("profile")
            .and_then(|v| v.as_str())
            .unwrap_or("sendcutsend.laser");
        resolve_bundled_profile(id).ok_or_else(|| {
            ToolError::msg(format!(
                "unknown profile {id:?} (sendcutsend.laser|pcb.outline|waterjet.generic)"
            ))
        })?
    };
    if let Some(ovp) = args.get("override_file").and_then(|v| v.as_str()) {
        let t = fs::read_to_string(ovp).map_err(|e| ToolError::msg(e.to_string()))?;
        let ov = load_override_json(&t).map_err(ToolError::msg)?;
        profile = apply_override(&profile, &ov).map_err(ToolError::msg)?;
    }
    let report = check_dfm(&profile, &part);
    let payload = json!({
        "ok": report.ok,
        "report": report,
        "part": part,
        "printer_start": false
    });
    Ok(json!({
        "content": [{"type": "text", "text": serde_json::to_string_pretty(&payload).unwrap()}]
    }))
}

fn tool_engine(args: &Value) -> Result<Value, ToolError> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("info");
    let payload = match action {
        "info" => crate::engine::info_json(),
        "install" => {
            let backend = args
                .get("backend")
                .and_then(|v| v.as_str())
                .unwrap_or("occt");
            crate::engine::install(backend).map_err(ToolError::msg)?
        }
        other => {
            return Err(ToolError::msg(format!(
                "unknown engine action {other:?} (info|install)"
            )))
        }
    };
    Ok(json!({
        "content": [{"type": "text", "text": serde_json::to_string_pretty(&payload).unwrap()}]
    }))
}

fn tool_schema(args: &Value) -> Result<Value, ToolError> {
    let face = args.get("face").and_then(|v| v.as_str()).unwrap_or("mcp");
    let payload = crate::schema::dump(face).map_err(ToolError::msg)?;
    Ok(json!({
        "content": [{"type": "text", "text": serde_json::to_string_pretty(&payload).unwrap()}]
    }))
}

fn tool_robot(args: &Value) -> Result<Value, ToolError> {
    let op = str_arg(args, "op")?;
    let path = PathBuf::from(str_arg(args, "path")?);
    let payload = match op {
        "gen" => robot_gen(&path, args)?,
        "validate" => robot_validate(&path)?,
        other => {
            return Err(ToolError::msg(format!(
                "unknown robot op {other:?} (gen|validate)"
            )))
        }
    };
    Ok(json!({
        "content": [{"type": "text", "text": serde_json::to_string_pretty(&payload).unwrap()}]
    }))
}

fn tool_parts(args: &Value) -> Result<Value, ToolError> {
    let op = str_arg(args, "op")?;
    let project = crate::policy::policy().project_root;
    let root = args
        .get("parts_root")
        .and_then(|v| v.as_str())
        .map(PathBuf::from)
        .unwrap_or_else(|| project.join("parts"));
    let payload = match op {
        "search" => parts_search(
            &root,
            args.get("query").and_then(|v| v.as_str()).unwrap_or(""),
        )?,
        "fetch" => parts_fetch(&root, str_arg(args, "id")?)?,
        "lock" => {
            let id = str_arg(args, "id")?;
            let project = args
                .get("project")
                .and_then(|v| v.as_str())
                .map(PathBuf::from)
                .unwrap_or_else(|| infer_parts_project(&root, &project));
            let lock_path = args
                .get("lock")
                .and_then(|v| v.as_str())
                .map(PathBuf::from)
                .unwrap_or_else(|| project.join("parts.lock"));
            let key = args.get("key").and_then(|v| v.as_str()).unwrap_or(id);
            parts_lock(&project, &root, &lock_path, id, key)?
        }
        other => {
            return Err(ToolError::msg(format!(
                "unknown parts op {other:?} (search|fetch|lock)"
            )))
        }
    };
    Ok(json!({
        "content": [{"type": "text", "text": serde_json::to_string_pretty(&payload).unwrap()}]
    }))
}

fn infer_parts_project(parts_root: &Path, policy_root: &Path) -> PathBuf {
    if parts_root.file_name().is_some_and(|n| n == "parts") {
        if let Some(parent) = parts_root.parent() {
            if parent.as_os_str().is_empty() {
                return policy_root.to_path_buf();
            }
            return parent.to_path_buf();
        }
    }
    policy_root.to_path_buf()
}

fn parts_rel_to_project(project: &Path, file: &Path) -> String {
    let file_c = file.canonicalize().unwrap_or_else(|_| file.to_path_buf());
    let proj_c = project
        .canonicalize()
        .unwrap_or_else(|_| project.to_path_buf());
    file_c
        .strip_prefix(&proj_c)
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|_| file.to_string_lossy().replace('\\', "/"))
}

fn parts_search(root: &Path, query: &str) -> Result<Value, ToolError> {
    let prov = LocalFsProvider::new(root);
    let results = prov
        .search(query)
        .map_err(|e| ToolError::msg(e.to_string()))?;
    Ok(json!({
        "ok": true,
        "provider": prov.id(),
        "parts_root": root,
        "results": results,
        "storefront": false
    }))
}

fn parts_fetch(root: &Path, id: &str) -> Result<Value, ToolError> {
    let prov = LocalFsProvider::new(root);
    match prov.fetch(id) {
        Ok(meta) => Ok(json!({
            "ok": true,
            "provider": prov.id(),
            "meta": meta,
            "downloaded": false,
            "storefront": false
        })),
        Err(cadrion_parts::ProviderError::NotFound(id)) => Err(ToolError::msg(format!(
            "CADRION-E-PARTS-NOT-FOUND: no local STEP for {id}"
        ))),
        Err(e) => Err(ToolError::msg(e.to_string())),
    }
}

fn parts_lock(
    project: &Path,
    root: &Path,
    lock_path: &Path,
    id: &str,
    key: &str,
) -> Result<Value, ToolError> {
    let fetch = parts_fetch(root, id)?;
    let meta = &fetch["meta"];
    let path = PathBuf::from(meta["path"].as_str().unwrap_or(""));
    let entry = PartsLockEntry {
        provider: fetch["provider"].as_str().unwrap_or("local").into(),
        id: meta["id"].as_str().unwrap_or(id).into(),
        version: None,
        sha256: meta["sha256"].as_str().unwrap_or("").into(),
        path: parts_rel_to_project(project, &path),
        license: meta["license"].as_str().map(|s| s.to_string()),
    };
    let written = upsert_lock_entry(lock_path, key, entry.clone())
        .map_err(|e| ToolError::msg(format!("CADRION-E-PARTS-LOCK: {e}")))?;
    verify_lock_entry(&written, key, project)
        .map_err(|e| ToolError::msg(format!("CADRION-E-PARTS-LOCK: {e}")))?;
    Ok(json!({
        "ok": true,
        "key": key,
        "entry": entry,
        "lock": lock_path,
        "verified": true,
        "storefront": false
    }))
}

fn tool_viewer_open(args: &Value) -> Result<Value, ToolError> {
    let once = args.get("once").and_then(|v| v.as_bool()).unwrap_or(true);
    if !once {
        return Err(ToolError::msg(
            "CADRION-E-VIEW: MCP/HTTP will not start the accept loop (use cadrion view, or once=true)",
        ));
    }
    let mut paths: Vec<PathBuf> = Vec::new();
    if let Some(arr) = args.get("paths").and_then(|v| v.as_array()) {
        for p in arr {
            if let Some(s) = p.as_str() {
                paths.push(PathBuf::from(s));
            }
        }
    }
    if let Some(p) = args.get("path").and_then(|v| v.as_str()) {
        paths.push(PathBuf::from(p));
    }
    if paths.is_empty() {
        return Err(ToolError::msg(
            "CADRION-E-USAGE: viewer_open needs path or paths",
        ));
    }
    let mut links = Vec::new();
    for p in paths {
        if !p.exists() {
            return Err(ToolError::msg(format!(
                "CADRION-E-VIEW: not found: {}",
                p.display()
            )));
        }
        let kind = viewer_kind(&p);
        links.push(json!({
            "path": p,
            "kind": kind,
            "url": null,
            "note": "prepared; run cadrion view without --once to serve loopback HTML"
        }));
    }
    let payload = json!({
        "ok": true,
        "once": true,
        "served": false,
        "interactive_cad": false,
        "wgpu": false,
        "links": links
    });
    Ok(json!({
        "content": [{"type": "text", "text": serde_json::to_string_pretty(&payload).unwrap()}]
    }))
}

fn viewer_kind(p: &Path) -> &'static str {
    if p.is_dir() {
        return "snap";
    }
    let name = p
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if name.ends_with(".cad.star") || name.ends_with(".star") {
        "star"
    } else if name.ends_with(".gcode") || name.ends_with(".gco") || name.ends_with(".nc") {
        "gcode"
    } else if name.ends_with(".robot.json") || name.ends_with(".urdf") {
        "robot"
    } else if name.ends_with(".png") || name.ends_with(".gif") || name.ends_with(".jpg") {
        "image"
    } else {
        "file"
    }
}

fn tool_gcode_check(args: &Value) -> Result<Value, ToolError> {
    let path = PathBuf::from(str_arg(args, "path")?);
    let text = fs::read_to_string(&path).map_err(|e| ToolError::msg(e.to_string()))?;
    let vol = PrinterVolume {
        x_mm: args.get("bed_x").and_then(|v| v.as_f64()).unwrap_or(256.0),
        y_mm: args.get("bed_y").and_then(|v| v.as_f64()).unwrap_or(256.0),
        z_mm: args.get("bed_z").and_then(|v| v.as_f64()).unwrap_or(256.0),
        max_hotend_c: args
            .get("max_hotend")
            .and_then(|v| v.as_f64())
            .unwrap_or(300.0),
        max_bed_c: args
            .get("max_bed")
            .and_then(|v| v.as_f64())
            .unwrap_or(110.0),
    };
    let report = check_gcode(&text, &vol);
    let payload = json!({
        "ok": report.ok,
        "report": report,
        "sha256": hex_sha256(text.as_bytes()),
        "path": path,
        "printer_start": false
    });
    Ok(json!({
        "content": [{"type": "text", "text": serde_json::to_string_pretty(&payload).unwrap()}]
    }))
}

fn load_robot_spec(path: &Path) -> Result<RobotSpec, ToolError> {
    let text = fs::read_to_string(path).map_err(|e| ToolError::msg(e.to_string()))?;
    serde_json::from_str(&text).map_err(|e| {
        ToolError::msg(format!(
            "robot json: {e} (inertial is required per link; not invented)"
        ))
    })
}

fn robot_gen(path: &Path, args: &Value) -> Result<Value, ToolError> {
    let robot = load_robot_spec(path)?;
    let (urdf, report) = emit_and_validate(&robot);
    if !report.ok {
        return Ok(json!({
            "ok": false,
            "report": report,
            "inertial_invented": false
        }));
    }
    let out_dir = args
        .get("out")
        .and_then(|v| v.as_str())
        .map(PathBuf::from)
        .unwrap_or_else(|| path.parent().unwrap_or(Path::new(".")).to_path_buf());
    fs::create_dir_all(&out_dir).map_err(|e| ToolError::msg(e.to_string()))?;
    let urdf_path = out_dir.join(format!("{}.urdf", robot.name));
    fs::write(&urdf_path, &urdf).map_err(|e| ToolError::msg(e.to_string()))?;
    let mut files = vec![urdf_path.display().to_string()];
    let with_srdf = args.get("srdf").and_then(|v| v.as_bool()).unwrap_or(true);
    let with_sdf = args.get("sdf").and_then(|v| v.as_bool()).unwrap_or(true);
    if with_srdf {
        let srdf = srdf_from_robot(&robot, "arm");
        let p = out_dir.join(format!("{}.srdf", robot.name));
        fs::write(&p, write_srdf(&srdf)).map_err(|e| ToolError::msg(e.to_string()))?;
        files.push(p.display().to_string());
    }
    if with_sdf {
        let p = out_dir.join(format!("{}.sdf", robot.name));
        fs::write(&p, write_sdf(&robot)).map_err(|e| ToolError::msg(e.to_string()))?;
        files.push(p.display().to_string());
    }
    Ok(json!({
        "ok": true,
        "report": report,
        "files": files,
        "inertial_invented": false
    }))
}

fn robot_validate(path: &Path) -> Result<Value, ToolError> {
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let report = if name.ends_with(".json") {
        let robot = load_robot_spec(path)?;
        emit_and_validate(&robot).1
    } else {
        let text = fs::read_to_string(path).map_err(|e| ToolError::msg(e.to_string()))?;
        if name.ends_with(".urdf") {
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
            return Err(ToolError::msg(
                "expected .json/.urdf/.srdf/.sdf (validate does not write files)",
            ));
        }
    };
    Ok(json!({
        "ok": report.ok,
        "report": report,
        "target": path.display().to_string(),
        "inertial_invented": false,
        "wrote": false
    }))
}

fn topo_from_ir(
    ir: &cadrion_lang::FeatureIr,
) -> Result<cadrion_inspect::TopologySnapshot, ToolError> {
    // Duplicate thin walker (same as bench) to avoid depending on cadrion-cli.
    use cadrion_inspect::{box_topology, cylinder_topology, SolidRec, TopologySnapshot};
    use cadrion_kernel::Point3;
    use cadrion_lang::{BooleanKind, IrNode};

    let mut solids: Vec<Option<SolidRec>> = vec![None; ir.nodes.len()];
    for (idx, node) in ir.nodes.iter().enumerate() {
        let rec = match node {
            IrNode::Box { dx, dy, dz, at } => {
                box_topology(*dx, *dy, *dz, Point3::new(at[0], at[1], at[2]))
            }
            IrNode::Cylinder { radius, height, at } => {
                cylinder_topology(*radius, *height, Point3::new(at[0], at[1], at[2]))
            }
            IrNode::Sphere { radius, at } => {
                let r = *radius;
                box_topology(2.0 * r, 2.0 * r, 2.0 * r, Point3::new(at[0], at[1], at[2]))
            }
            IrNode::Cone { radius, height, at } => {
                // H3-1: IR topology approx uses cylinder bbox — dims on cones are amber.
                cylinder_topology(*radius, *height, Point3::new(at[0], at[1], at[2]))
            }
            IrNode::Boolean { kind, a, b } => {
                let sa = solids
                    .get(a.0 as usize)
                    .and_then(|s| s.as_ref())
                    .ok_or_else(|| ToolError::msg("bad IR node"))?;
                let sb = solids
                    .get(b.0 as usize)
                    .and_then(|s| s.as_ref())
                    .ok_or_else(|| ToolError::msg("bad IR node"))?;
                let volume = match kind {
                    BooleanKind::Union => sa.volume_mm3 + sb.volume_mm3,
                    BooleanKind::Cut => (sa.volume_mm3 - sb.volume_mm3).max(0.0),
                    BooleanKind::Intersect => sa.volume_mm3.min(sb.volume_mm3),
                };
                SolidRec {
                    volume_mm3: volume,
                    centroid: sa.centroid,
                    faces: sa.faces.clone(),
                    edges: sa.edges.clone(),
                    vertices: sa.vertices.clone(),
                }
            }
            IrNode::Fillet { of, .. }
            | IrNode::Chamfer { of, .. }
            | IrNode::Label { of, .. }
            | IrNode::Translate { of, .. }
            | IrNode::Rotate { of, .. }
            | IrNode::Mirror { of, .. } => solids
                .get(of.0 as usize)
                .and_then(|s| s.as_ref())
                .ok_or_else(|| ToolError::msg("bad IR node"))?
                .clone(),
        };
        solids[idx] = Some(rec);
    }
    let root = solids
        .get(ir.root.0 as usize)
        .and_then(|s| s.as_ref())
        .ok_or_else(|| ToolError::msg("missing root"))?
        .clone();
    Ok(TopologySnapshot::single_solid(root))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn align_frame_diff_on_box() {
        let dir = std::env::temp_dir().join(format!("cadrion-h5-2-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("box.cad.star");
        fs::write(
            &path,
            "def gen_step():\n    return solid(box(10.0, 10.0, 10.0, at=CENTER), label=\"cube\")\n",
        )
        .unwrap();
        let refs = call_tool(
            "inspect_refs",
            &json!({"path": path.display().to_string(), "facts": true}),
        )
        .unwrap();
        let text = refs["content"][0]["text"].as_str().unwrap();
        let report: Value = serde_json::from_str(text).unwrap();
        let faces: Vec<&Value> = report["refs"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["kind"] == "face")
            .collect();
        let top = faces
            .iter()
            .find(|r| r["normal"]["z"].as_f64() == Some(1.0))
            .unwrap();
        let bot = faces
            .iter()
            .find(|r| r["normal"]["z"].as_f64() == Some(-1.0))
            .unwrap();
        let a = top["selector"].as_str().unwrap();
        let b = bot["selector"].as_str().unwrap();
        let align = call_tool(
            "align_check",
            &json!({
                "path": path.display().to_string(),
                "a": a,
                "b": b,
                "expect": "coaxial"
            }),
        )
        .unwrap();
        let align_p: Value =
            serde_json::from_str(align["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(align_p["ok"], true, "{align_p}");
        let frame = call_tool(
            "frame",
            &json!({"path": path.display().to_string(), "selector": a}),
        )
        .unwrap();
        let frame_p: Value =
            serde_json::from_str(frame["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(frame_p["kind"], "face");
        let p = path.display().to_string();
        let diff = call_tool("diff", &json!({"old": p, "new": p})).unwrap();
        let diff_p: Value =
            serde_json::from_str(diff["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(diff_p["diff"]["volume_delta_mm3"], 0.0);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn export_stl_writes_and_step_refuses() {
        let dir = std::env::temp_dir().join(format!("cadrion-h5-3-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("box.cad.star");
        fs::write(
            &path,
            "def gen_step():\n    return solid(box(10.0, 10.0, 10.0, at=CENTER), label=\"cube\")\n",
        )
        .unwrap();
        let stl = dir.join("box.stl");
        let ok = call_tool(
            "export",
            &json!({
                "path": path.display().to_string(),
                "format": "stl",
                "out": stl.display().to_string()
            }),
        )
        .unwrap();
        let p: Value = serde_json::from_str(ok["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(p["ok"], true);
        assert_eq!(p["mesh"], "ir-analytic-preview");
        let body = fs::read_to_string(&stl).unwrap();
        assert!(body.starts_with("solid cadrion"), "{body}");

        let step = dir.join("box.step");
        let err = call_tool(
            "export",
            &json!({
                "path": path.display().to_string(),
                "format": "step",
                "out": step.display().to_string()
            }),
        )
        .unwrap_err();
        assert!(err.to_string().contains("CADRION-E-UNSUPPORTED"), "{err}");
        assert!(!step.exists(), "mock must not write a STEP file");
        let _ = fs::remove_dir_all(&dir);
    }

    fn plate_flat_json() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/fab/plate.flat.json")
    }

    #[test]
    fn fab_check_plate_cites_profile_and_refuses_unknown() {
        let path = plate_flat_json();
        let ok = call_tool("fab_check", &json!({"path": path.display().to_string()})).unwrap();
        let p: Value = serde_json::from_str(ok["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(p["ok"], true, "{p}");
        assert_eq!(p["printer_start"], false);
        assert_eq!(p["report"]["profile_id"], "sendcutsend.laser");
        assert_eq!(p["report"]["profile_version"], "1.0.0");

        let dir = std::env::temp_dir().join(format!("cadrion-h5-4-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let fail_path = dir.join("tiny-hole.flat.json");
        fs::write(
            &fail_path,
            r#"{
              "width_mm": 100.0,
              "height_mm": 50.0,
              "thickness_mm": 3.0,
              "material": "Aluminum 5052",
              "holes_dia_mm": [0.4]
            }"#,
        )
        .unwrap();
        let fail = call_tool(
            "fab_check",
            &json!({"path": fail_path.display().to_string()}),
        )
        .unwrap();
        let fail_p: Value =
            serde_json::from_str(fail["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(fail_p["ok"], false, "{fail_p}");
        assert_eq!(fail_p["report"]["profile_version"], "1.0.0");
        assert_eq!(fail_p["printer_start"], false);

        let err = call_tool(
            "fab_check",
            &json!({"path": path.display().to_string(), "profile": "not-a-vendor"}),
        )
        .unwrap_err();
        assert!(err.to_string().contains("unknown profile"), "{err}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn engine_info_and_fail_closed_install() {
        let info = call_tool("engine", &json!({})).unwrap();
        let p: Value = serde_json::from_str(info["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(p["ok"], true);
        assert_eq!(p["compiled"]["mock"], true);
        assert_eq!(p["prebuilt_fetch"], false);

        let inst = call_tool("engine", &json!({"action": "install", "backend": "occt"}));
        match inst {
            Ok(v) => {
                let p: Value =
                    serde_json::from_str(v["content"][0]["text"].as_str().unwrap()).unwrap();
                assert_eq!(p["already_present"], true);
                assert_eq!(p["prebuilt_fetch"], false);
            }
            Err(e) => assert!(e.to_string().contains("CADRION-E-ENGINE-MISSING"), "{e}"),
        }
    }

    #[test]
    fn schema_mcp_lists_engine_and_schema() {
        let v = call_tool("schema", &json!({})).unwrap();
        let p: Value = serde_json::from_str(v["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(p["ok"], true);
        assert_eq!(p["source"], "live-surfaces");
        let names = p["mcp"]["tool_names"].as_array().unwrap();
        assert!(names.iter().any(|n| n == "engine"));
        assert!(names.iter().any(|n| n == "schema"));
        let err = call_tool("schema", &json!({"face": "cli"})).unwrap_err();
        assert!(err.to_string().contains("unknown schema face"), "{err}");
    }

    fn simple_arm_json() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/robots/simple_arm.robot.json")
    }

    #[test]
    fn robot_gen_and_validate_simple_arm_no_invented_inertial() {
        let spec = simple_arm_json();
        let dir = std::env::temp_dir().join(format!("cadrion-h5-8-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let gen = call_tool(
            "robot",
            &json!({
                "op": "gen",
                "path": spec.display().to_string(),
                "out": dir.display().to_string()
            }),
        )
        .unwrap();
        let p: Value = serde_json::from_str(gen["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(p["ok"], true, "{p}");
        assert_eq!(p["inertial_invented"], false);
        assert_eq!(p["report"]["ok"], true);
        let urdf = dir.join("simple_arm.urdf");
        assert!(urdf.is_file());

        let val = call_tool(
            "robot",
            &json!({
                "op": "validate",
                "path": spec.display().to_string()
            }),
        )
        .unwrap();
        let v: Value = serde_json::from_str(val["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(v["ok"], true, "{v}");
        assert_eq!(v["wrote"], false);
        assert_eq!(v["inertial_invented"], false);

        let urdf_val = call_tool(
            "robot",
            &json!({
                "op": "validate",
                "path": urdf.display().to_string()
            }),
        )
        .unwrap();
        let u: Value =
            serde_json::from_str(urdf_val["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(u["ok"], true, "{u}");

        let bad = dir.join("no-inertial.robot.json");
        fs::write(&bad, r#"{"name":"x","links":[{"name":"a"}],"joints":[]}"#).unwrap();
        let err = call_tool(
            "robot",
            &json!({"op": "validate", "path": bad.display().to_string()}),
        )
        .unwrap_err();
        assert!(err.to_string().contains("inertial"), "{err}");
        let _ = fs::remove_dir_all(&dir);
    }

    fn assembly_parts_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/assembly/parts")
    }

    #[test]
    fn parts_search_fetch_lock_local_only() {
        let root = assembly_parts_root();
        let project = root.parent().unwrap();
        let hits = call_tool(
            "parts",
            &json!({
                "op": "search",
                "query": "m6",
                "parts_root": root.display().to_string()
            }),
        )
        .unwrap();
        let p: Value = serde_json::from_str(hits["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(p["ok"], true, "{p}");
        assert_eq!(p["storefront"], false);
        assert!(p["results"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["id"] == "m6_bolt"));

        let fetched = call_tool(
            "parts",
            &json!({
                "op": "fetch",
                "id": "m6_bolt",
                "parts_root": root.display().to_string()
            }),
        )
        .unwrap();
        let f: Value =
            serde_json::from_str(fetched["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(f["ok"], true, "{f}");
        assert_eq!(f["downloaded"], false);
        assert_eq!(f["meta"]["sha256"].as_str().unwrap().len(), 64);

        let dir = std::env::temp_dir().join(format!("cadrion-h6-1-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let lock = dir.join("parts.lock");
        let locked = call_tool(
            "parts",
            &json!({
                "op": "lock",
                "id": "m6_bolt",
                "parts_root": root.display().to_string(),
                "lock": lock.display().to_string()
            }),
        )
        .unwrap();
        let l: Value =
            serde_json::from_str(locked["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(l["ok"], true, "{l}");
        assert_eq!(l["verified"], true);
        assert_eq!(l["entry"]["path"], "parts/m6_bolt.step");
        assert!(lock.is_file());
        let on_disk = cadrion_parts::load_parts_lock(&lock).unwrap();
        cadrion_parts::verify_lock_entry(&on_disk, "m6_bolt", project).unwrap();

        let err = call_tool(
            "parts",
            &json!({
                "op": "fetch",
                "id": "no-such-part",
                "parts_root": root.display().to_string()
            }),
        )
        .unwrap_err();
        assert!(
            err.to_string().contains("CADRION-E-PARTS-NOT-FOUND"),
            "{err}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn viewer_open_once_does_not_serve() {
        let path = simple_arm_json();
        let v = call_tool("viewer_open", &json!({"path": path.display().to_string()})).unwrap();
        let p: Value = serde_json::from_str(v["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(p["ok"], true, "{p}");
        assert_eq!(p["once"], true);
        assert_eq!(p["served"], false);
        assert_eq!(p["interactive_cad"], false);
        assert_eq!(p["wgpu"], false);
        assert_eq!(p["links"][0]["kind"], "robot");
        assert!(p["links"][0]["url"].is_null());

        let err = call_tool(
            "viewer_open",
            &json!({"path": path.display().to_string(), "once": false}),
        )
        .unwrap_err();
        assert!(err.to_string().contains("CADRION-E-VIEW"), "{err}");
        assert!(err.to_string().contains("accept loop"), "{err}");
    }

    #[test]
    fn gcode_check_sample_is_not_a_print() {
        let path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/fab/sample.gcode");
        let v = call_tool("gcode_check", &json!({"path": path.display().to_string()})).unwrap();
        let p: Value = serde_json::from_str(v["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(p["ok"], true, "{p}");
        assert_eq!(p["printer_start"], false);
        assert_eq!(p["sha256"].as_str().unwrap().len(), 64);
        assert!(p["report"]["move_count"].as_u64().unwrap() > 0);
    }
}
