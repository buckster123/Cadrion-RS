//! Generate expect.occt.json for parts 05–10 (ignored; local OCCT only).
#![cfg(feature = "occt")]

use std::fs;

use cadrion_bench::default_parity_root;
use cadrion_kernel::GeomKernel;
use cadrion_lang::{evaluate, execute_ir, EvalOptions, IrNode};
use cadrion_occt::OcctKernel;

#[test]
#[ignore]
fn write_expects_occt_5_10() {
    let root = default_parity_root().join("parts");
    let names = [
        "05_open_enclosure",
        "06_clevis_bracket",
        "07_finned_cylinder",
        "08_impeller",
        "09_spiral_stair",
        "10_planetary_stage",
    ];
    for name in names {
        let dir = root.join(name);
        let star = dir.join("part.cad.star");
        let src = fs::read_to_string(&star).unwrap();
        let r = evaluate(&src, &EvalOptions::new("p.cad.star"));
        assert!(r.ok, "{name} {:?}", r.diagnostics);
        let ir = r.ir.unwrap();
        let mut k = OcctKernel::new();
        let sid = execute_ir(&mut k, &ir).unwrap_or_else(|e| panic!("{name} exec: {e}"));
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
            "description": format!("OCCT tessellation golden for {name}"),
            "volume_mm3": f.volume_mm3,
            "volume_tol_frac": 0.12,
            "bbox_mm": {
                "min": [f.bbox_mm.min.x, f.bbox_mm.min.y, f.bbox_mm.min.z],
                "max": [f.bbox_mm.max.x, f.bbox_mm.max.y, f.bbox_mm.max.z]
            },
            "bbox_tol_mm": 2.5,
            "required_ops": ops,
            "params": ir.params,
            "faces_min": 1,
            "measures": []
        });
        let out = dir.join("expect.occt.json");
        fs::write(&out, serde_json::to_string_pretty(&expect).unwrap() + "\n").unwrap();
        eprintln!("wrote {} vol={}", out.display(), f.volume_mm3);
    }
}
