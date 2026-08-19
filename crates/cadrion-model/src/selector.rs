//! Stable selector tokens: `#o{obj}[.{solid}][.f{face}|.e{edge}|.v{vertex}]`.

use serde::{Deserialize, Serialize};
use std::fmt;

use cadrion_kernel::{Point3, Vec3};

/// Topology entity kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityKind {
    Object,
    Solid,
    Face,
    Edge,
    Vertex,
}

/// Parsed selector token.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Selector {
    /// Object index (1-based in token string, stored 1-based).
    pub object: u32,
    /// Optional solid index (1-based).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub solid: Option<u32>,
    /// Optional face index (1-based).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub face: Option<u32>,
    /// Optional edge index (1-based).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edge: Option<u32>,
    /// Optional vertex index (1-based).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vertex: Option<u32>,
}

impl Selector {
    pub fn object(object: u32) -> Self {
        Self {
            object,
            solid: None,
            face: None,
            edge: None,
            vertex: None,
        }
    }

    pub fn solid(object: u32, solid: u32) -> Self {
        Self {
            object,
            solid: Some(solid),
            face: None,
            edge: None,
            vertex: None,
        }
    }

    pub fn face(object: u32, solid: u32, face: u32) -> Self {
        Self {
            object,
            solid: Some(solid),
            face: Some(face),
            edge: None,
            vertex: None,
        }
    }

    pub fn edge(object: u32, solid: u32, edge: u32) -> Self {
        Self {
            object,
            solid: Some(solid),
            face: None,
            edge: Some(edge),
            vertex: None,
        }
    }

    pub fn kind(&self) -> EntityKind {
        if self.vertex.is_some() {
            EntityKind::Vertex
        } else if self.edge.is_some() {
            EntityKind::Edge
        } else if self.face.is_some() {
            EntityKind::Face
        } else if self.solid.is_some() {
            EntityKind::Solid
        } else {
            EntityKind::Object
        }
    }
}

impl fmt::Display for Selector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#o{}", self.object)?;
        if let Some(s) = self.solid {
            write!(f, ".{s}")?;
        }
        if let Some(face) = self.face {
            write!(f, ".f{face}")?;
        }
        if let Some(edge) = self.edge {
            write!(f, ".e{edge}")?;
        }
        if let Some(v) = self.vertex {
            write!(f, ".v{v}")?;
        }
        Ok(())
    }
}

/// Selector parse failure.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SelectorError {
    #[error("selector must start with '#o': {0}")]
    BadPrefix(String),
    #[error("invalid selector syntax: {0}")]
    Syntax(String),
    #[error("selector indices must be >= 1: {0}")]
    ZeroIndex(String),
}

/// Parse `#o1`, `#o1.2`, `#o1.2.f3`, `#o1.1.e7`, `#o1.1.v2`.
pub fn parse_selector(token: &str) -> Result<Selector, SelectorError> {
    let raw = token.trim();
    if !raw.starts_with("#o") {
        return Err(SelectorError::BadPrefix(raw.to_string()));
    }
    let rest = &raw[2..];
    if rest.is_empty() {
        return Err(SelectorError::Syntax(raw.to_string()));
    }

    let mut object: Option<u32> = None;
    let mut solid: Option<u32> = None;
    let mut face: Option<u32> = None;
    let mut edge: Option<u32> = None;
    let mut vertex: Option<u32> = None;

    // Split keeping letter prefixes on segments after the first numeric object.
    // Forms: 1 | 1.2 | 1.2.f3 | 1.1.e7 | 1.1.v2
    let parts: Vec<&str> = rest.split('.').collect();
    for (i, p) in parts.iter().enumerate() {
        if p.is_empty() {
            return Err(SelectorError::Syntax(raw.to_string()));
        }
        if i == 0 {
            object = Some(parse_index(p, raw)?);
            continue;
        }
        let bytes = p.as_bytes();
        match bytes[0] {
            b'f' | b'F' => {
                if face.is_some() || edge.is_some() || vertex.is_some() {
                    return Err(SelectorError::Syntax(raw.to_string()));
                }
                face = Some(parse_index(&p[1..], raw)?);
            }
            b'e' | b'E' => {
                if face.is_some() || edge.is_some() || vertex.is_some() {
                    return Err(SelectorError::Syntax(raw.to_string()));
                }
                edge = Some(parse_index(&p[1..], raw)?);
            }
            b'v' | b'V' => {
                if face.is_some() || edge.is_some() || vertex.is_some() {
                    return Err(SelectorError::Syntax(raw.to_string()));
                }
                vertex = Some(parse_index(&p[1..], raw)?);
            }
            b'0'..=b'9' => {
                if solid.is_some() || face.is_some() || edge.is_some() || vertex.is_some() {
                    return Err(SelectorError::Syntax(raw.to_string()));
                }
                solid = Some(parse_index(p, raw)?);
            }
            _ => return Err(SelectorError::Syntax(raw.to_string())),
        }
    }

    let object = object.ok_or_else(|| SelectorError::Syntax(raw.to_string()))?;
    Ok(Selector {
        object,
        solid,
        face,
        edge,
        vertex,
    })
}

fn parse_index(s: &str, raw: &str) -> Result<u32, SelectorError> {
    let n: u32 = s
        .parse()
        .map_err(|_| SelectorError::Syntax(raw.to_string()))?;
    if n == 0 {
        return Err(SelectorError::ZeroIndex(raw.to_string()));
    }
    Ok(n)
}

/// Face sort key: (cz, cy, cx, area) with quantized floats for stability.
pub fn sort_key_face(centroid: Point3, area: f64) -> (i64, i64, i64, i64) {
    (
        quantize(centroid.z),
        quantize(centroid.y),
        quantize(centroid.x),
        quantize(area),
    )
}

/// Solid sort key: (cz, cy, cx, volume).
pub fn sort_key_solid(centroid: Point3, volume: f64) -> (i64, i64, i64, i64) {
    (
        quantize(centroid.z),
        quantize(centroid.y),
        quantize(centroid.x),
        quantize(volume),
    )
}

fn quantize(v: f64) -> i64 {
    // 1e-6 mm resolution; stable across tiny float noise
    (v * 1_000_000.0).round() as i64
}

/// Assign 1-based solid indices after stable sort. Input items are (centroid, volume, payload).
pub fn assign_solid_indices<T>(mut items: Vec<(Point3, f64, T)>) -> Vec<(u32, Point3, f64, T)> {
    items.sort_by(|a, b| {
        sort_key_solid(a.0, a.1)
            .cmp(&sort_key_solid(b.0, b.1))
            .then_with(|| quantize(a.1).cmp(&quantize(b.1)))
    });
    items
        .into_iter()
        .enumerate()
        .map(|(i, (c, v, t))| ((i + 1) as u32, c, v, t))
        .collect()
}

/// Assign 1-based face indices after stable sort. Items: (centroid, area, normal, payload).
pub fn assign_face_indices<T>(
    mut items: Vec<(Point3, f64, Option<Vec3>, T)>,
) -> Vec<(u32, Point3, f64, Option<Vec3>, T)> {
    items.sort_by(|a, b| {
        sort_key_face(a.0, a.1)
            .cmp(&sort_key_face(b.0, b.1))
            .then_with(|| {
                // normal z as final tie-break
                let na = a.2.map(|n| quantize(n.z)).unwrap_or(0);
                let nb = b.2.map(|n| quantize(n.z)).unwrap_or(0);
                na.cmp(&nb)
            })
    });
    items
        .into_iter()
        .enumerate()
        .map(|(i, (c, a, n, t))| ((i + 1) as u32, c, a, n, t))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_roundtrip() {
        let cases = ["#o1", "#o1.2", "#o1.2.f3", "#o1.1.e7", "#o2.3.v1"];
        for c in cases {
            let s = parse_selector(c).unwrap();
            assert_eq!(s.to_string(), c);
        }
    }

    #[test]
    fn reject_zero_and_junk() {
        assert!(matches!(
            parse_selector("#o0"),
            Err(SelectorError::ZeroIndex(_))
        ));
        assert!(parse_selector("o1").is_err());
        assert!(parse_selector("#o1.f").is_err());
        assert!(parse_selector("#o1.2.f3.e1").is_err());
    }

    #[test]
    fn face_order_stable() {
        let items = vec![
            (Point3::new(0.0, 0.0, 10.0), 100.0, Some(Vec3::Z), "top"),
            (
                Point3::new(0.0, 0.0, 0.0),
                100.0,
                Some(Vec3::new(0.0, 0.0, -1.0)),
                "bot",
            ),
            (Point3::new(5.0, 0.0, 5.0), 50.0, Some(Vec3::X), "side"),
        ];
        let ordered = assign_face_indices(items);
        // lowest z first → bot, then side, then top
        assert_eq!(ordered[0].4, "bot");
        assert_eq!(ordered[0].0, 1);
        assert_eq!(ordered[1].4, "side");
        assert_eq!(ordered[2].4, "top");
        assert_eq!(ordered[2].0, 3);
    }

    #[test]
    fn property_selector_junk_no_panic() {
        let long = "x".repeat(1000);
        let samples = [
            "",
            "#",
            "#o",
            "#o9999999999",
            "#o1.2.3.4.5",
            "#o-1",
            long.as_str(),
            "#o1.f999999",
        ];
        for s in samples {
            let _ = parse_selector(s); // must not panic
        }
    }
}
