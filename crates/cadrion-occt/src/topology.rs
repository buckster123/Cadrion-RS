//! Build [`TopologySnapshot`] from live OCCT shapes.
//!
//! Uses **mesh clustering** (not Face::center_of_mass / normal_at_center), because those
//! helpers throw uncatchable C++ `StdFail_NotDone` on some boolean-cut faces via cxx.

use std::cmp::Ordering;
use std::collections::HashMap;

use cadrion_inspect::{EdgeRec, FaceRec, SolidRec, TopologySnapshot};
use cadrion_kernel::{GeomKernel, KernelResult, Point3, ShapeId, Vec3};
use glam::DVec3;
use opencascade::primitives::Shape;

use crate::OcctKernel;

impl OcctKernel {
    /// Topology snapshot from a live B-rep shape (for inspect/measure under `--kernel occt`).
    pub fn topology_snapshot(&self, shape: ShapeId) -> KernelResult<TopologySnapshot> {
        let s = self.get_pub(shape)?;
        let volume = self.facts(shape)?.volume_mm3;
        let solid = solid_from_mesh(s, volume);
        Ok(TopologySnapshot::single_solid(solid))
    }
}

fn solid_from_mesh(s: &Shape, volume_mm3: f64) -> SolidRec {
    let mesh = s.mesh();
    let verts = &mesh.vertices;
    let mut buckets: HashMap<[i32; 3], FaceAccum> = HashMap::new();

    for tri in mesh.indices.chunks_exact(3) {
        let a = verts[tri[0]];
        let b = verts[tri[1]];
        let c = verts[tri[2]];
        let e1 = b - a;
        let e2 = c - a;
        let n = e1.cross(e2);
        let area = n.length() * 0.5;
        if area < 1e-18 {
            continue;
        }
        let nn = n / n.length();
        let key = quantize_normal(nn);
        let centroid = (a + b + c) / 3.0;
        let entry = buckets.entry(key).or_default();
        entry.area += area;
        entry.cx += centroid.x * area;
        entry.cy += centroid.y * area;
        entry.cz += centroid.z * area;
        entry.nx += nn.x * area;
        entry.ny += nn.y * area;
        entry.nz += nn.z * area;
    }

    let mut faces: Vec<FaceRec> = buckets
        .into_values()
        .filter(|a| a.area > 1e-12)
        .map(|a| {
            let centroid = Point3::new(a.cx / a.area, a.cy / a.area, a.cz / a.area);
            let n = DVec3::new(a.nx, a.ny, a.nz);
            let nlen = n.length();
            let normal = if nlen > 1e-12 {
                Some(Vec3::new(n.x / nlen, n.y / nlen, n.z / nlen))
            } else {
                None
            };
            FaceRec {
                area_mm2: a.area,
                centroid,
                normal,
            }
        })
        .collect();

    // Stable-ish order for selectors: by centroid z, then y, then x
    faces.sort_by(|a, b| {
        cmp_f64(a.centroid.z, b.centroid.z)
            .then(cmp_f64(a.centroid.y, b.centroid.y))
            .then(cmp_f64(a.centroid.x, b.centroid.x))
    });

    // Unique mesh edges (capped) for inspect edge inventory
    let mut edge_keys: HashMap<(u32, u32), DVec3> = HashMap::new();
    for tri in mesh.indices.chunks_exact(3) {
        for &(i, j) in &[(tri[0], tri[1]), (tri[1], tri[2]), (tri[2], tri[0])] {
            let (lo, hi) = if i < j { (i, j) } else { (j, i) };
            let key = (lo as u32, hi as u32);
            edge_keys.entry(key).or_insert_with(|| {
                let pa = verts[lo];
                let pb = verts[hi];
                (pa + pb) * 0.5
            });
        }
    }
    let mut edges: Vec<EdgeRec> = edge_keys
        .into_iter()
        .take(512)
        .map(|((i, j), mid)| {
            let a = verts[i as usize];
            let b = verts[j as usize];
            EdgeRec {
                length_mm: (b - a).length(),
                midpoint: Point3::new(mid.x, mid.y, mid.z),
                start: Some(Point3::new(a.x, a.y, a.z)),
                end: Some(Point3::new(b.x, b.y, b.z)),
            }
        })
        .collect();
    edges.sort_by(|a, b| cmp_f64(a.midpoint.z, b.midpoint.z));

    let centroid = if faces.is_empty() {
        Point3::ORIGIN
    } else {
        let mut sx = 0.0;
        let mut sy = 0.0;
        let mut sz = 0.0;
        let mut w = 0.0;
        for f in &faces {
            sx += f.centroid.x * f.area_mm2;
            sy += f.centroid.y * f.area_mm2;
            sz += f.centroid.z * f.area_mm2;
            w += f.area_mm2;
        }
        if w > 0.0 {
            Point3::new(sx / w, sy / w, sz / w)
        } else {
            Point3::ORIGIN
        }
    };

    SolidRec {
        volume_mm3,
        centroid,
        faces,
        edges,
        vertices: Vec::new(),
    }
}

#[derive(Default)]
struct FaceAccum {
    area: f64,
    cx: f64,
    cy: f64,
    cz: f64,
    nx: f64,
    ny: f64,
    nz: f64,
}

fn quantize_normal(n: DVec3) -> [i32; 3] {
    // ~5.7° bins — axis-aligned box faces collapse cleanly to 6 buckets
    const S: f64 = 10.0;
    [
        (n.x * S).round() as i32,
        (n.y * S).round() as i32,
        (n.z * S).round() as i32,
    ]
}

fn cmp_f64(a: f64, b: f64) -> Ordering {
    a.partial_cmp(&b).unwrap_or(Ordering::Equal)
}
