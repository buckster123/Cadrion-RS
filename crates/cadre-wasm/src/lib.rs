//! Cadre WASM surface (H2-1) — portable IR + mock facts.
//!
//! **Honesty:** mock kernel only; no OCCT; not parity-eligible; not a multi-tenant host.
//! Build: `cargo build -p cadre-wasm --target wasm32-unknown-unknown --features browser`

#![deny(unsafe_code)]

use std::collections::BTreeMap;

use cadre_inspect::{box_topology, cylinder_topology, inspect_refs, SolidRec, TopologySnapshot};
use cadre_kernel::{GeomKernel, MockKernel, Point3};
use cadre_lang::{evaluate, execute_ir, BooleanKind, EvalOptions, FeatureIr, IrNode};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Crate version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Serialize, Deserialize)]
pub struct WasmBuildRequest {
    /// Starlark source (`.cad.star` body).
    pub source: String,
    /// Optional param overrides.
    #[serde(default)]
    pub set: BTreeMap<String, f64>,
    /// Filename for diagnostics (default part.cad.star).
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WasmIrRequest {
    /// Feature IR JSON (same shape as `.ir.json`).
    pub ir: FeatureIr,
}

/// Evaluate Starlark → IR + mock facts (JSON value).
pub fn build_json(req: &WasmBuildRequest) -> Value {
    let name = req.name.clone().unwrap_or_else(|| "part.cad.star".into());
    let mut opts = EvalOptions::new(name);
    opts.overrides = req.set.clone();
    let eval = evaluate(&req.source, &opts);
    if !eval.ok {
        return json!({
            "ok": false,
            "stage": "evaluate",
            "diagnostics": eval.diagnostics,
            "kernel": "mock",
            "parity_eligible": false,
            "wasm": true,
            "cadre_wasm": VERSION,
        });
    }
    let ir = eval.ir.expect("ir when ok");
    match facts_from_ir(&ir) {
        Ok(facts) => json!({
            "ok": true,
            "stage": "build",
            "label": ir.label,
            "params": ir.params,
            "node_count": ir.node_count(),
            "ir": ir,
            "facts": facts,
            "kernel": "mock",
            "parity_eligible": false,
            "wasm": true,
            "cadre_wasm": VERSION,
            "note": "WASM mock-only — STEP/OCCT unavailable in this target",
        }),
        Err(e) => json!({
            "ok": false,
            "stage": "execute",
            "error": e,
            "ir": ir,
            "kernel": "mock",
            "parity_eligible": false,
            "wasm": true,
            "cadre_wasm": VERSION,
        }),
    }
}

/// Execute pre-built IR on mock kernel → facts.
pub fn facts_ir_json(req: &WasmIrRequest) -> Value {
    match facts_from_ir(&req.ir) {
        Ok(facts) => json!({
            "ok": true,
            "stage": "facts_ir",
            "label": req.ir.label,
            "facts": facts,
            "kernel": "mock",
            "parity_eligible": false,
            "wasm": true,
            "cadre_wasm": VERSION,
        }),
        Err(e) => json!({
            "ok": false,
            "stage": "facts_ir",
            "error": e,
            "kernel": "mock",
            "parity_eligible": false,
            "wasm": true,
            "cadre_wasm": VERSION,
        }),
    }
}

/// H3-9: IR → analytic inspect_refs (selectors). Mock topology only.
pub fn inspect_ir_json(req: &WasmIrRequest) -> Value {
    match topology_from_ir(&req.ir) {
        Ok(snap) => {
            let report = inspect_refs(&snap, true);
            json!({
                "ok": true,
                "stage": "inspect_ir",
                "label": req.ir.label,
                "refs": report,
                "topology": "ir-analytic",
                "kernel": "mock",
                "parity_eligible": false,
                "wasm": true,
                "cadre_wasm": VERSION,
                "note": "H3-9 WASM inspect — IR-analytic selectors, not OCCT B-rep",
            })
        }
        Err(e) => json!({
            "ok": false,
            "stage": "inspect_ir",
            "error": e,
            "kernel": "mock",
            "parity_eligible": false,
            "wasm": true,
            "cadre_wasm": VERSION,
        }),
    }
}

fn topology_from_ir(ir: &FeatureIr) -> Result<TopologySnapshot, String> {
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
                cylinder_topology(*radius, *height, Point3::new(at[0], at[1], at[2]))
            }
            IrNode::Boolean { kind, a, b } => {
                let sa = solids
                    .get(a.0 as usize)
                    .and_then(|s| s.as_ref())
                    .ok_or_else(|| format!("IR node {} missing", a.0))?;
                let sb = solids
                    .get(b.0 as usize)
                    .and_then(|s| s.as_ref())
                    .ok_or_else(|| format!("IR node {} missing", b.0))?;
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
                .cloned()
                .ok_or_else(|| format!("IR node {} missing", of.0))?,
        };
        solids[idx] = Some(rec);
    }
    let root = solids
        .get(ir.root.0 as usize)
        .and_then(|s| s.as_ref())
        .cloned()
        .ok_or_else(|| "IR root missing".to_string())?;
    Ok(TopologySnapshot::single_solid(root))
}

fn facts_from_ir(ir: &FeatureIr) -> Result<Value, String> {
    let mut k = MockKernel::new();
    let sid = execute_ir(&mut k, ir).map_err(|e| e.to_string())?;
    let facts = k.facts(sid).map_err(|e| e.to_string())?;
    serde_json::to_value(facts).map_err(|e| e.to_string())
}

/// Library version / capability probe.
pub fn info_json() -> Value {
    json!({
        "ok": true,
        "cadre_wasm": VERSION,
        "kernel": "mock",
        "parity_eligible": false,
        "occt": false,
        "apis": ["build", "facts_ir", "inspect_ir", "info"],
        "note": "experimental WASM escape hatch (H2-1/H3-9); native CLI remains primary"
    })
}

// --- wasm-bindgen surface (browser feature) ---------------------------------

#[cfg(feature = "browser")]
mod browser {
    use super::*;
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    pub fn info() -> String {
        info_json().to_string()
    }

    /// JSON in → JSON out. See `WasmBuildRequest`.
    #[wasm_bindgen]
    pub fn build(req_json: &str) -> String {
        match serde_json::from_str::<WasmBuildRequest>(req_json) {
            Ok(req) => build_json(&req).to_string(),
            Err(e) => json!({"ok": false, "error": format!("bad request: {e}")}).to_string(),
        }
    }

    /// JSON IR in → facts out. See `WasmIrRequest`.
    #[wasm_bindgen]
    pub fn facts_ir(req_json: &str) -> String {
        match serde_json::from_str::<WasmIrRequest>(req_json) {
            Ok(req) => facts_ir_json(&req).to_string(),
            Err(e) => json!({"ok": false, "error": format!("bad request: {e}")}).to_string(),
        }
    }

    /// JSON IR in → inspect_refs (H3-9).
    #[wasm_bindgen]
    pub fn inspect_ir(req_json: &str) -> String {
        match serde_json::from_str::<WasmIrRequest>(req_json) {
            Ok(req) => inspect_ir_json(&req).to_string(),
            Err(e) => json!({"ok": false, "error": format!("bad request: {e}")}).to_string(),
        }
    }
}

#[cfg(feature = "browser")]
pub use browser::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_box() {
        let src = r#"
P = params(w=40.0, d=20.0, h=10.0)
def gen_step():
    return solid(box(P.w, P.d, P.h, at=CENTER), label="block")
"#;
        let v = build_json(&WasmBuildRequest {
            source: src.into(),
            set: BTreeMap::new(),
            name: Some("t.cad.star".into()),
        });
        assert_eq!(v["ok"], true, "{v}");
        assert_eq!(v["kernel"], "mock");
        assert_eq!(v["parity_eligible"], false);
        assert!(v["facts"]["volume_mm3"].as_f64().unwrap() > 0.0);
    }

    #[test]
    fn info_ok() {
        let v = info_json();
        assert_eq!(v["ok"], true);
        assert_eq!(v["occt"], false);
        let apis = v["apis"].as_array().unwrap();
        assert!(apis.iter().any(|a| a.as_str() == Some("inspect_ir")));
    }

    #[test]
    fn inspect_ir_box_has_refs() {
        let src = r#"
def gen_step():
    return solid(box(40.0, 20.0, 10.0, at=CENTER), label="block")
"#;
        let built = build_json(&WasmBuildRequest {
            source: src.into(),
            set: BTreeMap::new(),
            name: Some("t.cad.star".into()),
        });
        assert_eq!(built["ok"], true, "{built}");
        let ir: FeatureIr = serde_json::from_value(built["ir"].clone()).unwrap();
        let v = inspect_ir_json(&WasmIrRequest { ir });
        assert_eq!(v["ok"], true, "{v}");
        assert_eq!(v["stage"], "inspect_ir");
        let n = v["refs"]["refs"].as_array().map(|a| a.len()).unwrap_or(0);
        assert!(n > 0, "expected selector refs: {v}");
    }
}
