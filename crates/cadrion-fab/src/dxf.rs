//! Minimal DXF R12 text writer (units: mm).

use std::fmt::Write as _;

#[derive(Debug, Clone)]
pub struct DxfLayer {
    pub name: String,
    pub color: i32,
}

#[derive(Debug, Clone)]
pub enum DxfEntity {
    Line {
        layer: String,
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
    },
    Circle {
        layer: String,
        cx: f64,
        cy: f64,
        r: f64,
    },
    /// Closed LWPOLYLINE-style as LINE segments for R12 simplicity.
    Rect {
        layer: String,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
    },
}

/// Write a minimal ASCII DXF (R12-compatible subset).
pub fn write_dxf_r12(layers: &[DxfLayer], entities: &[DxfEntity]) -> String {
    let mut out = String::new();
    // HEADER
    out.push_str("0\nSECTION\n2\nHEADER\n");
    out.push_str("9\n$ACADVER\n1\nAC1009\n");
    out.push_str("9\n$INSUNITS\n70\n4\n"); // 4 = millimeters
    out.push_str("0\nENDSEC\n");
    // TABLES / LAYER
    out.push_str("0\nSECTION\n2\nTABLES\n");
    out.push_str("0\nTABLE\n2\nLAYER\n70\n");
    let _ = writeln!(out, "{}", layers.len().max(1));
    if layers.is_empty() {
        layer_row(&mut out, "0", 7);
    } else {
        for l in layers {
            layer_row(&mut out, &l.name, l.color);
        }
    }
    out.push_str("0\nENDTAB\n0\nENDSEC\n");
    // ENTITIES
    out.push_str("0\nSECTION\n2\nENTITIES\n");
    for e in entities {
        match e {
            DxfEntity::Line {
                layer,
                x1,
                y1,
                x2,
                y2,
            } => {
                out.push_str("0\nLINE\n");
                out.push_str(&format!("8\n{layer}\n"));
                out.push_str(&format!("10\n{x1}\n20\n{y1}\n30\n0.0\n"));
                out.push_str(&format!("11\n{x2}\n21\n{y2}\n31\n0.0\n"));
            }
            DxfEntity::Circle { layer, cx, cy, r } => {
                out.push_str("0\nCIRCLE\n");
                out.push_str(&format!("8\n{layer}\n"));
                out.push_str(&format!("10\n{cx}\n20\n{cy}\n30\n0.0\n"));
                out.push_str(&format!("40\n{r}\n"));
            }
            DxfEntity::Rect { layer, x, y, w, h } => {
                let pts = [
                    (*x, *y),
                    (*x + *w, *y),
                    (*x + *w, *y + *h),
                    (*x, *y + *h),
                    (*x, *y),
                ];
                for wdw in pts.windows(2) {
                    let (x1, y1) = wdw[0];
                    let (x2, y2) = wdw[1];
                    out.push_str("0\nLINE\n");
                    out.push_str(&format!("8\n{layer}\n"));
                    out.push_str(&format!("10\n{x1}\n20\n{y1}\n30\n0.0\n"));
                    out.push_str(&format!("11\n{x2}\n21\n{y2}\n31\n0.0\n"));
                }
            }
        }
    }
    out.push_str("0\nENDSEC\n0\nEOF\n");
    out
}

fn layer_row(out: &mut String, name: &str, color: i32) {
    out.push_str("0\nLAYER\n");
    out.push_str(&format!("2\n{name}\n"));
    out.push_str("70\n0\n");
    out.push_str(&format!("62\n{color}\n"));
    out.push_str("6\nCONTINUOUS\n");
}

/// Build a simple plate DXF: outer rect + hole circles (mm).
pub fn plate_with_holes_dxf(
    width: f64,
    height: f64,
    holes: &[(f64, f64, f64)], // cx, cy, diameter
) -> String {
    let layers = vec![
        DxfLayer {
            name: "OUTLINE".into(),
            color: 7,
        },
        DxfLayer {
            name: "HOLES".into(),
            color: 1,
        },
    ];
    let mut ents = vec![DxfEntity::Rect {
        layer: "OUTLINE".into(),
        x: 0.0,
        y: 0.0,
        w: width,
        h: height,
    }];
    for (cx, cy, d) in holes {
        ents.push(DxfEntity::Circle {
            layer: "HOLES".into(),
            cx: *cx,
            cy: *cy,
            r: d / 2.0,
        });
    }
    write_dxf_r12(&layers, &ents)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dxf_has_entities() {
        let s = plate_with_holes_dxf(100.0, 50.0, &[(25.0, 25.0, 6.0)]);
        assert!(s.contains("CIRCLE"));
        assert!(s.contains("LINE"));
        assert!(s.contains("$INSUNITS"));
        assert!(s.ends_with("EOF\n") || s.contains("EOF"));
    }
}
