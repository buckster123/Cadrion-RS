//! MCP tool implementations (thin wrappers over lang/inspect/render + fs).

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use cadrion_inspect::{
    align_refs, build_drawing_packet, diff_snapshots, frame_of, inspect_refs, measure, AlignExpect,
    DimSpec, MeasureKind, MeasureRequest,
};
use cadrion_lang::{evaluate, EvalOptions};
use cadrion_parts::{validate_assembly, AssemblySpec};
use cadrion_render::{mesh_from_ir, write_snapshot_packet, SnapshotOptions, ViewName};
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
}
