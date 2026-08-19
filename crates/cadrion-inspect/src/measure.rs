//! `inspect measure` — numeric distance / diameter / etc.

use cadrion_kernel::{Point3, Vec3};
use cadrion_model::{parse_selector, Selector, SelectorError};
use serde::{Deserialize, Serialize};

use crate::refs::{inspect_refs, RefEntry};
use crate::topology::TopologySnapshot;

/// Measurement kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeasureKind {
    Distance,
    Angle,
    Diameter,
    Thickness,
}

/// Measure request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MeasureRequest {
    pub a: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub b: Option<String>,
    pub kind: MeasureKind,
}

/// Measure result (JSON).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MeasureResult {
    pub kind: MeasureKind,
    pub value: f64,
    pub unit: String,
    pub construction: String,
    pub a: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub b: Option<String>,
}

/// Measure errors.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MeasureError {
    #[error("selector: {0}")]
    Selector(#[from] SelectorError),
    #[error("unknown ref {0}")]
    UnknownRef(String),
    #[error("measure {0} requires two refs")]
    NeedsTwo(&'static str),
    #[error("measure {0} not supported for kinds {1} / {2}")]
    Unsupported(&'static str, String, String),
    #[error("{0}")]
    Msg(String),
}

/// Run a measurement against a topology snapshot.
pub fn measure(
    snap: &TopologySnapshot,
    req: &MeasureRequest,
) -> Result<MeasureResult, MeasureError> {
    let report = inspect_refs(snap, false);
    let a_sel = parse_selector(&req.a)?;
    let a = find_ref(&report.refs, &a_sel, &req.a)?;

    match req.kind {
        MeasureKind::Distance => {
            let b_tok = req.b.as_deref().ok_or(MeasureError::NeedsTwo("distance"))?;
            let b_sel = parse_selector(b_tok)?;
            let b = find_ref(&report.refs, &b_sel, b_tok)?;
            let d = dist(a.centroid_mm, b.centroid_mm);
            Ok(MeasureResult {
                kind: MeasureKind::Distance,
                value: d,
                unit: "mm".into(),
                construction: format!(
                    "euclidean distance between centroids of {} and {}",
                    a.selector, b.selector
                ),
                a: a.selector.clone(),
                b: Some(b.selector.clone()),
            })
        }
        MeasureKind::Diameter => {
            // Face with circular edge: use max edge length / π as diameter for circular edges
            // Or if edge given, diameter = length/π for full circle.
            if let Some(len) = a.length_mm {
                let diam = len / std::f64::consts::PI;
                return Ok(MeasureResult {
                    kind: MeasureKind::Diameter,
                    value: diam,
                    unit: "mm".into(),
                    construction: format!("circle diameter from edge length/π on {}", a.selector),
                    a: a.selector.clone(),
                    b: None,
                });
            }
            if let Some(area) = a.area_mm2 {
                // assume circular face: A = π r² → d = 2 sqrt(A/π)
                let diam = 2.0 * (area / std::f64::consts::PI).sqrt();
                return Ok(MeasureResult {
                    kind: MeasureKind::Diameter,
                    value: diam,
                    unit: "mm".into(),
                    construction: format!("circle diameter from face area (πr²) on {}", a.selector),
                    a: a.selector.clone(),
                    b: None,
                });
            }
            Err(MeasureError::Msg(format!(
                "diameter needs face area or circular edge length on {}",
                req.a
            )))
        }
        MeasureKind::Angle => {
            let b_tok = req.b.as_deref().ok_or(MeasureError::NeedsTwo("angle"))?;
            let b_sel = parse_selector(b_tok)?;
            let b = find_ref(&report.refs, &b_sel, b_tok)?;
            let na = a
                .normal
                .ok_or_else(|| MeasureError::Msg(format!("{} has no normal", a.selector)))?;
            let nb = b
                .normal
                .ok_or_else(|| MeasureError::Msg(format!("{} has no normal", b.selector)))?;
            let cos = dot(na, nb).clamp(-1.0, 1.0);
            let deg = cos.acos().to_degrees();
            Ok(MeasureResult {
                kind: MeasureKind::Angle,
                value: deg,
                unit: "deg".into(),
                construction: format!("angle between normals of {} and {}", a.selector, b.selector),
                a: a.selector.clone(),
                b: Some(b.selector.clone()),
            })
        }
        MeasureKind::Thickness => {
            let b_tok = req
                .b
                .as_deref()
                .ok_or(MeasureError::NeedsTwo("thickness"))?;
            let b_sel = parse_selector(b_tok)?;
            let b = find_ref(&report.refs, &b_sel, b_tok)?;
            // thickness ≈ distance between parallel face centroids projected on normal
            let na = a
                .normal
                .ok_or_else(|| MeasureError::Msg(format!("{} has no normal", a.selector)))?;
            let delta = sub(b.centroid_mm, a.centroid_mm);
            let t = dot_point(delta, na).abs();
            Ok(MeasureResult {
                kind: MeasureKind::Thickness,
                value: t,
                unit: "mm".into(),
                construction: format!(
                    "projected distance along normal of {} toward {}",
                    a.selector, b.selector
                ),
                a: a.selector.clone(),
                b: Some(b.selector.clone()),
            })
        }
    }
}

fn find_ref<'a>(
    refs: &'a [RefEntry],
    sel: &Selector,
    raw: &str,
) -> Result<&'a RefEntry, MeasureError> {
    let token = sel.to_string();
    refs.iter()
        .find(|r| r.selector == token)
        .ok_or_else(|| MeasureError::UnknownRef(raw.to_string()))
}

fn dist(a: Point3, b: Point3) -> f64 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    let dz = a.z - b.z;
    (dx * dx + dy * dy + dz * dz).sqrt()
}

fn dot(a: Vec3, b: Vec3) -> f64 {
    a.x * b.x + a.y * b.y + a.z * b.z
}

fn sub(a: Point3, b: Point3) -> Point3 {
    Point3::new(a.x - b.x, a.y - b.y, a.z - b.z)
}

fn dot_point(a: Point3, b: Vec3) -> f64 {
    a.x * b.x + a.y * b.y + a.z * b.z
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::topology::{box_topology, TopologySnapshot};
    use cadrion_kernel::Point3;

    #[test]
    fn box_height_as_thickness() {
        let snap = TopologySnapshot::single_solid(box_topology(100.0, 60.0, 20.0, Point3::ORIGIN));
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
        let m = measure(
            &snap,
            &MeasureRequest {
                a: top.selector.clone(),
                b: Some(bot.selector.clone()),
                kind: MeasureKind::Thickness,
            },
        )
        .unwrap();
        assert!((m.value - 20.0).abs() < 1e-6, "got {}", m.value);
    }

    #[test]
    fn distance_between_opposite_faces() {
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
        let m = measure(
            &snap,
            &MeasureRequest {
                a: px.selector.clone(),
                b: Some(nx.selector.clone()),
                kind: MeasureKind::Distance,
            },
        )
        .unwrap();
        assert!((m.value - 100.0).abs() < 1e-6);
    }
}
