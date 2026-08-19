//! Robot intermediate model (JSON-friendly).

use serde::{Deserialize, Serialize};

use crate::inertial::Inertial;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Origin {
    /// xyz meters (URDF units)
    pub xyz: [f64; 3],
    /// rpy radians
    pub rpy: [f64; 3],
}

impl Default for Origin {
    fn default() -> Self {
        Self {
            xyz: [0.0, 0.0, 0.0],
            rpy: [0.0, 0.0, 0.0],
        }
    }
}

/// Geometry for visual/collision. Externally tagged JSON:
/// `{"box":{"size":[x,y,z]}}` / `{"cylinder":{"radius":r,"length":l}}` / …
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Geometry {
    Box { size: [f64; 3] },
    Cylinder { radius: f64, length: f64 },
    Sphere { radius: f64 },
    Mesh { filename: String, scale: [f64; 3] },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Material {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rgba: Option<[f64; 4]>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Visual {
    #[serde(default)]
    pub origin: Origin,
    pub geometry: Geometry,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub material: Option<Material>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Collision {
    #[serde(default)]
    pub origin: Origin,
    pub geometry: Geometry,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Link {
    pub name: String,
    pub inertial: Inertial,
    #[serde(default)]
    pub visual: Vec<Visual>,
    #[serde(default)]
    pub collision: Vec<Collision>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JointType {
    Fixed,
    Revolute,
    Continuous,
    Prismatic,
    Floating,
    Planar,
}

impl JointType {
    pub fn as_urdf(self) -> &'static str {
        match self {
            JointType::Fixed => "fixed",
            JointType::Revolute => "revolute",
            JointType::Continuous => "continuous",
            JointType::Prismatic => "prismatic",
            JointType::Floating => "floating",
            JointType::Planar => "planar",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Joint {
    pub name: String,
    #[serde(rename = "type")]
    pub joint_type: JointType,
    pub parent: String,
    pub child: String,
    #[serde(default)]
    pub origin: Origin,
    /// Axis for revolute/prismatic
    #[serde(default = "default_axis")]
    pub axis: [f64; 3],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lower: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upper: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub velocity: Option<f64>,
}

fn default_axis() -> [f64; 3] {
    [0.0, 0.0, 1.0]
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RobotSpec {
    pub name: String,
    pub links: Vec<Link>,
    pub joints: Vec<Joint>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn geometry_box_json() {
        let g: Geometry = serde_json::from_str(r#"{"box":{"size":[0.1,0.1,0.05]}}"#).unwrap();
        match g {
            Geometry::Box { size } => assert!((size[0] - 0.1).abs() < 1e-12),
            _ => panic!("expected box"),
        }
    }

    #[test]
    fn example_arm_json_loads() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/robots/simple_arm.robot.json"
        );
        let text = std::fs::read_to_string(path).expect("example json");
        let robot: RobotSpec = serde_json::from_str(&text).expect("parse robot");
        assert_eq!(robot.name, "simple_arm");
        assert_eq!(robot.links.len(), 3);
        assert_eq!(robot.joints.len(), 2);
    }
}
