//! Inertial tensors from simple solids (SI units: kg, m).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Inertial {
    #[serde(default)]
    pub origin_xyz: [f64; 3],
    #[serde(default)]
    pub origin_rpy: [f64; 3],
    pub mass: f64,
    pub ixx: f64,
    pub ixy: f64,
    pub ixz: f64,
    pub iyy: f64,
    pub iyz: f64,
    pub izz: f64,
}

impl Inertial {
    pub fn is_positive_definiteish(&self) -> bool {
        self.mass > 0.0 && self.ixx > 0.0 && self.iyy > 0.0 && self.izz > 0.0
    }

    /// Triangle inequality on principal moments (loose).
    pub fn triangle_ok(&self) -> bool {
        self.ixx + self.iyy >= self.izz - 1e-9
            && self.iyy + self.izz >= self.ixx - 1e-9
            && self.izz + self.ixx >= self.iyy - 1e-9
    }
}

/// Solid box size (m) + density (kg/m³) → inertial at COM.
pub fn box_inertial(size: [f64; 3], density: f64) -> Inertial {
    let (sx, sy, sz) = (size[0], size[1], size[2]);
    let mass = density * sx * sy * sz;
    let ixx = mass * (sy * sy + sz * sz) / 12.0;
    let iyy = mass * (sx * sx + sz * sz) / 12.0;
    let izz = mass * (sx * sx + sy * sy) / 12.0;
    Inertial {
        origin_xyz: [0.0, 0.0, 0.0],
        origin_rpy: [0.0, 0.0, 0.0],
        mass,
        ixx,
        ixy: 0.0,
        ixz: 0.0,
        iyy,
        iyz: 0.0,
        izz,
    }
}

/// Cylinder along Z, radius + length (m), density kg/m³.
pub fn cylinder_inertial(radius: f64, length: f64, density: f64) -> Inertial {
    let mass = density * std::f64::consts::PI * radius * radius * length;
    let izz = 0.5 * mass * radius * radius;
    let ixx = mass * (3.0 * radius * radius + length * length) / 12.0;
    Inertial {
        origin_xyz: [0.0, 0.0, 0.0],
        origin_rpy: [0.0, 0.0, 0.0],
        mass,
        ixx,
        ixy: 0.0,
        ixz: 0.0,
        iyy: ixx,
        iyz: 0.0,
        izz,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn box_mass_and_pd() {
        // 0.1 m cube, density 1000 → 1 kg
        let i = box_inertial([0.1, 0.1, 0.1], 1000.0);
        assert!((i.mass - 1.0).abs() < 1e-9);
        assert!(i.is_positive_definiteish());
        assert!(i.triangle_ok());
    }
}
