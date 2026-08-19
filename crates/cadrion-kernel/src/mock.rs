//! Analytical mock kernel — exercises the trait without OCCT.
//!
//! **Not parity-eligible.** Fillet/chamfer/STEP/tessellate are honest `Unsupported`
//! (or no-op label-only) so agents never mistake mock geometry for real B-rep.

use std::collections::HashMap;
use std::f64::consts::PI;
use std::path::Path;

use crate::error::{KernelError, KernelResult};
use crate::facts::{ShapeFacts, ValidityReport};
use crate::handles::{EdgeRef, ShapeId, ShapeLabel};
use crate::kernel::GeomKernel;
use crate::mesh::Mesh;
use crate::step::{StepReadOpts, StepWriteOpts};
use crate::types::{BBox, BooleanOp, Placement, Point3, TessTol};

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
    Sphere {
        radius: f64,
        at: Point3,
        label: Option<String>,
    },
    /// Cone along +Z; base radius at `at`, height toward +Z.
    Cone {
        radius: f64,
        height: f64,
        at: Point3,
        label: Option<String>,
    },
    /// Boolean result with cached analytic approximation (not true B-rep).
    Approx {
        volume_mm3: f64,
        bbox: BBox,
        solids: u32,
        faces: u32,
        edges: u32,
        label: Option<String>,
    },
}

/// In-process mock backend for unit tests and offline agent dry-runs.
#[derive(Debug, Default)]
pub struct MockKernel {
    next_id: u64,
    shapes: HashMap<u64, Solid>,
}

impl MockKernel {
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

    fn facts_of(solid: &Solid) -> ShapeFacts {
        match solid {
            Solid::Box { dx, dy, dz, at, .. } => {
                let hx = dx / 2.0;
                let hy = dy / 2.0;
                let hz = dz / 2.0;
                let bbox = BBox::from_min_max(
                    Point3::new(at.x - hx, at.y - hy, at.z - hz),
                    Point3::new(at.x + hx, at.y + hy, at.z + hz),
                );
                ShapeFacts {
                    bbox_mm: bbox,
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
                // cylinder base at z=at.z, extends +Z by h; lateral center xy = at
                let bbox = BBox::from_min_max(
                    Point3::new(at.x - r, at.y - r, at.z),
                    Point3::new(at.x + r, at.y + r, at.z + h),
                );
                let vol = PI * r * r * h;
                let area = 2.0 * PI * r * h + 2.0 * PI * r * r;
                ShapeFacts {
                    bbox_mm: bbox,
                    volume_mm3: vol,
                    area_mm2: Some(area),
                    centroid_mm: Some(Point3::new(at.x, at.y, at.z + h * 0.5)),
                    solids: 1,
                    faces: 3,
                    edges: 2,
                    vertices: Some(0),
                    mass_g: None,
                }
            }
            Solid::Sphere { radius, at, .. } => {
                let r = *radius;
                let bbox = BBox::from_min_max(
                    Point3::new(at.x - r, at.y - r, at.z - r),
                    Point3::new(at.x + r, at.y + r, at.z + r),
                );
                let vol = 4.0 / 3.0 * PI * r * r * r;
                let area = 4.0 * PI * r * r;
                ShapeFacts {
                    bbox_mm: bbox,
                    volume_mm3: vol,
                    area_mm2: Some(area),
                    centroid_mm: Some(*at),
                    solids: 1,
                    faces: 1,
                    edges: 0,
                    vertices: Some(0),
                    mass_g: None,
                }
            }
            Solid::Cone {
                radius, height, at, ..
            } => {
                let r = *radius;
                let h = *height;
                let bbox = BBox::from_min_max(
                    Point3::new(at.x - r, at.y - r, at.z),
                    Point3::new(at.x + r, at.y + r, at.z + h),
                );
                let vol = PI * r * r * h / 3.0;
                let area = PI * r * (r + (r * r + h * h).sqrt());
                ShapeFacts {
                    bbox_mm: bbox,
                    volume_mm3: vol,
                    area_mm2: Some(area),
                    centroid_mm: Some(Point3::new(at.x, at.y, at.z + h * 0.25)),
                    solids: 1,
                    faces: 2,
                    edges: 1,
                    vertices: Some(0),
                    mass_g: None,
                }
            }
            Solid::Approx {
                volume_mm3,
                bbox,
                solids,
                faces,
                edges,
                ..
            } => ShapeFacts {
                bbox_mm: *bbox,
                volume_mm3: *volume_mm3,
                area_mm2: None,
                centroid_mm: Some(bbox.center()),
                solids: *solids,
                faces: *faces,
                edges: *edges,
                vertices: None,
                mass_g: None,
            },
        }
    }

    fn label_of(solid: &Solid) -> Option<String> {
        match solid {
            Solid::Box { label, .. }
            | Solid::Cylinder { label, .. }
            | Solid::Sphere { label, .. }
            | Solid::Cone { label, .. }
            | Solid::Approx { label, .. } => label.clone(),
        }
    }

    fn set_label_on(solid: &mut Solid, label: Option<String>) {
        match solid {
            Solid::Box { label: l, .. }
            | Solid::Cylinder { label: l, .. }
            | Solid::Sphere { label: l, .. }
            | Solid::Cone { label: l, .. }
            | Solid::Approx { label: l, .. } => *l = label,
        }
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

    fn require_positive(dim: f64, name: &str) -> KernelResult<()> {
        if !dim.is_finite() || dim <= 0.0 {
            return Err(KernelError::invalid_arg(format!(
                "{name} must be finite and > 0, got {dim}"
            )));
        }
        Ok(())
    }
}

impl GeomKernel for MockKernel {
    fn backend_id(&self) -> &'static str {
        "mock"
    }

    fn backend_version(&self) -> &str {
        "0.1.0-mock"
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
        Self::require_positive(dx, "dx")?;
        Self::require_positive(dy, "dy")?;
        Self::require_positive(dz, "dz")?;
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
        Self::require_positive(radius, "radius")?;
        Self::require_positive(height, "height")?;
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
        let (volume, bbox, solids) = match op {
            BooleanOp::Union => (
                fa.volume_mm3 + fb.volume_mm3, // overcount if overlap — mock honesty note in validity
                Self::union_bbox(fa.bbox_mm, fb.bbox_mm),
                fa.solids + fb.solids,
            ),
            BooleanOp::Cut => ((fa.volume_mm3 - fb.volume_mm3).max(0.0), fa.bbox_mm, 1),
            BooleanOp::Intersect => {
                // crude: min volume, intersection AABB if overlapping axis ranges
                let min = Point3::new(
                    fa.bbox_mm.min.x.max(fb.bbox_mm.min.x),
                    fa.bbox_mm.min.y.max(fb.bbox_mm.min.y),
                    fa.bbox_mm.min.z.max(fb.bbox_mm.min.z),
                );
                let max = Point3::new(
                    fa.bbox_mm.max.x.min(fb.bbox_mm.max.x),
                    fa.bbox_mm.max.y.min(fb.bbox_mm.max.y),
                    fa.bbox_mm.max.z.min(fb.bbox_mm.max.z),
                );
                let empty = min.x >= max.x || min.y >= max.y || min.z >= max.z;
                if empty {
                    (0.0, BBox::from_min_max(Point3::ORIGIN, Point3::ORIGIN), 0)
                } else {
                    let bb = BBox::from_min_max(min, max);
                    (fa.volume_mm3.min(fb.volume_mm3), bb, 1)
                }
            }
        };
        Ok(self.alloc(Solid::Approx {
            volume_mm3: volume,
            bbox,
            solids,
            faces: fa.faces.saturating_add(fb.faces),
            edges: fa.edges.saturating_add(fb.edges),
            label: None,
        }))
    }

    fn fillet(&mut self, shape: ShapeId, _edges: &[EdgeRef], radius: f64) -> KernelResult<ShapeId> {
        let _ = self.get(shape)?;
        Self::require_positive(radius, "radius")?;
        Err(KernelError::unsupported("mock", "fillet"))
    }

    fn chamfer(
        &mut self,
        shape: ShapeId,
        _edges: &[EdgeRef],
        distance: f64,
    ) -> KernelResult<ShapeId> {
        let _ = self.get(shape)?;
        Self::require_positive(distance, "distance")?;
        Err(KernelError::unsupported("mock", "chamfer"))
    }

    fn set_label(&mut self, shape: ShapeId, label: ShapeLabel) -> KernelResult<ShapeId> {
        let s = self
            .shapes
            .get_mut(&shape.0)
            .ok_or_else(|| KernelError::unknown_shape(shape))?;
        Self::set_label_on(s, Some(label.0));
        Ok(shape)
    }

    fn facts(&self, shape: ShapeId) -> KernelResult<ShapeFacts> {
        Ok(Self::facts_of(self.get(shape)?))
    }

    fn validity(&self, shape: ShapeId) -> KernelResult<ValidityReport> {
        let s = self.get(shape)?;
        let f = Self::facts_of(s);
        let mut notes = Vec::new();
        if matches!(s, Solid::Approx { .. }) {
            notes.push("mock boolean volumes are analytic approximations, not B-rep".into());
        }
        if let Some(l) = Self::label_of(s) {
            notes.push(format!("label={l}"));
        }
        Ok(ValidityReport {
            closed: f.volume_mm3 > 0.0,
            positive_volume: f.volume_mm3 > 0.0,
            shells: if f.solids > 0 { 1 } else { 0 },
            notes,
        })
    }

    fn edges(&self, shape: ShapeId) -> KernelResult<Vec<EdgeRef>> {
        let f = Self::facts_of(self.get(shape)?);
        Ok((0..f.edges).map(EdgeRef).collect())
    }

    fn write_step(&self, shape: ShapeId, _path: &Path, _opts: &StepWriteOpts) -> KernelResult<()> {
        let _ = self.get(shape)?;
        Err(KernelError::unsupported("mock", "write_step"))
    }

    fn read_step(&mut self, _path: &Path, _opts: &StepReadOpts) -> KernelResult<ShapeId> {
        Err(KernelError::unsupported("mock", "read_step"))
    }

    fn tessellate(&self, shape: ShapeId, _tol: TessTol) -> KernelResult<Mesh> {
        let _ = self.get(shape)?;
        Err(KernelError::unsupported("mock", "tessellate"))
    }

    fn translate(&mut self, shape: ShapeId, dx: f64, dy: f64, dz: f64) -> KernelResult<ShapeId> {
        let s = self.get(shape)?.clone();
        Ok(self.alloc(Self::translate_solid(s, dx, dy, dz)))
    }

    fn rotate_about_axis(&mut self, shape: ShapeId, axis: &str, deg: f64) -> KernelResult<ShapeId> {
        if !deg.is_finite() {
            return Err(KernelError::invalid_arg("deg must be finite"));
        }
        let ax = axis.to_ascii_lowercase();
        if ax != "x" && ax != "y" && ax != "z" {
            return Err(KernelError::invalid_arg(
                "axis must be \"x\", \"y\", or \"z\"",
            ));
        }
        let s = self.get(shape)?.clone();
        Ok(self.alloc(Self::rotate_solid(s, &ax, deg)))
    }

    fn sphere(&mut self, radius: f64, placement: Placement) -> KernelResult<ShapeId> {
        Self::require_positive(radius, "radius")?;
        Ok(self.alloc(Solid::Sphere {
            radius,
            at: placement.origin,
            label: None,
        }))
    }

    fn cone(&mut self, radius: f64, height: f64, placement: Placement) -> KernelResult<ShapeId> {
        Self::require_positive(radius, "radius")?;
        Self::require_positive(height, "height")?;
        Ok(self.alloc(Solid::Cone {
            radius,
            height,
            at: placement.origin,
            label: None,
        }))
    }

    fn mirror_plane(&mut self, shape: ShapeId, plane: &str) -> KernelResult<ShapeId> {
        let pl = plane.to_ascii_lowercase();
        if pl != "xy" && pl != "yz" && pl != "zx" && pl != "xz" {
            return Err(KernelError::invalid_arg(
                "plane must be \"xy\", \"yz\", or \"zx\"",
            ));
        }
        let s = self.get(shape)?.clone();
        Ok(self.alloc(mirror_solid(s, &pl)))
    }
}

impl MockKernel {
    fn translate_solid(s: Solid, dx: f64, dy: f64, dz: f64) -> Solid {
        match s {
            Solid::Box {
                dx: bx,
                dy: by,
                dz: bz,
                at,
                label,
            } => Solid::Box {
                dx: bx,
                dy: by,
                dz: bz,
                at: Point3::new(at.x + dx, at.y + dy, at.z + dz),
                label,
            },
            Solid::Cylinder {
                radius,
                height,
                at,
                label,
            } => Solid::Cylinder {
                radius,
                height,
                at: Point3::new(at.x + dx, at.y + dy, at.z + dz),
                label,
            },
            Solid::Sphere { radius, at, label } => Solid::Sphere {
                radius,
                at: Point3::new(at.x + dx, at.y + dy, at.z + dz),
                label,
            },
            Solid::Cone {
                radius,
                height,
                at,
                label,
            } => Solid::Cone {
                radius,
                height,
                at: Point3::new(at.x + dx, at.y + dy, at.z + dz),
                label,
            },
            Solid::Approx {
                volume_mm3,
                bbox,
                solids,
                faces,
                edges,
                label,
            } => Solid::Approx {
                volume_mm3,
                bbox: BBox::from_min_max(
                    Point3::new(bbox.min.x + dx, bbox.min.y + dy, bbox.min.z + dz),
                    Point3::new(bbox.max.x + dx, bbox.max.y + dy, bbox.max.z + dz),
                ),
                solids,
                faces,
                edges,
                label,
            },
        }
    }

    fn rotate_solid(s: Solid, axis: &str, deg: f64) -> Solid {
        // Preserve volume; collapse boxes to Approx after rotate (rotated AABB).
        let f = Self::facts_of(&s);
        let label = Self::label_of(&s);
        // Special-case: pure Z-rotate of Z-cylinder keeps cylinder analytic if base center rotates in XY
        if let Solid::Cylinder {
            radius,
            height,
            at,
            label,
        } = &s
        {
            if axis == "z" {
                let (x, y, z) = rot_point(at.x, at.y, at.z, axis, deg);
                return Solid::Cylinder {
                    radius: *radius,
                    height: *height,
                    at: Point3::new(x, y, z),
                    label: label.clone(),
                };
            }
        }
        if let Solid::Box {
            dx,
            dy,
            dz,
            at,
            label,
        } = &s
        {
            // Rotate 8 corners of AABB then re-AABB (volume preserved)
            let hx = dx / 2.0;
            let hy = dy / 2.0;
            let hz = dz / 2.0;
            let corners = [
                [at.x - hx, at.y - hy, at.z - hz],
                [at.x + hx, at.y - hy, at.z - hz],
                [at.x - hx, at.y + hy, at.z - hz],
                [at.x + hx, at.y + hy, at.z - hz],
                [at.x - hx, at.y - hy, at.z + hz],
                [at.x + hx, at.y - hy, at.z + hz],
                [at.x - hx, at.y + hy, at.z + hz],
                [at.x + hx, at.y + hy, at.z + hz],
            ];
            let mut min = Point3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY);
            let mut max = Point3::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
            for c in corners {
                let (x, y, z) = rot_point(c[0], c[1], c[2], axis, deg);
                min.x = min.x.min(x);
                min.y = min.y.min(y);
                min.z = min.z.min(z);
                max.x = max.x.max(x);
                max.y = max.y.max(y);
                max.z = max.z.max(z);
            }
            return Solid::Approx {
                volume_mm3: dx * dy * dz,
                bbox: BBox::from_min_max(min, max),
                solids: 1,
                faces: 6,
                edges: 12,
                label: label.clone(),
            };
        }
        // Approx / other: rotate bbox corners
        let bb = f.bbox_mm;
        let corners = [
            [bb.min.x, bb.min.y, bb.min.z],
            [bb.max.x, bb.min.y, bb.min.z],
            [bb.min.x, bb.max.y, bb.min.z],
            [bb.max.x, bb.max.y, bb.min.z],
            [bb.min.x, bb.min.y, bb.max.z],
            [bb.max.x, bb.min.y, bb.max.z],
            [bb.min.x, bb.max.y, bb.max.z],
            [bb.max.x, bb.max.y, bb.max.z],
        ];
        let mut min = Point3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY);
        let mut max = Point3::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
        for c in corners {
            let (x, y, z) = rot_point(c[0], c[1], c[2], axis, deg);
            min.x = min.x.min(x);
            min.y = min.y.min(y);
            min.z = min.z.min(z);
            max.x = max.x.max(x);
            max.y = max.y.max(y);
            max.z = max.z.max(z);
        }
        Solid::Approx {
            volume_mm3: f.volume_mm3,
            bbox: BBox::from_min_max(min, max),
            solids: f.solids,
            faces: f.faces,
            edges: f.edges,
            label,
        }
    }
}

fn mirror_solid(s: Solid, plane: &str) -> Solid {
    // Reflect placement / bbox through coordinate plane through origin.
    let flip = |p: Point3| match plane {
        "yz" => Point3::new(-p.x, p.y, p.z),
        "zx" | "xz" => Point3::new(p.x, -p.y, p.z),
        _ => Point3::new(p.x, p.y, -p.z), // xy
    };
    match s {
        Solid::Box {
            dx,
            dy,
            dz,
            at,
            label,
        } => Solid::Box {
            dx,
            dy,
            dz,
            at: flip(at),
            label,
        },
        Solid::Cylinder {
            radius,
            height,
            at,
            label,
        } => {
            // Base point flips; height still +Z (mirror of Z-axis cylinder about xy flips base).
            let at2 = flip(at);
            let (at3, h) = if plane == "xy" {
                // base was at.z, extends +h; after z→-z base becomes at.z+h in old coords → new base at -at.z-h, height still +h toward -old
                (Point3::new(at.x, at.y, -(at.z + height)), height)
            } else {
                (at2, height)
            };
            Solid::Cylinder {
                radius,
                height: h,
                at: at3,
                label,
            }
        }
        Solid::Sphere { radius, at, label } => Solid::Sphere {
            radius,
            at: flip(at),
            label,
        },
        Solid::Cone {
            radius,
            height,
            at,
            label,
        } => {
            let at2 = flip(at);
            let (at3, h) = if plane == "xy" {
                (Point3::new(at.x, at.y, -(at.z + height)), height)
            } else {
                (at2, height)
            };
            Solid::Cone {
                radius,
                height: h,
                at: at3,
                label,
            }
        }
        Solid::Approx {
            volume_mm3,
            bbox,
            solids,
            faces,
            edges,
            label,
        } => {
            let c1 = flip(bbox.min);
            let c2 = flip(bbox.max);
            let min = Point3::new(c1.x.min(c2.x), c1.y.min(c2.y), c1.z.min(c2.z));
            let max = Point3::new(c1.x.max(c2.x), c1.y.max(c2.y), c1.z.max(c2.z));
            Solid::Approx {
                volume_mm3,
                bbox: BBox::from_min_max(min, max),
                solids,
                faces,
                edges,
                label,
            }
        }
    }
}

fn rot_point(x: f64, y: f64, z: f64, axis: &str, deg: f64) -> (f64, f64, f64) {
    let r = deg.to_radians();
    let (c, s) = (r.cos(), r.sin());
    match axis {
        "x" => (x, y * c - z * s, y * s + z * c),
        "y" => (x * c + z * s, y, -x * s + z * c),
        _ => (x * c - y * s, x * s + y * c, z), // z
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Density;

    #[test]
    fn box_volume_and_bbox() {
        let mut k = MockKernel::new();
        let s = k.box_solid(100.0, 60.0, 20.0, Placement::IDENTITY).unwrap();
        let f = k.facts(s).unwrap();
        assert!((f.volume_mm3 - 120_000.0).abs() < 1e-9);
        assert_eq!(f.solids, 1);
        assert_eq!(f.faces, 6);
        assert_eq!(f.edges, 12);
        let e = f.bbox_mm.extents_mm();
        assert!((e[0] - 100.0).abs() < 1e-12);
        assert!((e[1] - 60.0).abs() < 1e-12);
        assert!((e[2] - 20.0).abs() < 1e-12);
    }

    #[test]
    fn cylinder_volume() {
        let mut k = MockKernel::new();
        let s = k.cylinder(10.0, 50.0, Placement::IDENTITY).unwrap();
        let f = k.facts(s).unwrap();
        let expect = PI * 100.0 * 50.0;
        assert!((f.volume_mm3 - expect).abs() < 1e-9);
    }

    #[test]
    fn cut_reduces_volume() {
        let mut k = MockKernel::new();
        let a = k.box_at(100.0, 60.0, 20.0, Point3::ORIGIN).unwrap();
        let b = k
            .cylinder_at(4.0, 22.0, Point3::new(0.0, 0.0, -1.0))
            .unwrap();
        let cut = k.boolean(BooleanOp::Cut, a, b).unwrap();
        let fa = k.facts(a).unwrap().volume_mm3;
        let fb = k.facts(b).unwrap().volume_mm3;
        let fc = k.facts(cut).unwrap().volume_mm3;
        assert!((fc - (fa - fb).max(0.0)).abs() < 1e-9);
    }

    #[test]
    fn fillet_honestly_unsupported() {
        let mut k = MockKernel::new();
        let s = k.box_at(10.0, 10.0, 10.0, Point3::ORIGIN).unwrap();
        let err = k.fillet(s, &[], 1.0).unwrap_err();
        assert_eq!(err.code(), "CADRION-E-UNSUPPORTED");
    }

    #[test]
    fn mass_from_density() {
        let mut k = MockKernel::new();
        let s = k.box_at(10.0, 10.0, 10.0, Point3::ORIGIN).unwrap(); // 1000 mm³ = 1 cm³
        let f = k.facts_with_density(s, Density::g_per_cm3(2.7)).unwrap();
        assert!((f.mass_g.unwrap() - 2.7).abs() < 1e-9);
    }

    #[test]
    fn rejects_non_positive_dims() {
        let mut k = MockKernel::new();
        assert!(k.box_at(0.0, 1.0, 1.0, Point3::ORIGIN).is_err());
        assert!(k.cylinder_at(-1.0, 5.0, Point3::ORIGIN).is_err());
    }

    #[test]
    fn label_and_edges() {
        let mut k = MockKernel::new();
        let s = k.box_at(1.0, 2.0, 3.0, Point3::ORIGIN).unwrap();
        k.set_label(s, ShapeLabel::new("block")).unwrap();
        let v = k.validity(s).unwrap();
        assert!(v.notes.iter().any(|n| n.contains("block")));
        assert_eq!(k.edges(s).unwrap().len(), 12);
    }

    #[test]
    fn not_parity_eligible() {
        assert!(!MockKernel::new().parity_eligible());
    }
}
