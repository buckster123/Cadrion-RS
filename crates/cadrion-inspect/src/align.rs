//! Align check between two face/solid refs.

use serde::{Deserialize, Serialize};

use crate::lookup::{lookup_in_report, LookupError};
use crate::refs::inspect_refs;
use crate::topology::TopologySnapshot;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlignExpect {
    Coplanar,
    Coaxial,
    Distance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlignReport {
    pub ok: bool,
    pub expect: String,
    pub a: String,
    pub b: String,
    pub translation_err_mm: f64,
    pub angular_err_deg: f64,
    pub distance_mm: f64,
    pub tol_mm: f64,
    pub tol_deg: f64,
    pub detail: String,
}

#[derive(Debug, thiserror::Error)]
pub enum AlignError {
    #[error("{0}")]
    Lookup(#[from] LookupError),
    #[error("{0}")]
    Msg(String),
}

/// Align two refs (typically faces with normals).
pub fn align_refs(
    snap: &TopologySnapshot,
    a: &str,
    b: &str,
    expect: AlignExpect,
    expect_distance: Option<f64>,
    tol_mm: f64,
    tol_deg: f64,
) -> Result<AlignReport, AlignError> {
    let report = inspect_refs(snap, false);
    let ra = lookup_in_report(&report.refs, a)?;
    let rb = lookup_in_report(&report.refs, b)?;
    let ao = ra.centroid_mm;
    let bo = rb.centroid_mm;
    let dx = bo.x - ao.x;
    let dy = bo.y - ao.y;
    let dz = bo.z - ao.z;
    let dist = (dx * dx + dy * dy + dz * dz).sqrt();
    let ang = match (ra.normal, rb.normal) {
        (Some(na), Some(nb)) => {
            let dot = (na.x * nb.x + na.y * nb.y + na.z * nb.z).clamp(-1.0, 1.0);
            // smallest angle between axes (0 = parallel)
            dot.abs().acos().to_degrees()
        }
        _ => 0.0,
    };

    let (ok, detail) = match expect {
        AlignExpect::Coplanar => {
            // normals parallel + separation along normal small
            let sep = match ra.normal {
                Some(n) => (dx * n.x + dy * n.y + dz * n.z).abs(),
                None => dist,
            };
            let ok = ang <= tol_deg && sep <= tol_mm;
            (
                ok,
                format!("coplanar ang={ang:.4}deg normal_sep={sep:.4}mm"),
            )
        }
        AlignExpect::Coaxial => {
            let ok = ang <= tol_deg;
            (ok, format!("coaxial/parallel normals ang={ang:.4}deg"))
        }
        AlignExpect::Distance => {
            let want = expect_distance.unwrap_or(0.0);
            let err = (dist - want).abs();
            (
                err <= tol_mm,
                format!("distance got={dist:.4} want={want:.4} err={err:.4}"),
            )
        }
    };

    Ok(AlignReport {
        ok,
        expect: format!("{expect:?}").to_ascii_lowercase(),
        a: ra.selector.clone(),
        b: rb.selector.clone(),
        translation_err_mm: dist,
        angular_err_deg: ang,
        distance_mm: dist,
        tol_mm,
        tol_deg,
        detail,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::topology::{box_topology, TopologySnapshot};
    use cadrion_kernel::Point3;

    #[test]
    fn opposite_faces_distance() {
        let snap = TopologySnapshot::single_solid(box_topology(100.0, 60.0, 20.0, Point3::ORIGIN));
        let report = inspect_refs(&snap, false);
        let px = report
            .refs
            .iter()
            .find(|e| {
                e.kind == "face" && e.normal.map(|n| (n.x - 1.0).abs() < 1e-9).unwrap_or(false)
            })
            .unwrap();
        let nx = report
            .refs
            .iter()
            .find(|e| {
                e.kind == "face" && e.normal.map(|n| (n.x + 1.0).abs() < 1e-9).unwrap_or(false)
            })
            .unwrap();
        let r = align_refs(
            &snap,
            &px.selector,
            &nx.selector,
            AlignExpect::Distance,
            Some(100.0),
            0.1,
            1.0,
        )
        .unwrap();
        assert!(r.ok, "{}", r.detail);
    }

    #[test]
    fn top_bot_coplanar_normals() {
        let snap = TopologySnapshot::single_solid(box_topology(10.0, 10.0, 10.0, Point3::ORIGIN));
        let report = inspect_refs(&snap, false);
        let top = report
            .refs
            .iter()
            .find(|e| {
                e.kind == "face" && e.normal.map(|n| (n.z - 1.0).abs() < 1e-9).unwrap_or(false)
            })
            .unwrap();
        let bot = report
            .refs
            .iter()
            .find(|e| {
                e.kind == "face" && e.normal.map(|n| (n.z + 1.0).abs() < 1e-9).unwrap_or(false)
            })
            .unwrap();
        let r = align_refs(
            &snap,
            &top.selector,
            &bot.selector,
            AlignExpect::Coaxial,
            None,
            0.1,
            1.0,
        )
        .unwrap();
        assert!(r.ok, "opposite normals still parallel: {}", r.detail);
    }
}
