//! Derive analytic TopologySnapshot from simple IR (box/cylinder/label/boolean approx).

use cadrion_inspect::{box_topology, cylinder_topology, SolidRec, TopologySnapshot};
use cadrion_kernel::Point3;
use cadrion_lang::{BooleanKind, FeatureIr, IrNode, NodeId};

/// Best-effort topology from IR for inspect without OCCT.
/// Booleans use volume/bbox approximations (same honesty as MockKernel).
pub fn topology_from_ir(ir: &FeatureIr) -> Result<TopologySnapshot, String> {
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
                let sa = lookup(&solids, *a)?;
                let sb = lookup(&solids, *b)?;
                approx_boolean(*kind, sa, sb)
            }
            IrNode::Fillet { of, .. }
            | IrNode::Chamfer { of, .. }
            | IrNode::Label { of, .. }
            | IrNode::Translate { of, .. }
            | IrNode::Rotate { of, .. }
            | IrNode::Mirror { of, .. } => {
                // Pass-through geometry approx (fillet/chamfer change volume slightly — not modeled here)
                lookup(&solids, *of)?.clone()
            }
        };
        solids[idx] = Some(rec);
    }

    let root = lookup(&solids, ir.root)?.clone();
    Ok(TopologySnapshot::single_solid(root))
}

fn lookup(solids: &[Option<SolidRec>], id: NodeId) -> Result<&SolidRec, String> {
    solids
        .get(id.0 as usize)
        .and_then(|s| s.as_ref())
        .ok_or_else(|| format!("IR node {} missing in topology lower", id.0))
}

fn approx_boolean(kind: BooleanKind, a: &SolidRec, b: &SolidRec) -> SolidRec {
    let volume = match kind {
        BooleanKind::Union => a.volume_mm3 + b.volume_mm3,
        BooleanKind::Cut => (a.volume_mm3 - b.volume_mm3).max(0.0),
        BooleanKind::Intersect => a.volume_mm3.min(b.volume_mm3),
    };
    // Keep A's face/edge set as a coarse stand-in (inspect still gets stable tokens on A).
    SolidRec {
        volume_mm3: volume,
        centroid: a.centroid,
        faces: a.faces.clone(),
        edges: a.edges.clone(),
        vertices: a.vertices.clone(),
    }
}
