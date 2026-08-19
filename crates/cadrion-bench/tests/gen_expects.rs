use std::fs;

use cadrion_bench::default_parity_root;
use cadrion_kernel::{GeomKernel, MockKernel};
use cadrion_lang::{evaluate, execute_ir, EvalOptions, IrNode};

#[test]
#[ignore]
fn write_expects_5_10() {
    let root = default_parity_root().join("parts");
    for e in fs::read_dir(&root).unwrap() {
        let dir = e.unwrap().path();
        if !dir.is_dir() {
            continue;
        }
        let name = dir.file_name().unwrap().to_string_lossy().to_string();
        if !(name.starts_with("05")
            || name.starts_with("06")
            || name.starts_with("07")
            || name.starts_with("08")
            || name.starts_with("09")
            || name.starts_with("10"))
        {
            continue;
        }
        let star = dir.join("part.cad.star");
        let src = fs::read_to_string(&star).unwrap();
        let r = evaluate(&src, &EvalOptions::new("p.cad.star"));
        assert!(r.ok, "{name} {:?}", r.diagnostics);
        let ir = r.ir.unwrap();
        let mut k = MockKernel::new();
        let sid = execute_ir(&mut k, &ir).unwrap();
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
            if !ops.contains(&key.to_string()) {
                ops.push(key.to_string());
            }
        }
        let expect = serde_json::json!({
            "id": name,
            "label": ir.label,
            "description": format!("mock golden for {name}"),
            "volume_mm3": f.volume_mm3,
            "volume_tol_frac": 0.02,
            "bbox_mm": {
                "min": [f.bbox_mm.min.x, f.bbox_mm.min.y, f.bbox_mm.min.z],
                "max": [f.bbox_mm.max.x, f.bbox_mm.max.y, f.bbox_mm.max.z]
            },
            "bbox_tol_mm": 1.5,
            "required_ops": ops,
            "params": ir.params,
            "faces_min": 1,
            "measures": []
        });
        let out = dir.join("expect.json");
        fs::write(&out, serde_json::to_string_pretty(&expect).unwrap() + "\n").unwrap();
        eprintln!("wrote {} vol={}", out.display(), f.volume_mm3);
    }
}
