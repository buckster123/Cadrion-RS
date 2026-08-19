//! `cadrion build`

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use cadrion_kernel::StepWriteOpts;
use cadrion_lang::{evaluate, execute_ir, EvalOptions, FeatureIr};
use cadrion_model::{BuildCache, CacheKey};
use serde_json::{json, Value};

use crate::cli::{BuildArgs, Cli};
use crate::kernel_pick::open_kernel;
use crate::output::{emit, ExitCode};

pub fn run(cli: &Cli, args: &BuildArgs) -> ExitCode {
    let started = Instant::now();
    let target = match resolve_target(&args.target) {
        Ok(p) => p,
        Err(v) => {
            emit(cli.json, &v, false);
            return ExitCode::Usage;
        }
    };

    if target.is_dir() {
        let v = json!({
            "ok": false,
            "diagnostics": [{
                "code": "CADRION-E-EXPLICIT-TARGET",
                "severity": "error",
                "message": "directory-wide builds are refused",
                "target": target,
                "hint": "pass a single .cad.star file"
            }]
        });
        emit(cli.json, &v, false);
        return ExitCode::Usage;
    }

    let source = match fs::read_to_string(&target) {
        Ok(s) => s,
        Err(e) => {
            let v = json!({"ok": false, "diagnostics": [{"code": "CADRION-E-IO", "message": e.to_string(), "target": target}]});
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
    let params_json = serde_json::to_string(&overrides).unwrap_or_else(|_| "{}".into());

    let mut kernel = match open_kernel(cli.kernel) {
        Ok(k) => k,
        Err((code, v)) => {
            emit(cli.json, &v, false);
            return code;
        }
    };

    let project = cli
        .project
        .clone()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    let cache_root = project.join(".cadrion").join("cache");
    let cache = BuildCache::open(&cache_root).ok();

    let cache_key = CacheKey::from_source(
        &source,
        &params_json,
        env!("CARGO_PKG_VERSION"),
        kernel.id(),
        kernel.version(),
        Some(cadrion_lang::IR_VERSION),
    );

    if !args.no_cache {
        if let Some(cache) = &cache {
            if let Ok(Some(hit)) = cache.get(&cache_key) {
                let art = cache.artifact_abs(&hit);
                let body = json!({
                    "ok": true,
                    "artifacts": [{
                        "path": art,
                        "kind": kind_from_path(&art),
                        "sha256": hit.artifact_sha256,
                    }],
                    "facts": hit.facts_json.as_ref().and_then(|s| serde_json::from_str::<Value>(s).ok()),
                    "diagnostics": [],
                    "cache": {"hit": true, "key": hit.key_digest},
                    "meta": {
                        "source": target,
                        "kernel": kernel.id(),
                        "wall_ms": started.elapsed().as_millis() as u64,
                        "cadrion_version": env!("CARGO_PKG_VERSION"),
                    }
                });
                emit(cli.json, &body, true);
                return ExitCode::Ok;
            }
        }
    }

    let name = target
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("model.cad.star")
        .to_string();
    let mut opts = EvalOptions::new(name);
    opts.overrides = overrides;

    let eval = evaluate(&source, &opts);
    if !eval.ok {
        let body = json!({
            "ok": false,
            "artifacts": [],
            "diagnostics": eval.diagnostics,
            "cache": {"hit": false},
            "meta": {
                "source": target,
                "kernel": kernel.id(),
                "wall_ms": started.elapsed().as_millis() as u64,
            }
        });
        emit(cli.json, &body, false);
        return ExitCode::Eval;
    }
    let ir = eval.ir.expect("ok implies ir");

    let shape = match execute_ir(kernel.as_mut(), &ir) {
        Ok(s) => s,
        Err(e) => {
            let body = json!({
                "ok": false,
                "artifacts": [],
                "diagnostics": [{
                    "code": e.code(),
                    "severity": "error",
                    "message": e.to_string(),
                    "target": target,
                }],
                "cache": {"hit": false},
                "meta": {
                    "source": target,
                    "kernel": kernel.id(),
                    "wall_ms": started.elapsed().as_millis() as u64,
                }
            });
            emit(cli.json, &body, false);
            return ExitCode::Kernel;
        }
    };

    let facts = kernel.as_mut().facts(shape).ok();
    let validity = kernel.as_mut().validity(shape).ok();

    let stem = strip_model_suffix(&target);
    let default_step = PathBuf::from(format!("{}.step", stem.display()));
    let mut artifacts = Vec::new();
    let mut diagnostics = Vec::new();

    // Always persist IR next to source (source of truth companion).
    let ir_json = serde_json::to_string_pretty(&ir).unwrap_or_else(|_| "{}".into());
    let ir_out = PathBuf::from(format!("{}.ir.json", stem.display()));
    if let Err(e) = fs::write(&ir_out, &ir_json) {
        diagnostics.push(json!({
            "code": "CADRION-E-IO",
            "severity": "error",
            "message": format!("write IR: {e}"),
        }));
        let body = json!({"ok": false, "artifacts": [], "diagnostics": diagnostics});
        emit(cli.json, &body, false);
        return ExitCode::Io;
    }
    artifacts.push(json!({
        "path": ir_out,
        "kind": "ir",
        "sha256": cadrion_model::sha256_hex(&ir_json),
    }));

    // STEP (or honest degrade)
    let step_out = args
        .output
        .clone()
        .filter(|p| p.extension().and_then(|e| e.to_str()) != Some("json"))
        .unwrap_or(default_step);

    let step_ok = match kernel.as_mut().write_step(
        shape,
        &step_out,
        &StepWriteOpts {
            name: ir.label.clone(),
            ..StepWriteOpts::default()
        },
    ) {
        Ok(()) => true,
        Err(e) => {
            diagnostics.push(json!({
                "code": e.code(),
                "severity": if kernel.id() == "mock" { "warning" } else { "error" },
                "message": e.to_string(),
                "hint": if kernel.id() == "mock" {
                    "mock kernel cannot write STEP; IR artifact written. Use --kernel occt (binary built with --features occt) for STEP."
                } else {
                    "kernel STEP write failed"
                },
            }));
            false
        }
    };

    let mut primary_bytes: Option<Vec<u8>> = None;
    let mut primary_name = "part.ir.json";
    if step_ok {
        match fs::read(&step_out) {
            Ok(b) => {
                let hash = cadrion_model::sha256_bytes(&b);
                artifacts.push(json!({
                    "path": step_out,
                    "kind": "step",
                    "sha256": hash,
                }));
                primary_bytes = Some(b);
                primary_name = "part.step";
            }
            Err(e) => {
                diagnostics.push(json!({"code": "CADRION-E-IO", "message": e.to_string()}));
            }
        }
    }

    // Cache put (prefer STEP bytes, else IR)
    let mut cache_key_digest = None;
    if let Some(cache) = &cache {
        let bytes = primary_bytes
            .clone()
            .unwrap_or_else(|| ir_json.as_bytes().to_vec());
        let name = if primary_bytes.is_some() {
            primary_name
        } else {
            "part.ir.json"
        };
        if let Ok(entry) = cache.put(
            &cache_key,
            &bytes,
            name,
            Some(&ir_json),
            facts.as_ref().and_then(|f| serde_json::to_string(f).ok()),
        ) {
            cache_key_digest = Some(entry.key_digest);
        }
    }

    let ok = step_ok || kernel.id() == "mock"; // mock may succeed with IR-only
    let body = json!({
        "ok": ok,
        "artifacts": artifacts,
        "facts": facts,
        "validity": validity,
        "diagnostics": diagnostics,
        "cache": {
            "hit": false,
            "key": cache_key_digest,
            "stored": cache_key_digest.is_some(),
        },
        "ir": summarize_ir(&ir),
        "meta": {
            "source": target,
            "kernel": kernel.id(),
            "kernel_version": kernel.version(),
            "params": opts.overrides,
            "wall_ms": started.elapsed().as_millis() as u64,
            "cadrion_version": env!("CARGO_PKG_VERSION"),
        }
    });
    emit(cli.json, &body, ok);
    if ok {
        ExitCode::Ok
    } else {
        ExitCode::Kernel
    }
}

fn summarize_ir(ir: &FeatureIr) -> serde_json::Value {
    json!({
        "version": ir.version,
        "node_count": ir.node_count(),
        "root": ir.root.0,
        "label": ir.label,
        "params": ir.params,
    })
}

fn resolve_target(p: &Path) -> Result<PathBuf, serde_json::Value> {
    if !p.exists() {
        return Err(json!({
            "ok": false,
            "diagnostics": [{
                "code": "CADRION-E-IO",
                "message": format!("target not found: {}", p.display()),
            }]
        }));
    }
    Ok(p.canonicalize().unwrap_or_else(|_| p.to_path_buf()))
}

pub fn parse_sets(sets: &[String]) -> Result<BTreeMap<String, f64>, String> {
    let mut m = BTreeMap::new();
    for s in sets {
        let (k, v) = s
            .split_once('=')
            .ok_or_else(|| format!("--set expects KEY=VAL, got {s}"))?;
        let n: f64 = v
            .parse()
            .map_err(|_| format!("--set value not a number: {v}"))?;
        m.insert(k.to_string(), n);
    }
    Ok(m)
}

fn kind_from_path(p: &Path) -> &'static str {
    match p.extension().and_then(|e| e.to_str()) {
        Some("step") | Some("stp") => "step",
        Some("json") => "ir",
        Some("stl") => "stl",
        Some("glb") => "glb",
        _ => "artifact",
    }
}

/// `foo.cad.star` → `foo` (not `foo.cad`). Plain `foo.star` → `foo`.
fn strip_model_suffix(path: &Path) -> PathBuf {
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
