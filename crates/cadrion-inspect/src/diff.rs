//! Diff two topology snapshots (volume/area/bbox + selector remap hints).

use serde::{Deserialize, Serialize};

use crate::refs::{inspect_refs, RefEntry};
use crate::topology::TopologySnapshot;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffReport {
    pub ok: bool,
    pub volume_old_mm3: f64,
    pub volume_new_mm3: f64,
    pub volume_delta_mm3: f64,
    pub area_old_mm2: f64,
    pub area_new_mm2: f64,
    pub faces_old: u32,
    pub faces_new: u32,
    pub edges_old: u32,
    pub edges_new: u32,
    pub solids_old: u32,
    pub solids_new: u32,
    /// Heuristic remaps: old selector → new selector (same kind, nearest centroid).
    pub selector_remap: Vec<SelectorRemap>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectorRemap {
    pub old: String,
    pub new: String,
    pub kind: String,
    pub centroid_dist_mm: f64,
}

/// Compare old vs new topology (e.g. two builds).
pub fn diff_snapshots(old: &TopologySnapshot, new: &TopologySnapshot) -> DiffReport {
    let ro = inspect_refs(old, true);
    let rn = inspect_refs(new, true);
    let vo = ro.facts.as_ref().map(|f| f.volume_mm3).unwrap_or(0.0);
    let vn = rn.facts.as_ref().map(|f| f.volume_mm3).unwrap_or(0.0);
    let ao = ro.facts.as_ref().map(|f| f.area_mm2).unwrap_or(0.0);
    let an = rn.facts.as_ref().map(|f| f.area_mm2).unwrap_or(0.0);

    let mut notes = Vec::new();
    if (vo - vn).abs() > 1e-6 {
        notes.push(format!("volume changed by {:.4} mm³", vn - vo));
    } else {
        notes.push("volume unchanged (within 1e-6)".into());
    }
    if ro.faces != rn.faces {
        notes.push(format!("face count {} → {}", ro.faces, rn.faces));
    }

    let remap = remap_selectors(&ro.refs, &rn.refs);
    DiffReport {
        ok: true,
        volume_old_mm3: vo,
        volume_new_mm3: vn,
        volume_delta_mm3: vn - vo,
        area_old_mm2: ao,
        area_new_mm2: an,
        faces_old: ro.faces,
        faces_new: rn.faces,
        edges_old: ro.edges,
        edges_new: rn.edges,
        solids_old: ro.solids,
        solids_new: rn.solids,
        selector_remap: remap,
        notes,
    }
}

fn remap_selectors(old: &[RefEntry], new: &[RefEntry]) -> Vec<SelectorRemap> {
    let mut out = Vec::new();
    for o in old.iter().filter(|r| r.kind == "face" || r.kind == "edge") {
        let mut best: Option<(&RefEntry, f64)> = None;
        for n in new.iter().filter(|r| r.kind == o.kind) {
            let d = centroid_dist(o, n);
            if best.map(|(_, bd)| d < bd).unwrap_or(true) {
                best = Some((n, d));
            }
        }
        if let Some((n, d)) = best {
            if o.selector != n.selector || d > 1e-6 {
                out.push(SelectorRemap {
                    old: o.selector.clone(),
                    new: n.selector.clone(),
                    kind: o.kind.clone(),
                    centroid_dist_mm: d,
                });
            }
        }
    }
    out.sort_by(|a, b| {
        a.centroid_dist_mm
            .partial_cmp(&b.centroid_dist_mm)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    out.truncate(64);
    out
}

fn centroid_dist(a: &RefEntry, b: &RefEntry) -> f64 {
    let dx = a.centroid_mm.x - b.centroid_mm.x;
    let dy = a.centroid_mm.y - b.centroid_mm.y;
    let dz = a.centroid_mm.z - b.centroid_mm.z;
    (dx * dx + dy * dy + dz * dz).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::topology::{box_topology, TopologySnapshot};
    use cadrion_kernel::Point3;

    #[test]
    fn same_box_zero_delta() {
        let a = TopologySnapshot::single_solid(box_topology(10.0, 20.0, 30.0, Point3::ORIGIN));
        let b = TopologySnapshot::single_solid(box_topology(10.0, 20.0, 30.0, Point3::ORIGIN));
        let d = diff_snapshots(&a, &b);
        assert!((d.volume_delta_mm3).abs() < 1e-6);
        assert_eq!(d.faces_old, d.faces_new);
    }

    #[test]
    fn grown_box_positive_delta() {
        let a = TopologySnapshot::single_solid(box_topology(10.0, 10.0, 10.0, Point3::ORIGIN));
        let b = TopologySnapshot::single_solid(box_topology(20.0, 10.0, 10.0, Point3::ORIGIN));
        let d = diff_snapshots(&a, &b);
        assert!(d.volume_delta_mm3 > 500.0);
    }
}
