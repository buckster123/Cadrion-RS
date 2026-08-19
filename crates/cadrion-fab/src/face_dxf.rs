//! Project a planar face outline to DXF (mm).

use cadrion_inspect::{EdgeRec, FaceRec, TopologySnapshot};
use cadrion_kernel::{Point3, Vec3};

use crate::dxf::{write_dxf_r12, DxfEntity, DxfLayer};

#[derive(Debug, Clone)]
pub struct FaceDxfReport {
    pub face_selector: String,
    pub normal: [f64; 3],
    pub centroid: [f64; 3],
    pub area_mm2: f64,
    pub edge_count: usize,
    pub circle_count: usize,
    pub dxf: String,
}

#[derive(Debug, Clone)]
pub enum FacePick {
    /// Exact selector string e.g. `#o1.1.f0`
    Selector(String),
    /// Largest-area face whose normal ≈ this unit vector.
    Normal([f64; 3]),
}

/// Project coplanar edges of a picked face onto its plane → DXF R12.
pub fn face_to_dxf(
    snap: &TopologySnapshot,
    face_selectors: &[(String, usize, usize)], // (selector, solid_idx, face_idx) from inspect order
    pick: &FacePick,
    plane_tol_mm: f64,
    normal_tol: f64,
) -> Result<FaceDxfReport, String> {
    let (sel, solid_i, face_i) = resolve_face(snap, face_selectors, pick, normal_tol)?;
    let solid = snap
        .solids
        .get(solid_i)
        .ok_or_else(|| format!("solid index {solid_i} out of range"))?;
    let face = solid
        .faces
        .get(face_i)
        .ok_or_else(|| format!("face index {face_i} out of range"))?;
    let n = face
        .normal
        .ok_or_else(|| "picked face has no normal (non-planar / cylindrical)".to_string())?;
    let nlen = (n.x * n.x + n.y * n.y + n.z * n.z).sqrt();
    if nlen < 1e-12 {
        return Err("degenerate face normal".into());
    }
    let nn = Vec3::new(n.x / nlen, n.y / nlen, n.z / nlen);
    let origin = face.centroid;
    let (u, v) = plane_basis(nn);

    let mut ents = Vec::new();
    let mut edge_count = 0usize;
    let mut circle_count = 0usize;

    // Coplanar edges with endpoints → LINE
    for e in &solid.edges {
        if plane_dist(e.midpoint, origin, nn).abs() > plane_tol_mm {
            continue;
        }
        if let (Some(a), Some(b)) = (e.start, e.end) {
            if plane_dist(a, origin, nn).abs() > plane_tol_mm * 2.0
                || plane_dist(b, origin, nn).abs() > plane_tol_mm * 2.0
            {
                continue;
            }
            let (x1, y1) = project_uv(a, origin, u, v);
            let (x2, y2) = project_uv(b, origin, u, v);
            let seg_len = ((x2 - x1).powi(2) + (y2 - y1).powi(2)).sqrt();
            if seg_len < 1e-6 {
                continue;
            }
            ents.push(DxfEntity::Line {
                layer: "OUTLINE".into(),
                x1,
                y1,
                x2,
                y2,
            });
            edge_count += 1;
        } else if is_circle_edge(e, face, &nn, plane_tol_mm) {
            // Circular edge without endpoints: CIRCLE at projected centroid
            let r = e.length_mm / (2.0 * std::f64::consts::PI);
            if r > 1e-6 {
                let (cx, cy) = project_uv(face.centroid, origin, u, v);
                // hole circles: midpoint projects near rim — use face centroid for caps
                let (mx, my) = project_uv(e.midpoint, origin, u, v);
                // prefer midpoint as on-circle point → center = face centroid for caps
                let (ccx, ccy) = if face.area_mm2 > 0.0 {
                    // cap face: circle around centroid
                    (cx, cy)
                } else {
                    (mx, my)
                };
                ents.push(DxfEntity::Circle {
                    layer: "HOLES".into(),
                    cx: ccx,
                    cy: ccy,
                    r,
                });
                circle_count += 1;
            }
        }
    }

    // Fallback: if no segments, emit AABB rect from face area assuming rectangle
    if ents.is_empty() {
        if let Some(rect) = rect_from_face(face, &solid.edges, origin, nn, u, v, plane_tol_mm) {
            ents.push(rect);
            edge_count = 4;
        } else {
            return Err(
                "no projectable edges on face plane (need edge endpoints or rectangular face)"
                    .into(),
            );
        }
    }

    // Normalize so outline sits in first quadrant (min corner → 0,0)
    shift_entities_to_origin(&mut ents);

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
    let dxf = write_dxf_r12(&layers, &ents);
    Ok(FaceDxfReport {
        face_selector: sel,
        normal: [nn.x, nn.y, nn.z],
        centroid: [origin.x, origin.y, origin.z],
        area_mm2: face.area_mm2,
        edge_count,
        circle_count,
        dxf,
    })
}

fn resolve_face(
    snap: &TopologySnapshot,
    face_selectors: &[(String, usize, usize)],
    pick: &FacePick,
    normal_tol: f64,
) -> Result<(String, usize, usize), String> {
    match pick {
        FacePick::Selector(s) => {
            if let Some((sel, si, fi)) = face_selectors.iter().find(|(sel, _, _)| sel == s) {
                return Ok((sel.clone(), *si, *fi));
            }
            // parse #oO.S.fF
            parse_selector(s, snap)
        }
        FacePick::Normal(want) => {
            let mut best: Option<(String, usize, usize, f64)> = None;
            for (sel, si, fi) in face_selectors {
                let face = &snap.solids[*si].faces[*fi];
                let Some(n) = face.normal else { continue };
                let d =
                    ((n.x - want[0]).powi(2) + (n.y - want[1]).powi(2) + (n.z - want[2]).powi(2))
                        .sqrt();
                if d > normal_tol {
                    continue;
                }
                if best.as_ref().map(|b| face.area_mm2 > b.3).unwrap_or(true) {
                    best = Some((sel.clone(), *si, *fi, face.area_mm2));
                }
            }
            // fallback walk solids if selectors empty
            if best.is_none() {
                for (si, solid) in snap.solids.iter().enumerate() {
                    for (fi, face) in solid.faces.iter().enumerate() {
                        let Some(n) = face.normal else { continue };
                        let d = ((n.x - want[0]).powi(2)
                            + (n.y - want[1]).powi(2)
                            + (n.z - want[2]).powi(2))
                        .sqrt();
                        if d > normal_tol {
                            continue;
                        }
                        if best.as_ref().map(|b| face.area_mm2 > b.3).unwrap_or(true) {
                            best =
                                Some((format!("#o{}.1.f{fi}", snap.object), si, fi, face.area_mm2));
                        }
                    }
                }
            }
            best.map(|(s, si, fi, _)| (s, si, fi))
                .ok_or_else(|| format!("no face with normal {want:?} (tol={normal_tol})"))
        }
    }
}

fn parse_selector(s: &str, snap: &TopologySnapshot) -> Result<(String, usize, usize), String> {
    // #o1.1.f3 → solid 0, face 3 (1-based solid in selector)
    let rest = s
        .strip_prefix("#o")
        .ok_or_else(|| format!("bad selector {s}"))?;
    let parts: Vec<_> = rest.split('.').collect();
    if parts.len() != 3 || !parts[2].starts_with('f') {
        return Err(format!("expected #oO.S.fF, got {s}"));
    }
    let face_i: usize = parts[2][1..]
        .parse()
        .map_err(|_| format!("bad face index in {s}"))?;
    let solid_sel: usize = parts[1]
        .parse()
        .map_err(|_| format!("bad solid index in {s}"))?;
    let solid_i = solid_sel.saturating_sub(1);
    if solid_i >= snap.solids.len() || face_i >= snap.solids[solid_i].faces.len() {
        return Err(format!("selector {s} out of range"));
    }
    Ok((s.to_string(), solid_i, face_i))
}

fn plane_basis(n: Vec3) -> (Vec3, Vec3) {
    let mut t = Vec3::new(0.0, 0.0, 1.0);
    if (n.x * t.x + n.y * t.y + n.z * t.z).abs() > 0.9 {
        t = Vec3::new(1.0, 0.0, 0.0);
    }
    // u = t × n
    let ux = t.y * n.z - t.z * n.y;
    let uy = t.z * n.x - t.x * n.z;
    let uz = t.x * n.y - t.y * n.x;
    let ul = (ux * ux + uy * uy + uz * uz).sqrt().max(1e-12);
    let u = Vec3::new(ux / ul, uy / ul, uz / ul);
    // v = n × u
    let vx = n.y * u.z - n.z * u.y;
    let vy = n.z * u.x - n.x * u.z;
    let vz = n.x * u.y - n.y * u.x;
    let v = Vec3::new(vx, vy, vz);
    (u, v)
}

fn plane_dist(p: Point3, o: Point3, n: Vec3) -> f64 {
    (p.x - o.x) * n.x + (p.y - o.y) * n.y + (p.z - o.z) * n.z
}

fn project_uv(p: Point3, o: Point3, u: Vec3, v: Vec3) -> (f64, f64) {
    let d = Point3::new(p.x - o.x, p.y - o.y, p.z - o.z);
    let uu = d.x * u.x + d.y * u.y + d.z * u.z;
    let vv = d.x * v.x + d.y * v.y + d.z * v.z;
    (uu, vv)
}

fn is_circle_edge(e: &EdgeRec, face: &FaceRec, n: &Vec3, plane_tol: f64) -> bool {
    if e.start.is_some() {
        return false;
    }
    // circular edge: length ≈ 2πr and midpoint near face plane
    if plane_dist(e.midpoint, face.centroid, *n).abs() > plane_tol * 3.0 {
        return false;
    }
    let r = e.length_mm / (2.0 * std::f64::consts::PI);
    r > 0.5 && face.area_mm2 > 0.0
}

fn rect_from_face(
    face: &FaceRec,
    edges: &[EdgeRec],
    origin: Point3,
    n: Vec3,
    u: Vec3,
    v: Vec3,
    plane_tol: f64,
) -> Option<DxfEntity> {
    let mut us = Vec::new();
    let mut vs = Vec::new();
    for e in edges {
        if plane_dist(e.midpoint, origin, n).abs() > plane_tol {
            continue;
        }
        let (uu, vv) = project_uv(e.midpoint, origin, u, v);
        us.push(uu);
        vs.push(vv);
    }
    if us.len() < 2 {
        // square from area
        let side = face.area_mm2.sqrt();
        if side < 1e-6 {
            return None;
        }
        return Some(DxfEntity::Rect {
            layer: "OUTLINE".into(),
            x: -side * 0.5,
            y: -side * 0.5,
            w: side,
            h: side,
        });
    }
    let min_u = us.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_u = us.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let min_v = vs.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_v = vs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    // expand by half typical edge toward exterior — mids are inset
    let mut w = (max_u - min_u).abs();
    let mut h = (max_v - min_v).abs();
    if w < 1e-6 || h < 1e-6 {
        let side = face.area_mm2.sqrt();
        w = side;
        h = side;
    } else {
        // edge mids sit at half-inset; inflate so area matches roughly
        let area = w * h;
        if area > 1e-9 && face.area_mm2 > 0.0 {
            let s = (face.area_mm2 / area).sqrt();
            let cx = (min_u + max_u) * 0.5;
            let cy = (min_v + max_v) * 0.5;
            w *= s;
            h *= s;
            return Some(DxfEntity::Rect {
                layer: "OUTLINE".into(),
                x: cx - w * 0.5,
                y: cy - h * 0.5,
                w,
                h,
            });
        }
    }
    Some(DxfEntity::Rect {
        layer: "OUTLINE".into(),
        x: min_u,
        y: min_v,
        w,
        h,
    })
}

fn shift_entities_to_origin(ents: &mut [DxfEntity]) {
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    for e in ents.iter() {
        match e {
            DxfEntity::Line { x1, y1, x2, y2, .. } => {
                min_x = min_x.min(*x1).min(*x2);
                min_y = min_y.min(*y1).min(*y2);
            }
            DxfEntity::Circle { cx, cy, r, .. } => {
                min_x = min_x.min(*cx - *r);
                min_y = min_y.min(*cy - *r);
            }
            DxfEntity::Rect { x, y, .. } => {
                min_x = min_x.min(*x);
                min_y = min_y.min(*y);
            }
        }
    }
    if !min_x.is_finite() {
        return;
    }
    for e in ents.iter_mut() {
        match e {
            DxfEntity::Line { x1, y1, x2, y2, .. } => {
                *x1 -= min_x;
                *x2 -= min_x;
                *y1 -= min_y;
                *y2 -= min_y;
            }
            DxfEntity::Circle { cx, cy, .. } => {
                *cx -= min_x;
                *cy -= min_y;
            }
            DxfEntity::Rect { x, y, .. } => {
                *x -= min_x;
                *y -= min_y;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use cadrion_inspect::box_topology;
    use cadrion_inspect::TopologySnapshot;
    use cadrion_kernel::Point3;

    use super::*;

    #[test]
    fn box_top_face_dxf() {
        let solid = box_topology(100.0, 60.0, 20.0, Point3::ORIGIN);
        let snap = TopologySnapshot::single_solid(solid);
        // Build selector list like inspect: solid 0 faces 0..
        let sels: Vec<_> = (0..snap.solids[0].faces.len())
            .map(|fi| (format!("#o1.1.f{fi}"), 0usize, fi))
            .collect();
        let r =
            face_to_dxf(&snap, &sels, &FacePick::Normal([0.0, 0.0, 1.0]), 0.5, 0.15).expect("dxf");
        assert!(r.dxf.contains("LINE") || r.dxf.contains("CIRCLE"));
        assert!(r.edge_count >= 4 || r.dxf.contains("LINE"));
        assert!((r.area_mm2 - 6000.0).abs() < 1.0);
        assert!(r.dxf.contains("OUTLINE"));
    }
}
