//! Geometry primitives used across the kernel boundary.
//!
//! Units are **millimeters** and **degrees** unless a field name says otherwise.
//! Hand-rolled (no glam) so the kernel crate stays dependency-light and deterministic.

use serde::{Deserialize, Serialize};
use std::fmt;

/// 3D point in model space (mm).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Point3 {
    pub const ORIGIN: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };

    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }
}

impl fmt::Display for Point3 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({:.6}, {:.6}, {:.6})", self.x, self.y, self.z)
    }
}

/// 3D vector (mm or unitless direction, depending on context).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vec3 {
    pub const X: Self = Self {
        x: 1.0,
        y: 0.0,
        z: 0.0,
    };
    pub const Y: Self = Self {
        x: 0.0,
        y: 1.0,
        z: 0.0,
    };
    pub const Z: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 1.0,
    };
    pub const ZERO: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };

    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    pub fn length(self) -> f64 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }
}

/// Axis-aligned bounding box (mm).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BBox {
    pub min: Point3,
    pub max: Point3,
}

impl BBox {
    pub fn from_min_max(min: Point3, max: Point3) -> Self {
        Self { min, max }
    }

    /// `[dx, dy, dz]` extents in mm.
    pub fn extents_mm(self) -> [f64; 3] {
        [
            (self.max.x - self.min.x).abs(),
            (self.max.y - self.min.y).abs(),
            (self.max.z - self.min.z).abs(),
        ]
    }

    pub fn center(self) -> Point3 {
        Point3::new(
            (self.min.x + self.max.x) * 0.5,
            (self.min.y + self.max.y) * 0.5,
            (self.min.z + self.max.z) * 0.5,
        )
    }

    /// Volume of the AABB (not the solid) — diagnostic helper only.
    pub fn aabb_volume_mm3(self) -> f64 {
        let e = self.extents_mm();
        e[0] * e[1] * e[2]
    }
}

/// Rigid placement: translation only in v0 (rotation arrives with assemblies).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Placement {
    pub origin: Point3,
}

impl Placement {
    pub const IDENTITY: Self = Self {
        origin: Point3::ORIGIN,
    };

    pub fn at(origin: Point3) -> Self {
        Self { origin }
    }
}

impl Default for Placement {
    fn default() -> Self {
        Self::IDENTITY
    }
}

/// Boolean flavor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BooleanOp {
    Union,
    Cut,
    Intersect,
}

/// Tessellation tolerances.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TessTol {
    /// Linear deflection (mm).
    pub linear_mm: f64,
    /// Angular deflection (radians).
    pub angular_rad: f64,
}

impl Default for TessTol {
    fn default() -> Self {
        Self {
            linear_mm: 0.1,
            angular_rad: 0.5,
        }
    }
}

/// Optional density for mass properties (g/cm³). Water ≈ 1.0.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Density {
    pub g_per_cm3: f64,
}

impl Density {
    pub const fn g_per_cm3(v: f64) -> Self {
        Self { g_per_cm3: v }
    }
}
