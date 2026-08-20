//! Triangle mesh + camera views.

use cadrion_kernel::{Mesh, Point3};
use serde::{Deserialize, Serialize};

/// Named orthographic/isometric views.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ViewName {
    Iso,
    Front,
    Top,
    Right,
    Back,
    Left,
    Bottom,
}

impl ViewName {
    pub fn parse_list(s: &str) -> Vec<ViewName> {
        s.split(',')
            .filter_map(|p| match p.trim().to_ascii_lowercase().as_str() {
                "iso" => Some(ViewName::Iso),
                "front" => Some(ViewName::Front),
                "top" => Some(ViewName::Top),
                "right" => Some(ViewName::Right),
                "back" => Some(ViewName::Back),
                "left" => Some(ViewName::Left),
                "bottom" => Some(ViewName::Bottom),
                _ => None,
            })
            .collect()
    }

    pub fn as_str(self) -> &'static str {
        match self {
            ViewName::Iso => "iso",
            ViewName::Front => "front",
            ViewName::Top => "top",
            ViewName::Right => "right",
            ViewName::Back => "back",
            ViewName::Left => "left",
            ViewName::Bottom => "bottom",
        }
    }
}

/// Camera: look-at with orthographic scale.
#[derive(Debug, Clone, Copy)]
pub struct Camera {
    pub eye: Point3,
    pub target: Point3,
    pub up: Point3,
    /// Half-extent of the view frustum in world units (orthographic).
    pub half_extent: f64,
}

/// Build a camera for `view` framing `center` with characteristic size `radius`.
pub fn camera_for_view(view: ViewName, center: Point3, radius: f64) -> Camera {
    let d = (radius * 2.5).max(1.0);
    let (eye_off, up) = match view {
        ViewName::Iso => (Point3::new(d, d * 0.75, d), Point3::new(0.0, 0.0, 1.0)),
        ViewName::Front => (Point3::new(0.0, -d, 0.0), Point3::new(0.0, 0.0, 1.0)),
        ViewName::Back => (Point3::new(0.0, d, 0.0), Point3::new(0.0, 0.0, 1.0)),
        ViewName::Top => (Point3::new(0.0, 0.0, d), Point3::new(0.0, 1.0, 0.0)),
        ViewName::Bottom => (Point3::new(0.0, 0.0, -d), Point3::new(0.0, 1.0, 0.0)),
        ViewName::Right => (Point3::new(d, 0.0, 0.0), Point3::new(0.0, 0.0, 1.0)),
        ViewName::Left => (Point3::new(-d, 0.0, 0.0), Point3::new(0.0, 0.0, 1.0)),
    };
    Camera {
        eye: Point3::new(
            center.x + eye_off.x,
            center.y + eye_off.y,
            center.z + eye_off.z,
        ),
        target: center,
        up,
        half_extent: radius * 1.25 + 1e-6,
    }
}

/// Orbit camera at angle `theta` radians around +Z through center.
pub fn camera_orbit(center: Point3, radius: f64, theta: f64, elev: f64) -> Camera {
    let d = (radius * 2.5).max(1.0);
    let eye = Point3::new(
        center.x + d * elev.cos() * theta.cos(),
        center.y + d * elev.cos() * theta.sin(),
        center.z + d * elev.sin(),
    );
    Camera {
        eye,
        target: center,
        up: Point3::new(0.0, 0.0, 1.0),
        half_extent: radius * 1.25 + 1e-6,
    }
}

/// Axis-aligned bounds of a mesh.
pub fn mesh_bounds(mesh: &Mesh) -> (Point3, Point3) {
    let mut min = Point3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY);
    let mut max = Point3::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
    for c in mesh.positions.as_chunks::<3>().0 {
        let x = c[0] as f64;
        let y = c[1] as f64;
        let z = c[2] as f64;
        min.x = min.x.min(x);
        min.y = min.y.min(y);
        min.z = min.z.min(z);
        max.x = max.x.max(x);
        max.y = max.y.max(y);
        max.z = max.z.max(z);
    }
    if !min.x.is_finite() {
        min = Point3::ORIGIN;
        max = Point3::new(1.0, 1.0, 1.0);
    }
    (min, max)
}

pub fn bounds_center_radius(min: Point3, max: Point3) -> (Point3, f64) {
    let c = Point3::new(
        (min.x + max.x) * 0.5,
        (min.y + max.y) * 0.5,
        (min.z + max.z) * 0.5,
    );
    let dx = max.x - min.x;
    let dy = max.y - min.y;
    let dz = max.z - min.z;
    let r = (dx * dx + dy * dy + dz * dz).sqrt() * 0.5;
    (c, r.max(1e-3))
}
