//! Experimental pure-Rust geometry kernel (H10).
//!
//! **Honesty:**
//! - `parity_eligible() == false` always — never Parity-10 claims
//! - never the CLI default (`mock` / optional `occt` remain)
//! - default: analytic CSG seed (not upstream truck)
//! - optional `brep` feature: upstream truck-modeling + shapeops (H3-6 spike)
//! - STEP I/O unsupported on seed; spike tessellate is real triangulation
//! - `parity_eligible() == false` always
//!
//! Seed for a future pure-Rust path — not a promotion candidate until OCCT still
//! wins agent loops and a real B-rep stack is wired.

#![deny(unsafe_code)]

use std::collections::HashMap;
use std::f64::consts::PI;
use std::path::Path;

use cadre_kernel::{
    BBox, BooleanOp, EdgeRef, GeomKernel, KernelError, KernelResult, Mesh, Placement, Point3,
    ShapeFacts, ShapeId, ShapeLabel, StepReadOpts, StepWriteOpts, TessTol, ValidityReport,
};

#[derive(Debug, Clone)]
enum Solid {
    Box {
        dx: f64,
        dy: f64,
        dz: f64,
        at: Point3,
        label: Option<String>,
    },
    Cylinder {
        radius: f64,
        height: f64,
        at: Point3,
        label: Option<String>,
    },
    Approx {
        volume_mm3: f64,
        bbox: BBox,
        faces: u32,
        edges: u32,
        label: Option<String>,
    },
}

/// Experimental pure-Rust kernel (non-parity).
#[derive(Debug, Default)]
pub struct TruckKernel {
    next_id: u64,
    shapes: HashMap<u64, Solid>,
}

impl TruckKernel {
    pub fn new() -> Self {
        Self::default()
    }

    fn alloc(&mut self, solid: Solid) -> ShapeId {
        self.next_id += 1;
        let id = self.next_id;
        self.shapes.insert(id, solid);
        ShapeId(id)
    }

    fn get(&self, id: ShapeId) -> KernelResult<&Solid> {
        self.shapes
            .get(&id.0)
            .ok_or_else(|| KernelError::unknown_shape(id))
    }

    fn union_bbox(a: BBox, b: BBox) -> BBox {
        BBox::from_min_max(
            Point3::new(
                a.min.x.min(b.min.x),
                a.min.y.min(b.min.y),
                a.min.z.min(b.min.z),
            ),
            Point3::new(
                a.max.x.max(b.max.x),
                a.max.y.max(b.max.y),
                a.max.z.max(b.max.z),
            ),
        )
    }

    fn facts_of(s: &Solid) -> ShapeFacts {
        match s {
            Solid::Box { dx, dy, dz, at, .. } => {
                let hx = dx / 2.0;
                let hy = dy / 2.0;
                let hz = dz / 2.0;
                ShapeFacts {
                    bbox_mm: BBox::from_min_max(
                        Point3::new(at.x - hx, at.y - hy, at.z - hz),
                        Point3::new(at.x + hx, at.y + hy, at.z + hz),
                    ),
                    volume_mm3: dx * dy * dz,
                    area_mm2: Some(2.0 * (dx * dy + dy * dz + dz * dx)),
                    centroid_mm: Some(*at),
                    solids: 1,
                    faces: 6,
                    edges: 12,
                    vertices: Some(8),
                    mass_g: None,
                }
            }
            Solid::Cylinder {
                radius, height, at, ..
            } => {
                let r = *radius;
                let h = *height;
                ShapeFacts {
                    bbox_mm: BBox::from_min_max(
                        Point3::new(at.x - r, at.y - r, at.z),
                        Point3::new(at.x + r, at.y + r, at.z + h),
                    ),
                    volume_mm3: PI * r * r * h,
                    area_mm2: Some(2.0 * PI * r * h + 2.0 * PI * r * r),
                    centroid_mm: Some(Point3::new(at.x, at.y, at.z + h * 0.5)),
                    solids: 1,
                    faces: 3,
                    edges: 2,
                    vertices: Some(0),
                    mass_g: None,
                }
            }
            Solid::Approx {
                volume_mm3,
                bbox,
                faces,
                edges,
                ..
            } => ShapeFacts {
                bbox_mm: *bbox,
                volume_mm3: *volume_mm3,
                area_mm2: None,
                centroid_mm: Some(bbox.center()),
                solids: 1,
                faces: *faces,
                edges: *edges,
                vertices: None,
                mass_g: None,
            },
        }
    }
}

impl GeomKernel for TruckKernel {
    fn backend_id(&self) -> &'static str {
        // H3-1: name is the CLI kernel id; implementation is analytic CSG seed, not upstream truck.
        "truck"
    }

    fn backend_version(&self) -> &str {
        concat!(
            "cadre-truck/",
            env!("CARGO_PKG_VERSION"),
            " (truck-seed: analytic CSG; NOT upstream truck BREP; NON-PARITY)"
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
        Ok(self.alloc(Solid::Box {
            dx,
            dy,
            dz,
            at: placement.origin,
            label: None,
        }))
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
        Ok(self.alloc(Solid::Cylinder {
            radius,
            height,
            at: placement.origin,
            label: None,
        }))
    }

    fn boolean(&mut self, op: BooleanOp, a: ShapeId, b: ShapeId) -> KernelResult<ShapeId> {
        let fa = Self::facts_of(self.get(a)?);
        let fb = Self::facts_of(self.get(b)?);
        let (vol, faces, edges) = match op {
            BooleanOp::Union => (
                fa.volume_mm3 + fb.volume_mm3,
                fa.faces.saturating_add(fb.faces),
                fa.edges.saturating_add(fb.edges),
            ),
            BooleanOp::Cut => ((fa.volume_mm3 - fb.volume_mm3).max(0.0), fa.faces, fa.edges),
            BooleanOp::Intersect => (
                fa.volume_mm3.min(fb.volume_mm3),
                fa.faces.min(fb.faces).max(1),
                fa.edges.min(fb.edges),
            ),
        };
        let bbox = Self::union_bbox(fa.bbox_mm, fb.bbox_mm);
        Ok(self.alloc(Solid::Approx {
            volume_mm3: vol,
            bbox,
            faces,
            edges,
            label: None,
        }))
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
        let s = self
            .shapes
            .get_mut(&shape.0)
            .ok_or_else(|| KernelError::unknown_shape(shape))?;
        let name = label.0;
        match s {
            Solid::Box { label, .. }
            | Solid::Cylinder { label, .. }
            | Solid::Approx { label, .. } => *label = Some(name),
        }
        Ok(shape)
    }

    fn facts(&self, shape: ShapeId) -> KernelResult<ShapeFacts> {
        Ok(Self::facts_of(self.get(shape)?))
    }

    fn validity(&self, shape: ShapeId) -> KernelResult<ValidityReport> {
        let f = Self::facts_of(self.get(shape)?);
        Ok(ValidityReport {
            closed: true,
            positive_volume: f.volume_mm3 > 0.0,
            shells: 1,
            notes: vec![
                "truck experimental: analytic CSG only — not solid B-rep".into(),
                "NON-PARITY".into(),
            ],
        })
    }

    fn edges(&self, shape: ShapeId) -> KernelResult<Vec<EdgeRef>> {
        let f = Self::facts_of(self.get(shape)?);
        Ok((0..f.edges).map(EdgeRef).collect())
    }

    fn write_step(&self, _shape: ShapeId, _path: &Path, _opts: &StepWriteOpts) -> KernelResult<()> {
        Err(KernelError::unsupported(self.backend_id(), "write_step"))
    }

    fn read_step(&mut self, _path: &Path, _opts: &StepReadOpts) -> KernelResult<ShapeId> {
        Err(KernelError::unsupported(self.backend_id(), "read_step"))
    }

    fn tessellate(&self, shape: ShapeId, _tol: TessTol) -> KernelResult<Mesh> {
        let f = Self::facts_of(self.get(shape)?);
        let b = f.bbox_mm;
        let (xmin, ymin, zmin) = (b.min.x as f32, b.min.y as f32, b.min.z as f32);
        let (xmax, ymax, zmax) = (b.max.x as f32, b.max.y as f32, b.max.z as f32);
        let positions = vec![
            xmin, ymin, zmin, xmax, ymin, zmin, xmax, ymax, zmin, xmin, ymax, zmin, xmin, ymin,
            zmax, xmax, ymin, zmax, xmax, ymax, zmax, xmin, ymax, zmax,
        ];
        let indices = vec![
            0, 1, 2, 0, 2, 3, 4, 6, 5, 4, 7, 6, 0, 4, 5, 0, 5, 1, 1, 5, 6, 1, 6, 2, 2, 6, 7, 2, 7,
            3, 3, 7, 4, 3, 4, 0,
        ];
        Ok(Mesh {
            positions,
            normals: None,
            indices,
        })
    }
}

#[cfg(feature = "brep")]
mod brep;
#[cfg(feature = "brep")]
pub use brep::TruckBrepKernel;

/// Crate version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Whether this build compiled the H3-6 upstream truck spike.
pub const BREP_SPIKE: bool = cfg!(feature = "brep");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn box_cyl_boolean_facts() {
        let mut k = TruckKernel::new();
        assert!(!k.parity_eligible());
        assert_eq!(k.backend_id(), "truck");
        let b = k
            .box_at(20.0, 10.0, 5.0, Point3::new(0.0, 0.0, 0.0))
            .unwrap();
        let c = k.cylinder_at(2.0, 5.0, Point3::new(0.0, 0.0, 0.0)).unwrap();
        let cut = k.boolean(BooleanOp::Cut, b, c).unwrap();
        let f = k.facts(cut).unwrap();
        assert!(f.volume_mm3 < 20.0 * 10.0 * 5.0);
        assert!(f.volume_mm3 > 0.0);
        let mesh = k.tessellate(cut, TessTol::default()).unwrap();
        assert_eq!(mesh.triangle_count(), 12);
        assert!(k.backend_version().contains("NON-PARITY"));
    }

    #[test]
    fn step_unsupported() {
        let mut k = TruckKernel::new();
        let b = k.box_at(1.0, 1.0, 1.0, Point3::ORIGIN).unwrap();
        assert!(k
            .write_step(b, Path::new("/tmp/x.step"), &StepWriteOpts::default())
            .is_err());
    }
}
