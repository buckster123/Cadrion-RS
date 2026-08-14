//! `cadre inspect`

use std::fs;

use cadre_inspect::{
    align_refs, build_drawing_packet, diff_snapshots, frame_of, inspect_refs, measure, AlignExpect,
    DimSpec, MeasureKind, MeasureRequest, TopologySnapshot,
};
#[cfg(feature = "occt")]
use cadre_lang::execute_ir;
use cadre_lang::{evaluate, EvalOptions, FeatureIr};
use serde_json::json;

use crate::build_cmd::parse_sets;
use crate::cli::Cli;
use crate::cli::{AlignExpectArg, InspectArgs, InspectCmd, MeasureKindArg};
use crate::kernel_pick::open_kernel;
#[cfg(feature = "occt")]
use crate::kernel_pick::KernelBox;
use crate::output::{emit, ExitCode};
use crate::topo_from_ir::topology_from_ir;

pub fn run(cli: &Cli, args: &InspectArgs) -> ExitCode {
    match &args.cmd {
        InspectCmd::Refs(a) => refs(cli, a.target.clone(), a.facts, &a.set),
        InspectCmd::Measure(a) => measure_cmd(
            cli,
            a.target.clone(),
            a.a.clone(),
            a.b.clone(),
            a.kind,
            &a.set,
        ),
        InspectCmd::Align(a) => align_cmd(cli, a),
        InspectCmd::Frame(a) => frame_cmd(cli, a),
        InspectCmd::Diff(a) => diff_cmd(cli, a),
        InspectCmd::Dims(a) => dims_cmd(cli, a),
    }
}

fn load_ir(
    target: &std::path::Path,
    sets: &[String],
) -> Result<FeatureIr, (ExitCode, serde_json::Value)> {
    if target
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e == "json")
    {
        let text = fs::read_to_string(target).map_err(|e| {
            (
                ExitCode::Io,
                json!({"ok": false, "diagnostics": [{"code": "CADRE-E-IO", "message": e.to_string()}]}),
            )
        })?;
        let ir: FeatureIr = serde_json::from_str(&text).map_err(|e| {
            (
                ExitCode::Eval,
                json!({"ok": false, "diagnostics": [{"code": "CADRE-E-EVAL", "message": format!("bad IR json: {e}")}]}),
            )
        })?;
        return Ok(ir);
    }

    let source = fs::read_to_string(target).map_err(|e| {
        (
            ExitCode::Io,
            json!({"ok": false, "diagnostics": [{"code": "CADRE-E-IO", "message": e.to_string()}]}),
        )
    })?;
    let overrides = parse_sets(sets).map_err(|m| {
        (
            ExitCode::Usage,
            json!({"ok": false, "diagnostics": [{"code": "CADRE-E-USAGE", "message": m}]}),
        )
    })?;
    let name = target
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
    Ok(eval.ir.expect("ir"))
}

/// Prefer live OCCT topology when `--kernel occt`; else IR analytic approx.
fn resolve_topology(
    cli: &Cli,
    ir: &FeatureIr,
) -> Result<(TopologySnapshot, &'static str), (ExitCode, serde_json::Value)> {
    match cli.kernel {
        crate::cli::KernelId::Mock
        | crate::cli::KernelId::Truck
        | crate::cli::KernelId::TruckBrep => topology_from_ir(ir)
            .map(|s| {
                (
                    s,
                    if matches!(cli.kernel, crate::cli::KernelId::TruckBrep) {
                        "truck-brep-ir-fallback"
                    } else if matches!(cli.kernel, crate::cli::KernelId::Truck) {
                        "truck-analytic-nonparity"
                    } else {
                        "ir-analytic"
                    },
                )
            })
            .map_err(|m| {
                (
                    ExitCode::Internal,
                    json!({"ok": false, "diagnostics": [{"code": "CADRE-E-TOPO", "message": m}]}),
                )
            }),
        crate::cli::KernelId::Occt => {
            #[cfg(feature = "occt")]
            {
                let mut kb = open_kernel(cli.kernel)?;
                let sid = execute_ir(kb.as_mut(), ir).map_err(|e| {
                    (
                        ExitCode::Kernel,
                        json!({"ok": false, "diagnostics": [{"code": "CADRE-E-KERNEL", "message": e.to_string()}]}),
                    )
                })?;
                match &kb {
                    KernelBox::Occt(k) => k
                        .topology_snapshot(sid)
                        .map(|s| (s, "occt-brep"))
                        .map_err(|e| {
                            (
                                ExitCode::Kernel,
                                json!({"ok": false, "diagnostics": [{"code": "CADRE-E-TOPO", "message": e.to_string()}]}),
                            )
                        }),
                    _ => unreachable!("occt kernel box"),
                }
            }
            #[cfg(not(feature = "occt"))]
            {
                open_kernel(cli.kernel).map(|_| unreachable!())
            }
        }
    }
}

fn refs(cli: &Cli, target: std::path::PathBuf, facts: bool, sets: &[String]) -> ExitCode {
    let ir = match load_ir(&target, sets) {
        Ok(ir) => ir,
        Err((c, v)) => {
            emit(cli.json, &v, false);
            return c;
        }
    };
    let (snap, source_kind) = match resolve_topology(cli, &ir) {
        Ok(x) => x,
        Err((c, v)) => {
            emit(cli.json, &v, false);
            return c;
        }
    };
    let report = inspect_refs(&snap, facts);
    let body = json!({
        "ok": true,
        "object": report.object,
        "solids": report.solids,
        "faces": report.faces,
        "edges": report.edges,
        "refs": report.refs,
        "facts": report.facts,
        "meta": {
            "source": target,
            "topology": source_kind,
            "kernel": match cli.kernel {
                crate::cli::KernelId::Mock => "mock",
                crate::cli::KernelId::Occt => "occt",
                crate::cli::KernelId::Truck => "truck",
                crate::cli::KernelId::TruckBrep => "truck-brep",
            },
        },
    });
    emit(cli.json, &body, true);
    ExitCode::Ok
}

fn measure_cmd(
    cli: &Cli,
    target: std::path::PathBuf,
    a: String,
    b: Option<String>,
    kind: MeasureKindArg,
    sets: &[String],
) -> ExitCode {
    let ir = match load_ir(&target, sets) {
        Ok(ir) => ir,
        Err((c, v)) => {
            emit(cli.json, &v, false);
            return c;
        }
    };
    let (snap, source_kind) = match resolve_topology(cli, &ir) {
        Ok(x) => x,
        Err((c, v)) => {
            emit(cli.json, &v, false);
            return c;
        }
    };
    let kind = match kind {
        MeasureKindArg::Distance => MeasureKind::Distance,
        MeasureKindArg::Angle => MeasureKind::Angle,
        MeasureKindArg::Diameter => MeasureKind::Diameter,
        MeasureKindArg::Thickness => MeasureKind::Thickness,
    };
    match measure(&snap, &MeasureRequest { a, b, kind }) {
        Ok(r) => {
            let body = json!({
                "ok": true,
                "kind": r.kind,
                "value": r.value,
                "unit": r.unit,
                "construction": r.construction,
                "a": r.a,
                "b": r.b,
                "meta": { "topology": source_kind },
            });
            emit(cli.json, &body, true);
            ExitCode::Ok
        }
        Err(e) => {
            let v = json!({
                "ok": false,
                "diagnostics": [{
                    "code": "CADRE-E-MEASURE",
                    "severity": "error",
                    "message": e.to_string(),
                }]
            });
            emit(cli.json, &v, false);
            ExitCode::Validation
        }
    }
}

fn load_topo(
    cli: &Cli,
    target: &std::path::Path,
    sets: &[String],
) -> Result<(TopologySnapshot, &'static str), (ExitCode, serde_json::Value)> {
    let ir = load_ir(target, sets)?;
    resolve_topology(cli, &ir)
}

fn align_cmd(cli: &Cli, a: &crate::cli::AlignArgs) -> ExitCode {
    let (snap, source_kind) = match load_topo(cli, &a.target, &a.set) {
        Ok(x) => x,
        Err((c, v)) => {
            emit(cli.json, &v, false);
            return c;
        }
    };
    let expect = match a.expect {
        AlignExpectArg::Coplanar => AlignExpect::Coplanar,
        AlignExpectArg::Coaxial => AlignExpect::Coaxial,
        AlignExpectArg::Distance => AlignExpect::Distance,
    };
    match align_refs(&snap, &a.a, &a.b, expect, a.distance, a.tol, a.tol_deg) {
        Ok(r) => {
            emit(
                cli.json,
                &json!({"ok": r.ok, "align": r, "meta": {"topology": source_kind}}),
                r.ok,
            );
            if r.ok {
                ExitCode::Ok
            } else {
                ExitCode::Validation
            }
        }
        Err(e) => {
            emit(
                cli.json,
                &json!({"ok": false, "diagnostics":[{"code":"CADRE-E-ALIGN","message": e.to_string()}]}),
                false,
            );
            ExitCode::Validation
        }
    }
}

fn frame_cmd(cli: &Cli, a: &crate::cli::FrameArgs) -> ExitCode {
    let (snap, source_kind) = match load_topo(cli, &a.target, &a.set) {
        Ok(x) => x,
        Err((c, v)) => {
            emit(cli.json, &v, false);
            return c;
        }
    };
    match frame_of(&snap, &a.selector) {
        Ok(r) => {
            emit(
                cli.json,
                &json!({"ok": true, "frame": r, "meta": {"topology": source_kind}}),
                true,
            );
            ExitCode::Ok
        }
        Err(e) => {
            emit(
                cli.json,
                &json!({"ok": false, "diagnostics":[{"code":"CADRE-E-FRAME","message": e.to_string()}]}),
                false,
            );
            ExitCode::Validation
        }
    }
}

fn diff_cmd(cli: &Cli, a: &crate::cli::DiffArgs) -> ExitCode {
    let (old, _) = match load_topo(cli, &a.old, &a.set_old) {
        Ok(x) => x,
        Err((c, v)) => {
            emit(cli.json, &v, false);
            return c;
        }
    };
    let (new, _) = match load_topo(cli, &a.new, &a.set_new) {
        Ok(x) => x,
        Err((c, v)) => {
            emit(cli.json, &v, false);
            return c;
        }
    };
    let report = diff_snapshots(&old, &new);
    emit(
        cli.json,
        &json!({
            "ok": true,
            "diff": report,
            "old": a.old,
            "new": a.new,
        }),
        true,
    );
    ExitCode::Ok
}

fn dims_cmd(cli: &Cli, a: &crate::cli::DimsArgs) -> ExitCode {
    let (snap, topology) = match load_topo(cli, &a.target, &a.set) {
        Ok(x) => x,
        Err((c, v)) => {
            emit(cli.json, &v, false);
            return c;
        }
    };
    let mut specs: Vec<DimSpec> = Vec::new();
    if let Some(path) = &a.specs {
        match fs::read_to_string(path) {
            Ok(t) => match serde_json::from_str::<Vec<DimSpec>>(&t) {
                Ok(v) => specs.extend(v),
                Err(e) => {
                    emit(
                        cli.json,
                        &json!({"ok": false, "error": format!("specs json: {e}")}),
                        false,
                    );
                    return ExitCode::Validation;
                }
            },
            Err(e) => {
                emit(
                    cli.json,
                    &json!({"ok": false, "error": format!("read specs: {e}")}),
                    false,
                );
                return ExitCode::Io;
            }
        }
    }
    for s in &a.dim {
        match parse_dim_flag(s) {
            Ok(d) => specs.push(d),
            Err(e) => {
                emit(cli.json, &json!({"ok": false, "error": e}), false);
                return ExitCode::Usage;
            }
        }
    }
    let source = a
        .target
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("part")
        .to_string();
    let packet = build_drawing_packet(&snap, &source, topology, &specs);
    let out = a.output.clone().unwrap_or_else(|| {
        let stem = a
            .target
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("part");
        // strip .cad if stem is foo.cad
        let stem = stem.strip_suffix(".cad").unwrap_or(stem);
        a.target
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join(format!("{stem}.drawing.json"))
    });
    if let Err(e) = fs::write(
        &out,
        serde_json::to_string_pretty(&packet).unwrap_or_default(),
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
            "ok": packet.ok,
            "packet": packet,
            "out": out,
            "note": "not a drafting package — dimension facts only (H2-8 PMI alpha)",
        }),
        packet.ok,
    );
    if packet.ok {
        ExitCode::Ok
    } else {
        ExitCode::Validation
    }
}

/// Parse `A,B,kind` or `A,kind` (diameter).
fn parse_dim_flag(s: &str) -> Result<DimSpec, String> {
    let parts: Vec<&str> = s
        .split(',')
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .collect();
    match parts.as_slice() {
        [a, kind] => Ok(DimSpec {
            a: (*a).into(),
            b: None,
            kind: (*kind).into(),
            label: None,
        }),
        [a, b, kind] => Ok(DimSpec {
            a: (*a).into(),
            b: Some((*b).into()),
            kind: (*kind).into(),
            label: None,
        }),
        [a, b, kind, label] => Ok(DimSpec {
            a: (*a).into(),
            b: Some((*b).into()),
            kind: (*kind).into(),
            label: Some((*label).into()),
        }),
        _ => Err(format!(
            "bad --dim '{s}' (want A,B,kind or A,kind for diameter)"
        )),
    }
}
