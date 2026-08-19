//! Viewer payload for joint jog (alpha 2D FK).

use serde::{Deserialize, Serialize};

use crate::model::{JointType, RobotSpec};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JogJoint {
    pub name: String,
    pub joint_type: String,
    pub parent: String,
    pub child: String,
    pub origin_xyz: [f64; 3],
    pub origin_rpy: [f64; 3],
    pub axis: [f64; 3],
    pub lower: f64,
    pub upper: f64,
    pub movable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JogLink {
    pub name: String,
    /// Visual box size if present (meters), else default stick.
    pub size: Option<[f64; 3]>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JogPayload {
    pub name: String,
    pub root: String,
    pub links: Vec<JogLink>,
    pub joints: Vec<JogJoint>,
}

/// Build a compact jog payload for the loopback viewer.
pub fn jog_payload(robot: &RobotSpec) -> JogPayload {
    let children: std::collections::HashSet<_> =
        robot.joints.iter().map(|j| j.child.as_str()).collect();
    let root = robot
        .links
        .iter()
        .map(|l| l.name.as_str())
        .find(|n| !children.contains(n))
        .unwrap_or("base_link")
        .to_string();

    let links = robot
        .links
        .iter()
        .map(|l| {
            let size = l.visual.first().and_then(|v| match &v.geometry {
                crate::model::Geometry::Box { size } => Some(*size),
                crate::model::Geometry::Cylinder { radius, length } => {
                    Some([*radius * 2.0, *radius * 2.0, *length])
                }
                crate::model::Geometry::Sphere { radius } => {
                    Some([*radius * 2.0, *radius * 2.0, *radius * 2.0])
                }
                crate::model::Geometry::Mesh { .. } => None,
            });
            JogLink {
                name: l.name.clone(),
                size,
            }
        })
        .collect();

    let joints = robot
        .joints
        .iter()
        .map(|j| {
            let movable = matches!(
                j.joint_type,
                JointType::Revolute | JointType::Continuous | JointType::Prismatic
            );
            let (lower, upper) = match j.joint_type {
                JointType::Continuous => (-std::f64::consts::PI, std::f64::consts::PI),
                JointType::Prismatic => (j.lower.unwrap_or(0.0), j.upper.unwrap_or(0.2)),
                _ => (
                    j.lower.unwrap_or(-std::f64::consts::PI),
                    j.upper.unwrap_or(std::f64::consts::PI),
                ),
            };
            JogJoint {
                name: j.name.clone(),
                joint_type: j.joint_type.as_urdf().into(),
                parent: j.parent.clone(),
                child: j.child.clone(),
                origin_xyz: j.origin.xyz,
                origin_rpy: j.origin.rpy,
                axis: j.axis,
                lower,
                upper,
                movable,
            }
        })
        .collect();

    JogPayload {
        name: robot.name.clone(),
        root,
        links,
        joints,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_arm_has_two_movable() {
        let text = include_str!("../../../examples/robots/simple_arm.robot.json");
        let robot: RobotSpec = serde_json::from_str(text).unwrap();
        let j = jog_payload(&robot);
        assert_eq!(j.name, "simple_arm");
        assert_eq!(j.joints.iter().filter(|j| j.movable).count(), 2);
        assert_eq!(j.root, "base_link");
    }
}
