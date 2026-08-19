//! Hermetic Starlark evaluation → [`FeatureIr`].

use std::collections::BTreeMap;
use std::time::Instant;

use crate::diagnostic::{diagnostic_from_error, Diagnostic};
use crate::ir::{FeatureIr, NodeId};
use crate::stdlib::{register_stdlib, with_store, EvalStore};
use serde::{Deserialize, Serialize};
use starlark::environment::{Globals, GlobalsBuilder, Module};
use starlark::eval::Evaluator;
use starlark::syntax::{AstModule, Dialect};

/// Options for one evaluation.
#[derive(Debug, Clone, Default)]
pub struct EvalOptions {
    /// Logical file name (diagnostics + parse name).
    pub source_name: String,
    /// `--set k=v` overrides applied on top of `params()`.
    pub overrides: BTreeMap<String, f64>,
}

impl EvalOptions {
    pub fn new(source_name: impl Into<String>) -> Self {
        Self {
            source_name: source_name.into(),
            overrides: BTreeMap::new(),
        }
    }

    pub fn with_override(mut self, key: impl Into<String>, value: f64) -> Self {
        self.overrides.insert(key.into(), value);
        self
    }
}

/// Successful or failed evaluation payload (CLI `--json` shape).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvalResult {
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ir: Option<FeatureIr>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<Diagnostic>,
    pub meta: EvalMeta,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvalMeta {
    pub source_name: String,
    pub wall_ms: u64,
    pub ir_version: u32,
    pub node_count: usize,
}

/// Evaluate Cadrion Starlark source and produce feature IR.
///
/// Hermetic guarantees for this host:
/// - no `load()` loader installed (imports fail)
/// - load() AST refs refused before eval
/// - no ambient clock/env/fs APIs exposed in stdlib
/// - `print` discarded
/// - param overrides are the only host→model input channel
pub fn evaluate(source: &str, opts: &EvalOptions) -> EvalResult {
    let start = Instant::now();
    let name = if opts.source_name.is_empty() {
        "model.cad.star".to_string()
    } else {
        opts.source_name.clone()
    };

    match evaluate_inner(source, &name, &opts.overrides) {
        Ok(ir) => {
            let node_count = ir.node_count();
            EvalResult {
                ok: true,
                ir: Some(ir),
                diagnostics: Vec::new(),
                meta: EvalMeta {
                    source_name: name,
                    wall_ms: elapsed_ms(start),
                    ir_version: crate::ir::IR_VERSION,
                    node_count,
                },
            }
        }
        Err(diags) => EvalResult {
            ok: false,
            ir: None,
            diagnostics: diags,
            meta: EvalMeta {
                source_name: name,
                wall_ms: elapsed_ms(start),
                ir_version: crate::ir::IR_VERSION,
                node_count: 0,
            },
        },
    }
}

fn elapsed_ms(start: Instant) -> u64 {
    start.elapsed().as_millis() as u64
}

fn cadrion_globals() -> Globals {
    GlobalsBuilder::standard().with(register_stdlib).build()
}

fn evaluate_inner(
    source: &str,
    name: &str,
    overrides: &BTreeMap<String, f64>,
) -> Result<FeatureIr, Vec<Diagnostic>> {
    let dialect = Dialect::Extended;
    let ast = AstModule::parse(name, source.to_owned(), &dialect).map_err(|e| {
        vec![diagnostic_from_error(name, &e.to_string())
            .with_hint("fix Starlark syntax before build")]
    })?;

    {
        let loads = ast.loads();
        if !loads.is_empty() {
            let mods: Vec<_> = loads.iter().map(|l| l.module_id.to_string()).collect();
            return Err(vec![Diagnostic::error(
                "CADRION-E-HERMETIC-LOAD",
                format!("load() is disabled (hermetic model code); attempted: {mods:?}"),
            )
            .with_target(name)
            .with_hint("inline helpers or wait for sanctioned library modules")]);
        }
    }

    let globals = cadrion_globals();
    let store = EvalStore::new(overrides.clone());

    let (result, store) = with_store(store, || {
        Module::with_temp_heap(|module| -> Result<NodeId, Vec<Diagnostic>> {
            {
                let heap = module.heap();
                let center = heap.alloc((0.0, 0.0, 0.0));
                module.set("CENTER", center);
            }

            let gen_step = {
                let mut eval = Evaluator::new(&module);
                eval.eval_module(ast, &globals)
                    .map_err(|e| vec![diagnostic_from_error(name, &e.to_string())])?;

                module.get("gen_step").ok_or_else(|| {
                    vec![
                        Diagnostic::error("CADRION-E-NO-ENTRY", "module must define gen_step()")
                            .with_target(name)
                            .with_hint(
                                "add: def gen_step():\n    return solid(box(...), label=\"part\")",
                            ),
                    ]
                })?
            };

            let root_val = {
                let mut eval = Evaluator::new(&module);
                eval.eval_function(gen_step, &[], &[])
                    .map_err(|e| vec![diagnostic_from_error(name, &e.to_string())])?
            };

            let root_i = root_val.unpack_i32().ok_or_else(|| {
                vec![Diagnostic::error(
                    "CADRION-E-BAD-RETURN",
                    format!(
                        "gen_step() must return a shape id (int), got {}",
                        root_val.get_type()
                    ),
                )
                .with_target(name)
                .with_hint("return the result of box/cylinder/cut/solid(...)")]
            })?;
            if root_i < 0 {
                return Err(vec![Diagnostic::error(
                    "CADRION-E-BAD-RETURN",
                    format!("gen_step() returned negative shape id {root_i}"),
                )
                .with_target(name)]);
            }
            Ok(NodeId(root_i as u32))
        })
    });

    let root = result?;
    store
        .builder
        .finish(root)
        .map_err(|msg| vec![Diagnostic::error("CADRION-E-IR", msg).with_target(name)])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{BooleanKind, IrNode};

    const BLOCK: &str = r#"
P = params(width=100.0, depth=60.0, height=20.0, hole_d=8.0)

def gen_step():
    blk = box(P.width, P.depth, P.height, at=CENTER)
    hole = cylinder(P.hole_d / 2.0, P.height + 2.0, at=(0.0, 0.0, -1.0))
    body = cut(blk, hole)
    return solid(body, label="calibration_block")
"#;

    #[test]
    fn evaluates_block_to_ir() {
        let r = evaluate(BLOCK, &EvalOptions::new("block.cad.star"));
        assert!(r.ok, "{:?}", r.diagnostics);
        let ir = r.ir.unwrap();
        assert_eq!(ir.params.get("width"), Some(&100.0));
        assert_eq!(ir.label.as_deref(), Some("calibration_block"));
        assert!(ir.node_count() >= 4);
        assert!(matches!(
            ir.node(ir.root),
            Some(IrNode::Label { name, .. }) if name == "calibration_block"
        ));
        assert!(ir.nodes.iter().any(|n| matches!(
            n,
            IrNode::Boolean {
                kind: BooleanKind::Cut,
                ..
            }
        )));
    }

    #[test]
    fn overrides_win() {
        let opts = EvalOptions::new("block.cad.star").with_override("width", 120.0);
        let r = evaluate(BLOCK, &opts);
        assert!(r.ok, "{:?}", r.diagnostics);
        let ir = r.ir.unwrap();
        assert_eq!(ir.params.get("width"), Some(&120.0));
        let box_n = ir
            .nodes
            .iter()
            .find_map(|n| match n {
                IrNode::Box { dx, .. } => Some(*dx),
                _ => None,
            })
            .unwrap();
        assert!((box_n - 120.0).abs() < 1e-12);
    }

    #[test]
    fn missing_gen_step_is_structured() {
        let src = "P = params(x=1.0)\n";
        let r = evaluate(src, &EvalOptions::new("nope.cad.star"));
        assert!(!r.ok);
        assert_eq!(r.diagnostics[0].code, "CADRION-E-NO-ENTRY");
        let json = serde_json::to_string(&r).unwrap();
        assert!(json.contains("CADRION-E-NO-ENTRY"));
    }

    #[test]
    fn syntax_error_diagnostic() {
        let r = evaluate("def gen_step(:\n  pass\n", &EvalOptions::new("bad.star"));
        assert!(!r.ok);
        assert_eq!(r.diagnostics[0].code, "CADRION-E-EVAL");
    }

    #[test]
    fn load_refused() {
        let src = r#"
load("other.star", "x")
def gen_step():
    return box(1,1,1)
"#;
        let r = evaluate(src, &EvalOptions::new("load.cad.star"));
        assert!(!r.ok);
        assert_eq!(r.diagnostics[0].code, "CADRION-E-HERMETIC-LOAD");
    }

    #[test]
    fn list_comp_holes() {
        let src = r#"
def gen_step():
    blk = box(100.0, 60.0, 20.0, at=CENTER)
    holes = [cylinder(4.0, 22.0, at=(x, y, -1.0)) for x in (-40.0, 40.0) for y in (-20.0, 20.0)]
    body = cut(blk, union_all(holes))
    return solid(body, label="block")
"#;
        let r = evaluate(src, &EvalOptions::new("holes.cad.star"));
        assert!(r.ok, "{:?}", r.diagnostics);
        let ir = r.ir.unwrap();
        let cyls = ir
            .nodes
            .iter()
            .filter(|n| matches!(n, IrNode::Cylinder { .. }))
            .count();
        assert_eq!(cyls, 4);
    }

    #[test]
    fn result_json_shape() {
        let r = evaluate(BLOCK, &EvalOptions::new("block.cad.star"));
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(v["ok"], true);
        assert!(v["ir"]["nodes"].is_array());
        assert!(v["meta"]["wall_ms"].is_number());
    }

    /// Property: random-ish junk must not panic; may fail eval.
    #[test]
    fn property_junk_sources_do_not_panic() {
        let long = "x".repeat(10_000);
        let samples = [
            "",
            "\0\0\0",
            "def gen_step():\n  return 1/0\n",
            "load('x.star','y')\ndef gen_step():\n  pass\n",
            "def gen_step():\n  return solid(box(1,1,1))\n",
            long.as_str(),
            "def gen_step():\n  return solid(box(1e308, 1e308, 1e308))\n",
            "def gen_step():\n  return 1\n",
        ];
        for (i, src) in samples.iter().enumerate() {
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _ = evaluate(src, &EvalOptions::new(&format!("fuzz{i}.star")));
            }));
            assert!(r.is_ok(), "evaluate panicked on sample {i}");
        }
    }
}
