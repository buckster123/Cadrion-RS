//! H3-6 spike: **upstream** `truck` B-rep (Apache-2.0).
//!
//! Scope: box + cylinder + boolean (and/or/cut) + tessellate.
//! STEP / fillet / chamfer remain Unsupported (H5-10: pinned crates have no STEP).
//! `parity_eligible() == false`.
//! Not the CLI default.

use std::collections::HashMap;
use std::path::Path;

use cadrion_kernel::{
    BBox, BooleanOp, EdgeRef, GeomKernel, KernelError, KernelResult, Mesh, Placement, Point3,
    ShapeFacts, ShapeId, ShapeLabel, StepReadOpts, StepWriteOpts, TessTol, ValidityReport,
};
use truck_meshalgo::prelude::*;
use truck_modeling::builder;
use truck_modeling::{Curve, Point3 as TPoint3, Rad, Solid, Vector3};

/// Upstream-truck kernel (NON-PARITY).
#[derive(Debug, Default)]
pub struct TruckBrepKernel {
    next_id: u64,
    shapes: HashMap<u64, Entry>,
}

#[derive(Debug, Clone)]
struct Entry {
    solid: Solid,
    label: Option<String>,
}

impl TruckBrepKernel {
    pub fn new() -> Self {
        Self::default()
    }

    fn alloc(&mut self, solid: Solid) -> ShapeId {
        self.next_id += 1;
        let id = self.next_id;
        self.shapes.insert(id, Entry { solid, label: None });
        ShapeId(id)
    }

    fn get(&self, id: ShapeId) -> KernelResult<&Entry> {
        self.shapes
            .get(&id.0)
            .ok_or_else(|| KernelError::unknown_shape(id))
    }

    fn get_mut(&mut self, id: ShapeId) -> KernelResult<&mut Entry> {
        self.shapes
            .get_mut(&id.0)
            .ok_or_else(|| KernelError::unknown_shape(id))
    }

    fn box_solid_inner(dx: f64, dy: f64, dz: f64, at: Point3) -> Solid {
        // Cadrion convention: box centered at `at`.
        let o = TPoint3::new(at.x - dx * 0.5, at.y - dy * 0.5, at.z - dz * 0.5);
        let v = builder::vertex(o);
        let e = builder::tsweep(&v, Vector3::new(dx, 0.0, 0.0));
        let f = builder::tsweep(&e, Vector3::new(0.0, dy, 0.0));
        builder::tsweep(&f, Vector3::new(0.0, 0.0, dz))
    }

    fn cylinder_inner(radius: f64, height: f64, at: Point3) -> KernelResult<Solid> {
        // Base disk at `at`, +Z height. rsweep angle > 2π closes the wire.
        let v = builder::vertex(TPoint3::new(at.x + radius, at.y, at.z));
        let axis = TPoint3::new(at.x, at.y, at.z);
        let wire = builder::rsweep(&v, axis, Vector3::unit_z(), Rad(7.0));
        let face = builder::try_attach_plane(&[wire]).map_err(|e| {
            KernelError::diagnostic(
                "CADRION-E-KERNEL",
                format!("truck cylinder attach_plane: {e}"),
                None,
            )
        })?;
        Ok(builder::tsweep(&face, Vector3::unit_z() * height))
    }

    fn tess_tol(tol: TessTol) -> f64 {
        // truck panics if tol ≤ TOLERANCE (~1e-6). Cadrion TessTol is mm-ish.
        let t = if tol.linear_mm.is_finite() && tol.linear_mm > 0.0 {
            tol.linear_mm
        } else {
            0.5
        };
        t.max(1.0e-3)
    }

    fn polygon(solid: &Solid, tol: f64) -> KernelResult<PolygonMesh> {
        Ok(solid.triangulation(tol).to_polygon())
    }

    fn mesh_from_polygon(poly: &PolygonMesh) -> Mesh {
        let positions: Vec<f32> = poly
            .positions()
            .iter()
            .flat_map(|p| [p.x as f32, p.y as f32, p.z as f32])
            .collect();
        let mut indices = Vec::new();
        for face in poly.faces().tri_faces() {
            // StandardVertex: pos / uv / nor indices
            indices.push(face[0].pos as u32);
            indices.push(face[1].pos as u32);
            indices.push(face[2].pos as u32);
        }
        Mesh {
            positions,
            normals: None,
            indices,
        }
    }

    fn signed_volume_mm3(mesh: &Mesh) -> f64 {
        let p = &mesh.positions;
        let mut acc = 0.0_f64;
        for tri in mesh.indices.as_chunks::<3>().0 {
            let ia = (tri[0] as usize) * 3;
            let ib = (tri[1] as usize) * 3;
            let ic = (tri[2] as usize) * 3;
            if ic + 2 >= p.len() {
                continue;
            }
            let ax = p[ia] as f64;
            let ay = p[ia + 1] as f64;
            let az = p[ia + 2] as f64;
            let bx = p[ib] as f64;
            let by = p[ib + 1] as f64;
            let bz = p[ib + 2] as f64;
            let cx = p[ic] as f64;
            let cy = p[ic + 1] as f64;
            let cz = p[ic + 2] as f64;
            acc += ax * (by * cz - bz * cy) - ay * (bx * cz - bz * cx) + az * (bx * cy - by * cx);
        }
        (acc / 6.0).abs()
    }

    fn bbox_of(mesh: &Mesh) -> BBox {
        let mut xmin = f64::INFINITY;
        let mut ymin = f64::INFINITY;
        let mut zmin = f64::INFINITY;
        let mut xmax = f64::NEG_INFINITY;
        let mut ymax = f64::NEG_INFINITY;
        let mut zmax = f64::NEG_INFINITY;
        for c in mesh.positions.as_chunks::<3>().0 {
            xmin = xmin.min(c[0] as f64);
            ymin = ymin.min(c[1] as f64);
            zmin = zmin.min(c[2] as f64);
            xmax = xmax.max(c[0] as f64);
            ymax = ymax.max(c[1] as f64);
            zmax = zmax.max(c[2] as f64);
        }
        if !xmin.is_finite() {
            return BBox::from_min_max(Point3::ORIGIN, Point3::new(1.0, 1.0, 1.0));
        }
        BBox::from_min_max(Point3::new(xmin, ymin, zmin), Point3::new(xmax, ymax, zmax))
    }

    fn facts_of(solid: &Solid) -> KernelResult<ShapeFacts> {
        let mesh = Self::mesh_from_polygon(&Self::polygon(solid, 0.5)?);
        let vol = Self::signed_volume_mm3(&mesh);
        let faces = solid.boundaries().iter().map(|sh| sh.len()).sum::<usize>() as u32;
        Ok(ShapeFacts {
            bbox_mm: Self::bbox_of(&mesh),
            volume_mm3: vol,
            area_mm2: None,
            centroid_mm: Some(Self::bbox_of(&mesh).center()),
            solids: 1,
            faces,
            edges: 0,
            vertices: Some((mesh.positions.len() / 3) as u32),
            mass_g: None,
        })
    }
}

impl GeomKernel for TruckBrepKernel {
    fn backend_id(&self) -> &'static str {
        "truck-brep"
    }

    fn backend_version(&self) -> &str {
        concat!(
            "cadrion-truck/",
            env!("CARGO_PKG_VERSION"),
            " (upstream truck-modeling 0.6 + shapeops 0.4; H3-6 spike; NON-PARITY)"
        )
    }

    fn parity_eligible(&self) -> bool {
        false
    }

    fn box_solid(
        &mut self,
        dx: f64,
        dy: f64,
        dz: f64,
        placement: Placement,
    ) -> KernelResult<ShapeId> {
        if ![dx, dy, dz].into_iter().all(|v| v.is_finite() && v > 0.0) {
            return Err(KernelError::invalid_arg("box dims must be finite > 0"));
        }
        Ok(self.alloc(Self::box_solid_inner(dx, dy, dz, placement.origin)))
    }

    fn cylinder(
        &mut self,
        radius: f64,
        height: f64,
        placement: Placement,
    ) -> KernelResult<ShapeId> {
        if !(radius.is_finite() && radius > 0.0 && height.is_finite() && height > 0.0) {
            return Err(KernelError::invalid_arg(
                "cylinder radius/height must be finite > 0",
            ));
        }
        Ok(self.alloc(Self::cylinder_inner(radius, height, placement.origin)?))
    }

    fn boolean(&mut self, op: BooleanOp, a: ShapeId, b: ShapeId) -> KernelResult<ShapeId> {
        let sa = self.get(a)?.solid.clone();
        let sb = self.get(b)?.solid.clone();
        let tol = 0.05;
        let out = match op {
            BooleanOp::Union => truck_shapeops::or(&sa, &sb, tol),
            BooleanOp::Intersect => truck_shapeops::and(&sa, &sb, tol),
            BooleanOp::Cut => {
                let mut inv = sb;
                inv.not();
                truck_shapeops::and(&sa, &inv, tol)
            }
        };
        match out {
            Some(s) => Ok(self.alloc(s)),
            None => Err(KernelError::diagnostic(
                "CADRION-E-KERNEL",
                format!("truck-shapeops {op:?} returned None (no intersection / failed classify)"),
                Some("try simpler overlap or larger solids; spike is experimental".into()),
            )),
        }
    }

    fn fillet(
        &mut self,
        _shape: ShapeId,
        _edges: &[EdgeRef],
        _radius: f64,
    ) -> KernelResult<ShapeId> {
        Err(KernelError::unsupported(self.backend_id(), "fillet"))
    }

    fn chamfer(
        &mut self,
        _shape: ShapeId,
        _edges: &[EdgeRef],
        _distance: f64,
    ) -> KernelResult<ShapeId> {
        Err(KernelError::unsupported(self.backend_id(), "chamfer"))
    }

    fn set_label(&mut self, shape: ShapeId, label: ShapeLabel) -> KernelResult<ShapeId> {
        self.get_mut(shape)?.label = Some(label.0);
        Ok(shape)
    }

    fn facts(&self, shape: ShapeId) -> KernelResult<ShapeFacts> {
        Self::facts_of(&self.get(shape)?.solid)
    }

    fn validity(&self, shape: ShapeId) -> KernelResult<ValidityReport> {
        let s = &self.get(shape)?.solid;
        let f = Self::facts_of(s)?;
        Ok(ValidityReport {
            closed: s.is_geometric_consistent(),
            positive_volume: f.volume_mm3 > 0.0,
            shells: s.boundaries().len() as u32,
            notes: vec![
                "H3-6 truck-brep: real B-rep via truck-shapeops — still NON-PARITY".into(),
                "STEP write not in this spike".into(),
            ],
        })
    }

    fn edges(&self, shape: ShapeId) -> KernelResult<Vec<EdgeRef>> {
        let n = self.get(shape)?.solid.edge_iter().count();
        Ok((0..n as u32).map(EdgeRef).collect())
    }

    fn write_step(&self, _shape: ShapeId, _path: &Path, _opts: &StepWriteOpts) -> KernelResult<()> {
        // H5-10 / G1: truck-modeling/shapeops/meshalgo do not write STEP. No truck-stepio pin.
        Err(KernelError::unsupported(self.backend_id(), "write_step"))
    }

    fn read_step(&mut self, _path: &Path, _opts: &StepReadOpts) -> KernelResult<ShapeId> {
        Err(KernelError::unsupported(self.backend_id(), "read_step"))
    }

    fn tessellate(&self, shape: ShapeId, tol: TessTol) -> KernelResult<Mesh> {
        let solid = &self.get(shape)?.solid;
        let poly = Self::polygon(solid, Self::tess_tol(tol))?;
        let mesh = Self::mesh_from_polygon(&poly);
        if mesh.triangle_count() == 0 {
            return Err(KernelError::diagnostic(
                "CADRION-E-KERNEL",
                "truck tessellate produced 0 triangles",
                None,
            ));
        }
        Ok(mesh)
    }
}

// Silence unused Curve import if rustc complains — used by truck types transitively.
#[allow(dead_code)]
fn _curve_marker(_: &Curve) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn box_tessellate_not_bbox() {
        let mut k = TruckBrepKernel::new();
        let b = k
            .box_at(10.0, 10.0, 10.0, Point3::new(5.0, 5.0, 5.0))
            .unwrap();
        let mesh = k.tessellate(b, TessTol::default()).unwrap();
        assert!(
            mesh.triangle_count() >= 12,
            "box mesh tris {}",
            mesh.triangle_count()
        );
        let f = k.facts(b).unwrap();
        assert!((f.volume_mm3 - 1000.0).abs() < 50.0, "vol {}", f.volume_mm3);
    }

    #[test]
    fn box_cut_mesh_not_bbox() {
        let mut k = TruckBrepKernel::new();
        assert!(!k.parity_eligible());
        assert_eq!(k.backend_id(), "truck-brep");
        // Unit-scale overlap like truck-shapeops punched-cube example:
        // cube [0,10]^3 and a through-hole cylinder.
        let b = k
            .box_at(10.0, 10.0, 10.0, Point3::new(5.0, 5.0, 5.0))
            .unwrap();
        let c = k
            .cylinder_at(2.0, 12.0, Point3::new(5.0, 5.0, -1.0))
            .unwrap();
        let cut = k.boolean(BooleanOp::Cut, b, c).unwrap();
        let f = k.facts(cut).unwrap();
        let box_vol = 1000.0;
        assert!(
            f.volume_mm3 < box_vol * 0.98,
            "cut volume {} should be below box {}",
            f.volume_mm3,
            box_vol
        );
        assert!(
            f.volume_mm3 > box_vol * 0.4,
            "cut vanished? {}",
            f.volume_mm3
        );
        let mesh = k.tessellate(cut, TessTol::default()).unwrap();
        assert!(
            mesh.triangle_count() > 12,
            "expected non-bbox tessellation, got {} tris",
            mesh.triangle_count()
        );
        assert!(k.backend_version().contains("NON-PARITY"));
    }

    #[test]
    fn step_still_unsupported() {
        let mut k = TruckBrepKernel::new();
        assert!(!k.parity_eligible());
        let b = k.box_at(1.0, 1.0, 1.0, Point3::ORIGIN).unwrap();
        let path =
            std::env::temp_dir().join(format!("cadrion-h5-10-brep-{}.step", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let err = k
            .write_step(b, &path, &StepWriteOpts::default())
            .unwrap_err();
        assert_eq!(err.code(), "CADRION-E-UNSUPPORTED");
        assert!(!path.exists(), "refuse must not write a STEP file");
    }
}
