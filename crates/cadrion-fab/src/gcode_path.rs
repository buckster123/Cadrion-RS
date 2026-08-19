//! G-code path extraction for viewer layer scrub.

use serde::{Deserialize, Serialize};

use crate::gcode::parse_word;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GcodePoint {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    /// True when this segment extruded (E increased) or forced travel=false.
    pub extrude: bool,
    pub line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GcodeLayer {
    pub z: f64,
    /// Range into `points` [start, end).
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GcodePath {
    pub points: Vec<GcodePoint>,
    pub layers: Vec<GcodeLayer>,
    pub bbox_mm: Option<[f64; 6]>,
    pub move_count: usize,
}

/// Parse G0/G1 moves into a polyline + Z-layer index for scrubbing.
pub fn extract_gcode_path(text: &str) -> GcodePath {
    let mut points = Vec::new();
    let mut x = 0.0f64;
    let mut y = 0.0f64;
    let mut z = 0.0f64;
    let mut e = 0.0f64;
    let mut moves = 0usize;
    let mut xmin = f64::INFINITY;
    let mut xmax = f64::NEG_INFINITY;
    let mut ymin = f64::INFINITY;
    let mut ymax = f64::NEG_INFINITY;
    let mut zmin = f64::INFINITY;
    let mut zmax = f64::NEG_INFINITY;
    let mut seeded = false;

    for (i, raw) in text.lines().enumerate() {
        let line_no = i + 1;
        let s = raw.split(';').next().unwrap_or("").trim();
        if s.is_empty() {
            continue;
        }
        let up = s.to_ascii_uppercase();
        if !(up.starts_with("G0")
            || up.starts_with("G1")
            || up.starts_with("G2")
            || up.starts_with("G3"))
        {
            continue;
        }
        if !seeded {
            points.push(GcodePoint {
                x,
                y,
                z,
                extrude: false,
                line: 0,
            });
            seeded = true;
        }
        moves += 1;
        if let Some(v) = parse_word(s, 'X') {
            x = v;
        }
        if let Some(v) = parse_word(s, 'Y') {
            y = v;
        }
        if let Some(v) = parse_word(s, 'Z') {
            z = v;
        }
        let mut extrude = up.starts_with("G1");
        if let Some(v) = parse_word(s, 'E') {
            extrude = v > e + 1e-9;
            e = v;
        } else if up.starts_with("G0") {
            extrude = false;
        }
        points.push(GcodePoint {
            x,
            y,
            z,
            extrude,
            line: line_no,
        });
        xmin = xmin.min(x);
        xmax = xmax.max(x);
        ymin = ymin.min(y);
        ymax = ymax.max(y);
        zmin = zmin.min(z);
        zmax = zmax.max(z);
    }

    let layers = build_layers(&points);
    let bbox = if moves > 0 {
        Some([xmin, xmax, ymin, ymax, zmin, zmax])
    } else {
        None
    };

    GcodePath {
        points,
        layers,
        bbox_mm: bbox,
        move_count: moves,
    }
}

fn build_layers(points: &[GcodePoint]) -> Vec<GcodeLayer> {
    if points.is_empty() {
        return Vec::new();
    }
    let mut layers = Vec::new();
    let mut start = 0usize;
    let mut z0 = points[0].z;
    for (i, p) in points.iter().enumerate().skip(1) {
        if (p.z - z0).abs() > 0.05 {
            layers.push(GcodeLayer {
                z: z0,
                start,
                end: i,
            });
            start = i;
            z0 = p.z;
        }
    }
    layers.push(GcodeLayer {
        z: z0,
        start,
        end: points.len(),
    });
    layers
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layers_split_on_z() {
        let g = "G1 X0 Y0 Z0.2\nG1 X10 Y0 Z0.2 E1\nG1 X10 Y10 Z0.2 E2\nG1 X0 Y0 Z0.4\nG1 X5 Y5 Z0.4 E3\n";
        let p = extract_gcode_path(g);
        assert!(p.move_count >= 4);
        assert!(p.layers.len() >= 2, "layers={:?}", p.layers.len());
        // First move sets Z=0.2; seed at 0 may form a thin layer — find 0.2 layer.
        assert!(
            p.layers.iter().any(|l| (l.z - 0.2).abs() < 0.01),
            "layers z={:?}",
            p.layers.iter().map(|l| l.z).collect::<Vec<_>>()
        );
    }
}
