//! Open CASCADE Technology backend for [`cadrion_kernel::GeomKernel`].
//!
//! Links LGPL OCCT via the `opencascade` crate. Not part of default CI
//! (`cargo test --workspace --exclude cadrion-occt`). See `docs/occt-binding.md`.

mod topology;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use cadrion_kernel::{
    BBox, BooleanOp, EdgeRef, GeomKernel, KernelError, KernelResult, Mesh, Placement, Point3,
    ShapeFacts, ShapeId, ShapeLabel, StepReadOpts, StepWriteOpts, TessTol, ValidityReport,
};
use glam::dvec3;
use opencascade::adhoc::AdHocShape;
use opencascade::primitives::{IntoShape, Shape};

/// OCCT-backed geometry kernel.
///
/// # Thread safety
///
/// `opencascade::Shape` is `!Send` because cxx unique pointers are not auto-Send.
/// Each `OcctKernel` is still safe to move between threads if **one thread at a time**
/// owns and mutates it (no shared interior mutability). HTTP/MCP job workers should
/// hold a kernel per job, not share one across tasks.
pub struct OcctKernel {
    next_id: u64,
    shapes: HashMap<u64, Shape>,
    labels: HashMap<u64, String>,
}

// SAFETY: Shapes are uniquely owned behind the HashMap; we never share &mut across threads.
unsafe impl Send for OcctKernel {}

impl Default for OcctKernel {
    fn default() -> Self {
        Self::new()
    }
}

impl OcctKernel {
    /// Create an empty kernel.
    pub fn new() -> Self {
        Self {
            next_id: 0,
            shapes: HashMap::new(),
            labels: HashMap::new(),
        }
    }

    fn alloc(&mut self, shape: Shape) -> ShapeId {
        self.next_id += 1;
        let id = self.next_id;
        self.shapes.insert(id, shape);
        ShapeId(id)
    }

    fn get(&self, id: ShapeId) -> KernelResult<&Shape> {
        self.shapes
            .get(&id.0)
            .ok_or_else(|| KernelError::unknown_shape(id))
    }

    pub(crate) fn get_pub(&self, id: ShapeId) -> KernelResult<&Shape> {
        self.get(id)
    }

    fn map_occt_err(op: &str, err: opencascade::Error) -> KernelError {
        KernelError::diagnostic(
            "CADRION-E-KERNEL",
            format!("{op}: {err}"),
            Some("check geometry validity / feature parameters".into()),
        )
    }

    /// Deep-copy via BRep transform (no STEP I/O).
    fn clone_shape(shape: &Shape) -> KernelResult<Shape> {
        shape
            .deep_copy()
            .map_err(|e| Self::map_occt_err("deep_copy", e))
    }

    /// Select edges by `EdgeRef` indices (stable OCCT explorer order).
    /// Empty `edges` → all edges; `refs` always lists what will be filleted/chamfered.
    fn select_edges(
        work: &Shape,
        edges: &[EdgeRef],
        shape: ShapeId,
    ) -> KernelResult<(Vec<opencascade::primitives::Edge>, Vec<String>)> {
        use opencascade::primitives::Edge;
        if edges.is_empty() {
            let all: Vec<Edge> = work.edges().collect();
            let refs: Vec<String> = (0..all.len()).map(|i| format!("#e{i}")).collect();
            return Ok((all, refs));
        }
        let wanted: std::collections::HashSet<u32> = edges.iter().map(|e| e.0).collect();
        let mut selected = Vec::new();
        let mut refs = Vec::new();
        for (i, edge) in work.edges().enumerate() {
            let idx = i as u32;
            if wanted.contains(&idx) {
                selected.push(edge);
                refs.push(format!("#e{idx}"));
            }
        }
        if selected.len() != wanted.len() {
            return Err(KernelError::diagnostic(
                "CADRION-E-UNKNOWN-EDGE",
                format!(
                    "requested {} edges, found {} on shape {shape} (shape has {} edges)",
                    wanted.len(),
                    selected.len(),
                    work.edges().count()
                ),
                Some("run inspect edges / use smaller indices; selectors are #e0..#eN".into()),
            )
            .with_shape(shape)
            .with_refs(wanted.iter().map(|i| format!("#e{i}"))));
        }
        Ok((selected, refs))
    }

    fn fillet_fail(
        shape: ShapeId,
        radius: f64,
        edge_count: u32,
        refs: &[String],
        err: opencascade::Error,
    ) -> KernelError {
        KernelError::diagnostic(
            "CADRION-E-FILLET-FAILED",
            format!(
                "fillet r={radius} failed on shape {shape} ({edge_count} edges available): {err}"
            ),
            Some(
                "reduce radius; fillet fewer edges via edges=[…]; mock kernel always Unsupported — use OCCT"
                    .into(),
            ),
        )
        .with_shape(shape)
        .with_refs(refs.iter().cloned())
    }

    fn chamfer_fail(
        shape: ShapeId,
        distance: f64,
        edge_count: u32,
        refs: &[String],
        err: opencascade::Error,
    ) -> KernelError {
        KernelError::diagnostic(
            "CADRION-E-CHAMFER-FAILED",
            format!(
                "chamfer d={distance} failed on shape {shape} ({edge_count} edges available): {err}"
            ),
            Some(
                "reduce distance; chamfer fewer edges via edges=[…]; mock kernel always Unsupported — use OCCT"
                    .into(),
            ),
        )
        .with_shape(shape)
        .with_refs(refs.iter().cloned())
    }

    fn bbox_from_mesh(shape: &Shape) -> BBox {
        let mesh = shape.mesh();
        if mesh.vertices.is_empty() {
            return BBox::from_min_max(Point3::ORIGIN, Point3::ORIGIN);
        }
        let mut min = Point3::new(mesh.vertices[0].x, mesh.vertices[0].y, mesh.vertices[0].z);
        let mut max = min;
        for v in mesh.vertices.iter().skip(1) {
            min.x = min.x.min(v.x);
            min.y = min.y.min(v.y);
            min.z = min.z.min(v.z);
            max.x = max.x.max(v.x);
            max.y = max.y.max(v.y);
            max.z = max.z.max(v.z);
        }
        BBox::from_min_max(min, max)
    }

    /// Volume estimate from tessellation (signed tetrahedra from origin).
    /// Good enough for golden tests within a few percent; exact GProp needs sys ffi.
    fn volume_from_mesh(shape: &Shape) -> f64 {
        let mesh = shape.mesh();
        let v = &mesh.vertices;
        let mut vol = 0.0;
        for tri in mesh.indices.chunks_exact(3) {
            let a = v[tri[0]];
            let b = v[tri[1]];
            let c = v[tri[2]];
            // scalar triple product / 6
            vol += a.dot(b.cross(c)) / 6.0;
        }
        vol.abs()
    }

    fn area_from_mesh(shape: &Shape) -> f64 {
        let mesh = shape.mesh();
        let v = &mesh.vertices;
        let mut area = 0.0;
        for tri in mesh.indices.chunks_exact(3) {
            let a = v[tri[0]];
            let b = v[tri[1]];
            let c = v[tri[2]];
            area += (b - a).cross(c - a).length() * 0.5;
        }
        area
    }

    fn centroid_from_mesh(shape: &Shape) -> Point3 {
        let mesh = shape.mesh();
        if mesh.vertices.is_empty() {
            return Point3::ORIGIN;
        }
        let mut sx = 0.0;
        let mut sy = 0.0;
        let mut sz = 0.0;
        for p in &mesh.vertices {
            sx += p.x;
            sy += p.y;
            sz += p.z;
        }
        let n = mesh.vertices.len() as f64;
        Point3::new(sx / n, sy / n, sz / n)
    }
}

impl GeomKernel for OcctKernel {
    fn backend_id(&self) -> &'static str {
        "occt"
    }

    fn backend_version(&self) -> &str {
        "opencascade-0.2"
    }

    fn parity_eligible(&self) -> bool {
        true
    }

    fn box_solid(
        &mut self,
        dx: f64,
        dy: f64,
        dz: f64,
        placement: Placement,
    ) -> KernelResult<ShapeId> {
        if dx <= 0.0 || dy <= 0.0 || dz <= 0.0 {
            return Err(KernelError::invalid_arg(format!(
                "box dims must be > 0, got {dx},{dy},{dz}"
            )));
        }
        // Centered at placement.origin; OCCT make_box is corner-based.
        let o = placement.origin;
        let p1 = dvec3(o.x - dx * 0.5, o.y - dy * 0.5, o.z - dz * 0.5);
        let p2 = dvec3(o.x + dx * 0.5, o.y + dy * 0.5, o.z + dz * 0.5);
        let shape = AdHocShape::make_box_point_point(p1, p2).into_shape();
        Ok(self.alloc(shape))
    }

    fn cylinder(
        &mut self,
        radius: f64,
        height: f64,
        placement: Placement,
    ) -> KernelResult<ShapeId> {
        if radius <= 0.0 || height <= 0.0 {
            return Err(KernelError::invalid_arg(format!(
                "cylinder radius/height must be > 0, got r={radius} h={height}"
            )));
        }
        let o = placement.origin;
        let shape = AdHocShape::make_cylinder(dvec3(o.x, o.y, o.z), radius, height).into_shape();
        Ok(self.alloc(shape))
    }

    fn boolean(&mut self, op: BooleanOp, a: ShapeId, b: ShapeId) -> KernelResult<ShapeId> {
        let sa = self.get(a)?;
        let sb = self.get(b)?;
        // Prefer AdHocShape boolean ops: Shape::subtract/union call SectionEdges()
        // which throws StdFail_NotDone on some OCCT builds. AdHoc only takes Shape().
        let result = match op {
            BooleanOp::Union => {
                let mut left = AdHocShape(Self::clone_shape(sa)?);
                left.union(sb);
                left.into_shape()
            }
            BooleanOp::Cut => {
                let mut left = AdHocShape(Self::clone_shape(sa)?);
                left.subtract(sb);
                left.into_shape()
            }
            BooleanOp::Intersect => {
                let mut left = AdHocShape(Self::clone_shape(sa)?);
                left.intersect(sb);
                left.into_shape()
            }
        };
        Ok(self.alloc(result))
    }

    fn fillet(&mut self, shape: ShapeId, edges: &[EdgeRef], radius: f64) -> KernelResult<ShapeId> {
        if radius <= 0.0 {
            return Err(KernelError::invalid_arg(format!(
                "fillet radius must be > 0, got {radius}"
            )));
        }
        let mut work = Self::clone_shape(self.get(shape)?)?;
        let edge_count = work.edges().count() as u32;
        let (selected, refs) = Self::select_edges(&work, edges, shape)?;
        let res = if edges.is_empty() {
            work.fillet(radius)
        } else {
            work.fillet_edges(radius, &selected)
        };
        match res {
            Ok(()) => Ok(self.alloc(work)),
            Err(e) => Err(Self::fillet_fail(shape, radius, edge_count, &refs, e)),
        }
    }

    fn chamfer(
        &mut self,
        shape: ShapeId,
        edges: &[EdgeRef],
        distance: f64,
    ) -> KernelResult<ShapeId> {
        if distance <= 0.0 {
            return Err(KernelError::invalid_arg(format!(
                "chamfer distance must be > 0, got {distance}"
            )));
        }
        let mut work = Self::clone_shape(self.get(shape)?)?;
        let edge_count = work.edges().count() as u32;
        let (selected, refs) = Self::select_edges(&work, edges, shape)?;
        let res = if edges.is_empty() {
            work.chamfer(distance)
        } else {
            work.chamfer_edges(distance, &selected)
        };
        match res {
            Ok(()) => Ok(self.alloc(work)),
            Err(e) => Err(Self::chamfer_fail(shape, distance, edge_count, &refs, e)),
        }
    }

    fn set_label(&mut self, shape: ShapeId, label: ShapeLabel) -> KernelResult<ShapeId> {
        let _ = self.get(shape)?;
        self.labels.insert(shape.0, label.0);
        Ok(shape)
    }

    fn facts(&self, shape: ShapeId) -> KernelResult<ShapeFacts> {
        let s = self.get(shape)?;
        let volume = Self::volume_from_mesh(s);
        let area = Self::area_from_mesh(s);
        let bbox = Self::bbox_from_mesh(s);
        let centroid = Self::centroid_from_mesh(s);
        let faces = s.faces().count() as u32;
        let edges = s.edges().count() as u32;
        Ok(ShapeFacts {
            bbox_mm: bbox,
            volume_mm3: volume,
            area_mm2: Some(area),
            centroid_mm: Some(centroid),
            solids: 1,
            faces,
            edges,
            vertices: None,
            mass_g: None,
        })
    }

    fn validity(&self, shape: ShapeId) -> KernelResult<ValidityReport> {
        let f = self.facts(shape)?;
        let mut notes = vec!["volume/area from tessellation (approx)".into()];
        if let Some(l) = self.labels.get(&shape.0) {
            notes.push(format!("label={l}"));
        }
        Ok(ValidityReport {
            closed: f.volume_mm3 > 0.0,
            positive_volume: f.volume_mm3 > 0.0,
            shells: 1,
            notes,
        })
    }

    fn edges(&self, shape: ShapeId) -> KernelResult<Vec<EdgeRef>> {
        let s = self.get(shape)?;
        let n = s.edges().count() as u32;
        Ok((0..n).map(EdgeRef).collect())
    }

    fn write_step(&self, shape: ShapeId, path: &Path, _opts: &StepWriteOpts) -> KernelResult<()> {
        let s = self.get(shape)?;
        s.write_step(path)
            .map_err(|e| Self::map_occt_err("write_step", e))
    }

    fn read_step(&mut self, path: &Path, _opts: &StepReadOpts) -> KernelResult<ShapeId> {
        let shape = Shape::read_step(path).map_err(|e| Self::map_occt_err("read_step", e))?;
        Ok(self.alloc(shape))
    }

    fn tessellate(&self, shape: ShapeId, _tol: TessTol) -> KernelResult<Mesh> {
        let s = self.get(shape)?;
        let m = s.mesh();
        let mut positions = Vec::with_capacity(m.vertices.len() * 3);
        for v in &m.vertices {
            positions.push(v.x as f32);
            positions.push(v.y as f32);
            positions.push(v.z as f32);
        }
        let mut normals = Vec::with_capacity(m.normals.len() * 3);
        for n in &m.normals {
            normals.push(n.x as f32);
            normals.push(n.y as f32);
            normals.push(n.z as f32);
        }
        let indices: Vec<u32> = m.indices.iter().map(|&i| i as u32).collect();
        Ok(Mesh {
            positions,
            normals: if normals.is_empty() {
                None
            } else {
                Some(normals)
            },
            indices,
        })
    }

    fn translate(&mut self, shape: ShapeId, dx: f64, dy: f64, dz: f64) -> KernelResult<ShapeId> {
        if ![dx, dy, dz].into_iter().all(|v| v.is_finite()) {
            return Err(KernelError::invalid_arg("translate offsets must be finite"));
        }
        let src = self.get(shape)?;
        let out = src
            .transformed_with(|trsf| {
                use opencascade_sys::ffi;
                let v = ffi::new_vec(dx, dy, dz);
                trsf.set_translation_vec(&v);
            })
            .map_err(|e| Self::map_occt_err("translate", e))?;
        Ok(self.alloc(out))
    }

    fn rotate_about_axis(&mut self, shape: ShapeId, axis: &str, deg: f64) -> KernelResult<ShapeId> {
        if !deg.is_finite() {
            return Err(KernelError::invalid_arg("deg must be finite"));
        }
        let ax = axis.to_ascii_lowercase();
        let dir = match ax.as_str() {
            "x" => (1.0, 0.0, 0.0),
            "y" => (0.0, 1.0, 0.0),
            "z" => (0.0, 0.0, 1.0),
            _ => {
                return Err(KernelError::invalid_arg(
                    "axis must be \"x\", \"y\", or \"z\"",
                ))
            }
        };
        let src = self.get(shape)?;
        let out = src
            .transformed_with(|trsf| {
                use opencascade_sys::ffi;
                let origin = ffi::new_point(0.0, 0.0, 0.0);
                let d = ffi::gp_Dir_ctor(dir.0, dir.1, dir.2);
                let axis1 = ffi::gp_Ax1_ctor(&origin, &d);
                trsf.SetRotation(&axis1, deg.to_radians());
            })
            .map_err(|e| Self::map_occt_err("rotate", e))?;
        Ok(self.alloc(out))
    }

    fn sphere(&mut self, radius: f64, placement: Placement) -> KernelResult<ShapeId> {
        if radius <= 0.0 {
            return Err(KernelError::invalid_arg(format!(
                "sphere radius must be > 0, got {radius}"
            )));
        }
        let shape = Shape::sphere(radius).map_err(|e| Self::map_occt_err("sphere", e))?;
        let sid = self.alloc(shape);
        let o = placement.origin;
        if o.x.abs() > 1e-15 || o.y.abs() > 1e-15 || o.z.abs() > 1e-15 {
            return self.translate(sid, o.x, o.y, o.z);
        }
        Ok(sid)
    }

    fn cone(&mut self, radius: f64, height: f64, placement: Placement) -> KernelResult<ShapeId> {
        // H3-1 honesty: opencascade-sys 0.2 has no MakeCone. Do **not** silently
        // substitute a cylinder (volume would be 3× a true cone). Fail closed.
        let _ = (radius, height, placement);
        Err(KernelError::unsupported(
            self.backend_id(),
            "cone (no MakeCone in opencascade-sys 0.2; refuse cylinder stand-in — use mock for analytic cone or loft later)",
        ))
    }

    fn mirror_plane(&mut self, shape: ShapeId, plane: &str) -> KernelResult<ShapeId> {
        // Plane mirror = central inversion (scale -1) then 180° about the plane normal's
        // complementary axis. sys only exposes SetMirror(gp_Ax1) (axis symmetry), not Ax2.
        let pl = plane.to_ascii_lowercase();
        let rot_axis = match pl.as_str() {
            "xy" => (0.0, 0.0, 1.0),        // after scale-1: need 180 about Z → (x,y,-z)
            "yz" => (1.0, 0.0, 0.0),        // → (-x,y,z)
            "zx" | "xz" => (0.0, 1.0, 0.0), // → (x,-y,z)
            _ => {
                return Err(KernelError::invalid_arg(
                    "plane must be \"xy\", \"yz\", or \"zx\"",
                ))
            }
        };
        let src = self.get(shape)?;
        // Two sequential BRep transforms (no STEP).
        let inverted = src
            .transformed_with(|trsf| {
                use opencascade_sys::ffi;
                let origin = ffi::new_point(0.0, 0.0, 0.0);
                trsf.SetScale(&origin, -1.0);
            })
            .map_err(|e| Self::map_occt_err("mirror/scale", e))?;
        let out = inverted
            .transformed_with(|trsf| {
                use opencascade_sys::ffi;
                let origin = ffi::new_point(0.0, 0.0, 0.0);
                let d = ffi::gp_Dir_ctor(rot_axis.0, rot_axis.1, rot_axis.2);
                let axis1 = ffi::gp_Ax1_ctor(&origin, &d);
                trsf.SetRotation(&axis1, std::f64::consts::PI);
            })
            .map_err(|e| Self::map_occt_err("mirror/rotate", e))?;
        Ok(self.alloc(out))
    }
}

/// Convenience: write STEP next to a logical name under `dir`.
pub fn step_path(dir: impl Into<PathBuf>, basename: &str) -> PathBuf {
    dir.into().join(format!("{basename}.step"))
}
