//! PMI / drawing alpha (H2-8) — linear dimension facts → JSON drawing packet.
//!
//! **Not a drafting package.** No sheets, no GD&T symbols, no STEP AP242 PMI.

use serde::{Deserialize, Serialize};

use crate::measure::{measure, MeasureKind, MeasureRequest, MeasureResult};
use crate::refs::{inspect_refs, RefEntry};
use crate::topology::TopologySnapshot;

/// One linear (or diameter) dimension fact attached to selectors.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DimFact {
    pub id: String,
    /// `linear` | `diameter` | `thickness` | `angle`
    pub kind: String,
    pub a: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub b: Option<String>,
    pub value: f64,
    pub unit: String,
    pub construction: String,
    /// Human label (optional).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

/// Explicit dim request (optional input JSON / CLI flags).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DimSpec {
    pub a: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub b: Option<String>,
    /// distance | diameter | thickness | angle (default distance)
    #[serde(default = "default_kind")]
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

fn default_kind() -> String {
    "distance".into()
}

/// Drawing packet — alpha PMI surface.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DrawingPacket {
    pub ok: bool,
    pub schema: String,
    pub version: u32,
    /// Source label (file stem or IR label).
    pub source: String,
    pub dims: Vec<DimFact>,
    pub notes: Vec<String>,
    /// Topology provenance.
    pub topology: String,
}

/// Build a drawing packet from optional explicit specs; if empty, auto-linear opposite faces.
pub fn build_drawing_packet(
    snap: &TopologySnapshot,
    source: &str,
    topology_note: &str,
    specs: &[DimSpec],
) -> DrawingPacket {
    let mut notes = vec![
        "PMI alpha (H2-8): dimension facts only — not a drafting package".into(),
        "No sheets / title blocks / GD&T / STEP AP242 PMI".into(),
    ];
    let mut dims = Vec::new();
    let mut errors = 0u32;

    if specs.is_empty() {
        notes.push("auto: linear dims between opposite-normal face pairs".into());
        dims.extend(auto_linear_dims(snap));
    } else {
        for (i, s) in specs.iter().enumerate() {
            match resolve_spec(snap, s, i) {
                Ok(d) => dims.push(d),
                Err(e) => {
                    errors += 1;
                    notes.push(format!("dim[{i}] failed: {e}"));
                }
            }
        }
    }

    DrawingPacket {
        ok: errors == 0 && !dims.is_empty(),
        schema: "cadrion.drawing_packet".into(),
        version: 1,
        source: source.into(),
        dims,
        notes,
        topology: topology_note.into(),
    }
}

fn resolve_spec(snap: &TopologySnapshot, s: &DimSpec, i: usize) -> Result<DimFact, String> {
    let kind = match s.kind.to_ascii_lowercase().as_str() {
        "distance" | "linear" | "dist" => MeasureKind::Distance,
        "diameter" | "dia" => MeasureKind::Diameter,
        "thickness" | "thick" => MeasureKind::Thickness,
        "angle" => MeasureKind::Angle,
        other => return Err(format!("unknown kind '{other}'")),
    };
    let m = measure(
        snap,
        &MeasureRequest {
            a: s.a.clone(),
            b: s.b.clone(),
            kind,
        },
    )
    .map_err(|e| e.to_string())?;
    Ok(fact_from_measure(m, i, s.label.clone()))
}

fn fact_from_measure(m: MeasureResult, i: usize, label: Option<String>) -> DimFact {
    let kind = match m.kind {
        MeasureKind::Distance => "linear",
        MeasureKind::Diameter => "diameter",
        MeasureKind::Thickness => "thickness",
        MeasureKind::Angle => "angle",
    };
    DimFact {
        id: format!("d{}", i + 1),
        kind: kind.into(),
        a: m.a,
        b: m.b,
        value: m.value,
        unit: m.unit,
        construction: m.construction,
        label,
    }
}

fn auto_linear_dims(snap: &TopologySnapshot) -> Vec<DimFact> {
    let report = inspect_refs(snap, false);
    let faces: Vec<&RefEntry> = report
        .refs
        .iter()
        .filter(|r| r.kind == "face" && r.normal.is_some())
        .collect();
    let mut out = Vec::new();
    let mut used = std::collections::HashSet::new();
    for i in 0..faces.len() {
        for j in (i + 1)..faces.len() {
            let a = faces[i];
            let b = faces[j];
            let na = a.normal.unwrap();
            let nb = b.normal.unwrap();
            let dot = na.x * nb.x + na.y * nb.y + na.z * nb.z;
            // opposite-ish normals
            if dot > -0.95 {
                continue;
            }
            let key = {
                let mut pair = [a.selector.as_str(), b.selector.as_str()];
                pair.sort_unstable();
                format!("{}|{}", pair[0], pair[1])
            };
            if !used.insert(key) {
                continue;
            }
            // thickness along a's normal
            match measure(
                snap,
                &MeasureRequest {
                    a: a.selector.clone(),
                    b: Some(b.selector.clone()),
                    kind: MeasureKind::Thickness,
                },
            ) {
                Ok(m) if m.value > 1e-6 && m.value.is_finite() => {
                    let idx = out.len();
                    let mut f = fact_from_measure(m, idx, Some("auto-opposite".into()));
                    f.kind = "linear".into();
                    out.push(f);
                }
                _ => {}
            }
            if out.len() >= 12 {
                return out;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::topology::{box_topology, TopologySnapshot};
    use cadrion_kernel::Point3;

    #[test]
    fn auto_dims_on_box() {
        let snap = TopologySnapshot::single_solid(box_topology(100.0, 60.0, 20.0, Point3::ORIGIN));
        let pkt = build_drawing_packet(&snap, "box", "ir-analytic", &[]);
        assert!(pkt.ok, "{pkt:?}");
        assert!(!pkt.dims.is_empty());
        // expect ~100, 60, 20 among values
        let vals: Vec<f64> = pkt.dims.iter().map(|d| d.value).collect();
        assert!(
            vals.iter().any(|v| (*v - 100.0).abs() < 1e-3)
                || vals.iter().any(|v| (*v - 60.0).abs() < 1e-3)
                || vals.iter().any(|v| (*v - 20.0).abs() < 1e-3),
            "vals={vals:?}"
        );
        assert_eq!(pkt.schema, "cadrion.drawing_packet");
    }

    #[test]
    fn explicit_dim_spec() {
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
        let specs = vec![DimSpec {
            a: top.selector.clone(),
            b: Some(bot.selector.clone()),
            kind: "thickness".into(),
            label: Some("height".into()),
        }];
        let pkt = build_drawing_packet(&snap, "box", "ir-analytic", &specs);
        assert!(pkt.ok);
        assert_eq!(pkt.dims.len(), 1);
        assert!((pkt.dims[0].value - 20.0).abs() < 1e-6);
        assert_eq!(pkt.dims[0].label.as_deref(), Some("height"));
    }
}
