//! Local frame for a face/edge/solid ref.

use cadrion_kernel::{Point3, Vec3};
use serde::{Deserialize, Serialize};

use crate::lookup::{lookup_in_report, LookupError};
use crate::refs::inspect_refs;
use crate::topology::TopologySnapshot;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameReport {
    pub selector: String,
    pub kind: String,
    pub origin_mm: Point3,
    /// Unit axes (x,y,z) in world space. For faces: z = normal, x/y tangent.
    pub x_axis: [f64; 3],
    pub y_axis: [f64; 3],
    pub z_axis: [f64; 3],
    pub construction: String,
}

#[derive(Debug, thiserror::Error)]
pub enum FrameError {
    #[error("{0}")]
    Lookup(#[from] LookupError),
    #[error("{0}")]
    Msg(String),
}

/// Build a local frame for a selector (prefer face normal as +Z).
pub fn frame_of(snap: &TopologySnapshot, selector: &str) -> Result<FrameReport, FrameError> {
    let report = inspect_refs(snap, false);
    let r = lookup_in_report(&report.refs, selector)?;
    let origin = r.centroid_mm;
    let (x, y, z, construction) = match r.kind.as_str() {
        "face" => {
            let n = r.normal.ok_or_else(|| {
                FrameError::Msg(format!("{} has no normal (non-planar?)", r.selector))
            })?;
            let z = normalize(n);
            let (x, y) = plane_basis(z);
            (
                x,
                y,
                z,
                format!("face frame: +Z = normal of {}", r.selector),
            )
        }
        "solid" | "object" => {
            let z = Vec3::Z;
            let x = Vec3::X;
            let y = Vec3::Y;
            (
                x,
                y,
                z,
                format!("world-aligned frame at centroid of {}", r.selector),
            )
        }
        "edge" => {
            // approximate: z along +Z if unknown direction
            let z = Vec3::Z;
            let (x, y) = plane_basis(z);
            (
                x,
                y,
                z,
                format!(
                    "edge frame at midpoint of {} (direction not stored; z=world+Z)",
                    r.selector
                ),
            )
        }
        other => {
            return Err(FrameError::Msg(format!(
                "frame not supported for kind {other}"
            )))
        }
    };
    Ok(FrameReport {
        selector: r.selector.clone(),
        kind: r.kind.clone(),
        origin_mm: origin,
        x_axis: [x.x, x.y, x.z],
        y_axis: [y.x, y.y, y.z],
        z_axis: [z.x, z.y, z.z],
        construction,
    })
}

fn normalize(v: Vec3) -> Vec3 {
    let l = (v.x * v.x + v.y * v.y + v.z * v.z).sqrt().max(1e-12);
    Vec3::new(v.x / l, v.y / l, v.z / l)
}

fn plane_basis(z: Vec3) -> (Vec3, Vec3) {
    let mut t = Vec3::new(0.0, 0.0, 1.0);
    if (z.x * t.x + z.y * t.y + z.z * t.z).abs() > 0.9 {
        t = Vec3::new(1.0, 0.0, 0.0);
    }
    // x = t × z
    let x = normalize(Vec3::new(
        t.y * z.z - t.z * z.y,
        t.z * z.x - t.x * z.z,
        t.x * z.y - t.y * z.x,
    ));
    // y = z × x
    let y = Vec3::new(
        z.y * x.z - z.z * x.y,
        z.z * x.x - z.x * x.z,
        z.x * x.y - z.y * x.x,
    );
    (x, y)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::topology::{box_topology, TopologySnapshot};
    use cadrion_kernel::Point3;

    #[test]
    fn top_face_frame_z_up() {
        let snap = TopologySnapshot::single_solid(box_topology(10.0, 20.0, 30.0, Point3::ORIGIN));
        let report = inspect_refs(&snap, false);
        let top = report
            .refs
            .iter()
            .find(|e| {
                e.kind == "face" && e.normal.map(|n| (n.z - 1.0).abs() < 1e-9).unwrap_or(false)
            })
            .unwrap();
        let f = frame_of(&snap, &top.selector).unwrap();
        assert!((f.z_axis[2] - 1.0).abs() < 1e-9);
        assert!((f.origin_mm.z - 15.0).abs() < 1e-6);
    }
}
