//! Generate expect.occt.json for H4 fillet parts (ignored; local OCCT).
#![cfg(feature = "occt")]

use std::fs;

use cadrion_bench::default_parity_root;
use cadrion_kernel::GeomKernel;
use cadrion_lang::{evaluate, execute_ir, EvalOptions, IrNode};
use cadrion_occt::OcctKernel;

#[test]
#[ignore]
fn write_expects_fillet_occt() {
    let root = default_parity_root().join("parts");
    for name in ["11_filleted_plate", "12_chamfered_brick", "13_filleted_l"] {
        let dir = root.join(name);
        let src = fs::read_to_string(dir.join("part.cad.star")).expect("star");
        let r = evaluate(&src, &EvalOptions::new("part.cad.star"));
        assert!(r.ok, "{name}: {:?}", r.diagnostics);
        let ir = r.ir.unwrap();
        let mut k = OcctKernel::new();
        let sid = execute_ir(&mut k, &ir).expect("exec");
        let f = k.facts(sid).unwrap();
        let mut ops = Vec::new();
        for n in &ir.nodes {
            let key = match n {
                IrNode::Box { .. } => "box",
                IrNode::Cylinder { .. } => "cylinder",
                IrNode::Sphere { .. } => "sphere",
                IrNode::Cone { .. } => "cone",
                IrNode::Boolean { .. } => "boolean",
                IrNode::Fillet { .. } => "fillet",
                IrNode::Chamfer { .. } => "chamfer",
                IrNode::Label { .. } => "label",
                IrNode::Translate { .. } => "translate",
                IrNode::Rotate { .. } => "rotate",
                IrNode::Mirror { .. } => "mirror",
            };
            if !ops.iter().any(|o| o == key) {
                ops.push(key.to_string());
            }
        }
        let expect = serde_json::json!({
            "id": name,
            "label": ir.label,
            "description": format!("OCCT golden (H4/H3-7 fillet) for {name}"),
            "volume_mm3": f.volume_mm3,
            "volume_tol_frac": 0.12,
            "bbox_mm": {
                "min": [f.bbox_mm.min.x, f.bbox_mm.min.y, f.bbox_mm.min.z],
                "max": [f.bbox_mm.max.x, f.bbox_mm.max.y, f.bbox_mm.max.z],
            },
            "bbox_tol_mm": 2.0,
            "faces_min": f.faces.saturating_sub(2),
            "edges_min": 1,
            "required_ops": ops,
            "params": ir.params,
        });
        let path = dir.join("expect.occt.json");
        fs::write(&path, serde_json::to_string_pretty(&expect).unwrap() + "\n").unwrap();
        eprintln!("wrote {} vol={}", path.display(), f.volume_mm3);
    }
}
