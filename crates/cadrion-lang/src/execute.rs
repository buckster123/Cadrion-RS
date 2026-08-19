//! Lower feature IR onto a [`GeomKernel`].

use cadrion_kernel::{
    BooleanOp, EdgeRef, GeomKernel, KernelError, KernelResult, Placement, Point3, ShapeId,
    ShapeLabel,
};

use crate::ir::{BooleanKind, FeatureIr, IrNode, NodeId};

/// Execute `ir` on `kernel`, returning the root [`ShapeId`].
pub fn execute_ir(kernel: &mut dyn GeomKernel, ir: &FeatureIr) -> KernelResult<ShapeId> {
    let mut map: Vec<Option<ShapeId>> = vec![None; ir.nodes.len()];

    for (idx, node) in ir.nodes.iter().enumerate() {
        let id = match node {
            IrNode::Box { dx, dy, dz, at } => kernel.box_solid(
                *dx,
                *dy,
                *dz,
                Placement::at(Point3::new(at[0], at[1], at[2])),
            )?,
            IrNode::Cylinder { radius, height, at } => kernel.cylinder(
                *radius,
                *height,
                Placement::at(Point3::new(at[0], at[1], at[2])),
            )?,
            IrNode::Sphere { radius, at } => {
                kernel.sphere(*radius, Placement::at(Point3::new(at[0], at[1], at[2])))?
            }
            IrNode::Cone { radius, height, at } => kernel.cone(
                *radius,
                *height,
                Placement::at(Point3::new(at[0], at[1], at[2])),
            )?,
            IrNode::Boolean { kind, a, b } => {
                let sa = lookup(&map, *a)?;
                let sb = lookup(&map, *b)?;
                let op = match kind {
                    BooleanKind::Union => BooleanOp::Union,
                    BooleanKind::Cut => BooleanOp::Cut,
                    BooleanKind::Intersect => BooleanOp::Intersect,
                };
                kernel.boolean(op, sa, sb)?
            }
            IrNode::Fillet { of, radius, edges } => {
                let s = lookup(&map, *of)?;
                let edge_refs: Vec<EdgeRef> = edges.iter().copied().map(EdgeRef).collect();
                kernel.fillet(s, &edge_refs, *radius)?
            }
            IrNode::Chamfer {
                of,
                distance,
                edges,
            } => {
                let s = lookup(&map, *of)?;
                let edge_refs: Vec<EdgeRef> = edges.iter().copied().map(EdgeRef).collect();
                kernel.chamfer(s, &edge_refs, *distance)?
            }
            IrNode::Label { of, name } => {
                let s = lookup(&map, *of)?;
                kernel.set_label(s, ShapeLabel::new(name.clone()))?
            }
            IrNode::Translate { of, by } => {
                let s = lookup(&map, *of)?;
                kernel.translate(s, by[0], by[1], by[2])?
            }
            IrNode::Rotate { of, axis, deg } => {
                let s = lookup(&map, *of)?;
                kernel.rotate_about_axis(s, axis, *deg)?
            }
            IrNode::Mirror { of, plane } => {
                let s = lookup(&map, *of)?;
                kernel.mirror_plane(s, plane)?
            }
        };
        map[idx] = Some(id);
    }

    lookup(&map, ir.root)
}

fn lookup(map: &[Option<ShapeId>], id: NodeId) -> KernelResult<ShapeId> {
    map.get(id.0 as usize).and_then(|s| *s).ok_or_else(|| {
        KernelError::diagnostic(
            "CADRION-E-IR-REF",
            format!("IR node {} not produced", id.0),
            Some("internal: execute_ir ordering bug or corrupt IR".into()),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{evaluate, EvalOptions};
    use cadrion_kernel::MockKernel;

    #[test]
    fn execute_box_on_mock() {
        let src = r#"
def gen_step():
    return solid(box(10.0, 20.0, 30.0, at=CENTER), label="b")
"#;
        let r = evaluate(src, &EvalOptions::new("t.cad.star"));
        assert!(r.ok, "{:?}", r.diagnostics);
        let ir = r.ir.unwrap();
        let mut k = MockKernel::new();
        let sid = execute_ir(&mut k, &ir).unwrap();
        let f = k.facts(sid).unwrap();
        assert!((f.volume_mm3 - 6000.0).abs() < 1e-9);
    }

    #[test]
    fn sphere_and_linear_pattern() {
        let src = r#"
def gen_step():
    s = sphere(10.0, at=CENTER)
    p = linear_pattern(box(5.0, 5.0, 5.0, at=CENTER), 3.0, 10.0, 0.0, 0.0)
    return solid(union(s, p), label="sp")
"#;
        let r = evaluate(src, &EvalOptions::new("t.cad.star"));
        assert!(r.ok, "{:?}", r.diagnostics);
        let ir = r.ir.unwrap();
        assert!(ir.nodes.iter().any(|n| matches!(n, IrNode::Sphere { .. })));
        let mut k = MockKernel::new();
        let sid = execute_ir(&mut k, &ir).unwrap();
        let f = k.facts(sid).unwrap();
        assert!(f.volume_mm3 > 4000.0, "vol {}", f.volume_mm3);
    }

    #[test]
    fn polar_and_mirror() {
        let src = r#"
def gen_step():
    fin = box(20.0, 2.0, 10.0, at=(15.0, 0.0, 5.0))
    body = polar_pattern(fin, 4.0)
    m = mirror(box(10.0, 10.0, 10.0, at=(20.0, 0.0, 0.0)), "yz")
    return solid(union(body, m), label="pm")
"#;
        let r = evaluate(src, &EvalOptions::new("t.cad.star"));
        assert!(r.ok, "{:?}", r.diagnostics);
        let ir = r.ir.unwrap();
        assert!(ir.nodes.iter().any(|n| matches!(n, IrNode::Mirror { .. })));
        let mut k = MockKernel::new();
        let sid = execute_ir(&mut k, &ir).unwrap();
        assert!(k.facts(sid).unwrap().volume_mm3 > 100.0);
    }
}
