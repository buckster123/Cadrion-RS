//! `inspect refs` — selector inventory + optional facts.

use cadrion_kernel::{Point3, Vec3};
use cadrion_model::{assign_face_indices, assign_solid_indices, Selector};
use serde::{Deserialize, Serialize};

use crate::topology::{FaceRec, TopologySnapshot};

/// One row in the refs report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RefEntry {
    pub selector: String,
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub area_mm2: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub length_mm: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volume_mm3: Option<f64>,
    pub centroid_mm: Point3,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub normal: Option<Vec3>,
}

/// Full refs report (JSON for `--json`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RefsReport {
    pub object: u32,
    pub solids: u32,
    pub faces: u32,
    pub edges: u32,
    pub refs: Vec<RefEntry>,
    /// Aggregate facts when requested.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub facts: Option<FactsSummary>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FactsSummary {
    pub volume_mm3: f64,
    pub area_mm2: f64,
    pub centroid_mm: Point3,
    pub solid_count: u32,
    pub face_count: u32,
    pub edge_count: u32,
}

/// Build selector inventory from a topology snapshot.
///
/// Solids and faces are renumbered with stable sort keys so the same geometry
/// yields the same `#o…` tokens across runs.
pub fn inspect_refs(snap: &TopologySnapshot, include_facts: bool) -> RefsReport {
    let obj = snap.object.max(1);
    let solid_items: Vec<_> = snap
        .solids
        .iter()
        .map(|s| (s.centroid, s.volume_mm3, s))
        .collect();
    let solids = assign_solid_indices(solid_items);

    let mut refs = Vec::new();
    let mut total_faces = 0u32;
    let mut total_edges = 0u32;
    let mut total_volume = 0.0;
    let mut total_area = 0.0;
    let mut cx = 0.0;
    let mut cy = 0.0;
    let mut cz = 0.0;

    // object token
    refs.push(RefEntry {
        selector: Selector::object(obj).to_string(),
        kind: "object".into(),
        area_mm2: None,
        length_mm: None,
        volume_mm3: Some(snap.solids.iter().map(|s| s.volume_mm3).sum()),
        centroid_mm: volume_weighted_centroid(snap),
        normal: None,
    });

    for (s_idx, centroid, volume, solid) in solids {
        total_volume += volume;
        cx += centroid.x * volume;
        cy += centroid.y * volume;
        cz += centroid.z * volume;

        refs.push(RefEntry {
            selector: Selector::solid(obj, s_idx).to_string(),
            kind: "solid".into(),
            area_mm2: None,
            length_mm: None,
            volume_mm3: Some(volume),
            centroid_mm: centroid,
            normal: None,
        });

        let face_items: Vec<_> = solid
            .faces
            .iter()
            .map(|f: &FaceRec| (f.centroid, f.area_mm2, f.normal, f))
            .collect();
        let faces = assign_face_indices(face_items);
        for (f_idx, fcent, area, normal, _) in faces {
            total_faces += 1;
            total_area += area;
            refs.push(RefEntry {
                selector: Selector::face(obj, s_idx, f_idx).to_string(),
                kind: "face".into(),
                area_mm2: Some(area),
                length_mm: None,
                volume_mm3: None,
                centroid_mm: fcent,
                normal,
            });
        }

        // edges: sort by midpoint then length
        let mut edges: Vec<_> = solid.edges.iter().collect();
        edges.sort_by(|a, b| {
            let ka = (
                quant(a.midpoint.z),
                quant(a.midpoint.y),
                quant(a.midpoint.x),
                quant(a.length_mm),
            );
            let kb = (
                quant(b.midpoint.z),
                quant(b.midpoint.y),
                quant(b.midpoint.x),
                quant(b.length_mm),
            );
            ka.cmp(&kb)
        });
        for (i, e) in edges.iter().enumerate() {
            total_edges += 1;
            let e_idx = (i + 1) as u32;
            refs.push(RefEntry {
                selector: Selector::edge(obj, s_idx, e_idx).to_string(),
                kind: "edge".into(),
                area_mm2: None,
                length_mm: Some(e.length_mm),
                volume_mm3: None,
                centroid_mm: e.midpoint,
                normal: None,
            });
        }
    }

    let facts = if include_facts {
        let centroid = if total_volume > 0.0 {
            Point3::new(cx / total_volume, cy / total_volume, cz / total_volume)
        } else {
            Point3::ORIGIN
        };
        Some(FactsSummary {
            volume_mm3: total_volume,
            area_mm2: total_area,
            centroid_mm: centroid,
            solid_count: snap.solids.len() as u32,
            face_count: total_faces,
            edge_count: total_edges,
        })
    } else {
        None
    };

    RefsReport {
        object: obj,
        solids: snap.solids.len() as u32,
        faces: total_faces,
        edges: total_edges,
        refs,
        facts,
    }
}

fn volume_weighted_centroid(snap: &TopologySnapshot) -> Point3 {
    let mut v = 0.0;
    let mut cx = 0.0;
    let mut cy = 0.0;
    let mut cz = 0.0;
    for s in &snap.solids {
        v += s.volume_mm3;
        cx += s.centroid.x * s.volume_mm3;
        cy += s.centroid.y * s.volume_mm3;
        cz += s.centroid.z * s.volume_mm3;
    }
    if v <= 0.0 {
        return Point3::ORIGIN;
    }
    Point3::new(cx / v, cy / v, cz / v)
}

fn quant(v: f64) -> i64 {
    (v * 1_000_000.0).round() as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::topology::box_topology;
    use cadrion_kernel::Point3;

    #[test]
    fn box_refs_stable_and_complete() {
        let solid = box_topology(100.0, 60.0, 20.0, Point3::ORIGIN);
        let snap = TopologySnapshot::single_solid(solid);
        let r1 = inspect_refs(&snap, true);
        let r2 = inspect_refs(&snap, true);
        assert_eq!(r1.refs, r2.refs);
        assert_eq!(r1.faces, 6);
        assert_eq!(r1.edges, 12);
        assert!(r1.refs.iter().any(|e| e.selector == "#o1.1.f1"));
        let facts = r1.facts.unwrap();
        assert!((facts.volume_mm3 - 120_000.0).abs() < 1e-6);
        // top face (+Z) should be last or high z — find face with normal +Z
        let top = r1
            .refs
            .iter()
            .find(|e| {
                e.kind == "face" && e.normal.map(|n| (n.z - 1.0).abs() < 1e-9).unwrap_or(false)
            })
            .unwrap();
        assert!((top.centroid_mm.z - 10.0).abs() < 1e-9);
    }
}
