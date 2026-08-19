//! Validators for robot descriptions.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::model::{JointType, RobotSpec};
use crate::srdf::SrdfSpec;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    pub ok: bool,
    pub kind: String,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl ValidationReport {
    fn finish(mut self) -> Self {
        self.ok = self.errors.is_empty();
        self
    }
}

/// Structural validation of RobotSpec before XML emit.
pub fn validate_robot(robot: &RobotSpec) -> ValidationReport {
    let mut r = ValidationReport {
        ok: false,
        kind: "robot_spec".into(),
        errors: vec![],
        warnings: vec![],
    };

    if robot.name.is_empty() {
        r.errors.push("robot name empty".into());
    }
    if robot.links.is_empty() {
        r.errors.push("no links".into());
    }

    let mut link_names = HashSet::new();
    for l in &robot.links {
        if !link_names.insert(l.name.clone()) {
            r.errors.push(format!("duplicate link '{}'", l.name));
        }
        if !l.inertial.is_positive_definiteish() {
            r.errors
                .push(format!("link '{}': non-positive mass/inertia", l.name));
        } else if !l.inertial.triangle_ok() {
            r.warnings.push(format!(
                "link '{}': inertia triangle inequality soft-fail",
                l.name
            ));
        }
    }

    let mut joint_names = HashSet::new();
    let mut child_of: HashMap<String, String> = HashMap::new();
    for j in &robot.joints {
        if !joint_names.insert(j.name.clone()) {
            r.errors.push(format!("duplicate joint '{}'", j.name));
        }
        if !link_names.contains(&j.parent) {
            r.errors
                .push(format!("joint '{}': unknown parent '{}'", j.name, j.parent));
        }
        if !link_names.contains(&j.child) {
            r.errors
                .push(format!("joint '{}': unknown child '{}'", j.name, j.child));
        }
        if j.parent == j.child {
            r.errors.push(format!("joint '{}': parent==child", j.name));
        }
        if child_of.insert(j.child.clone(), j.name.clone()).is_some() {
            r.errors.push(format!(
                "link '{}' has multiple parents (not a tree)",
                j.child
            ));
        }
        let axis_n = (j.axis[0] * j.axis[0] + j.axis[1] * j.axis[1] + j.axis[2] * j.axis[2]).sqrt();
        if matches!(
            j.joint_type,
            JointType::Revolute | JointType::Continuous | JointType::Prismatic
        ) && axis_n < 1e-9
        {
            r.errors.push(format!("joint '{}': zero axis", j.name));
        }
        if j.joint_type == JointType::Revolute && (j.lower.is_none() || j.upper.is_none()) {
            r.errors.push(format!(
                "joint '{}': revolute requires lower and upper limits",
                j.name
            ));
        }
        if j.joint_type == JointType::Prismatic && (j.lower.is_none() || j.upper.is_none()) {
            r.errors.push(format!(
                "joint '{}': prismatic requires lower and upper limits",
                j.name
            ));
        }
        if let (Some(lo), Some(hi)) = (j.lower, j.upper) {
            if lo > hi {
                r.errors
                    .push(format!("joint '{}': lower ({lo}) > upper ({hi})", j.name));
            }
        }
        if let Some(e) = j.effort {
            if e < 0.0 {
                r.errors
                    .push(format!("joint '{}': negative effort", j.name));
            }
        }
        if let Some(v) = j.velocity {
            if v < 0.0 {
                r.errors
                    .push(format!("joint '{}': negative velocity", j.name));
            }
        }
    }

    // roots: links that are never a child
    let children: HashSet<_> = robot.joints.iter().map(|j| j.child.as_str()).collect();
    let roots: Vec<_> = robot
        .links
        .iter()
        .filter(|l| !children.contains(l.name.as_str()))
        .map(|l| l.name.as_str())
        .collect();
    if roots.len() != 1 {
        r.errors.push(format!(
            "expected exactly one root link, found {}: {roots:?}",
            roots.len()
        ));
    }

    // cycle check via parent walk
    for l in &robot.links {
        let mut seen = HashSet::new();
        let mut cur = l.name.as_str();
        while let Some(jname) = child_of.get(cur) {
            if !seen.insert(cur.to_string()) {
                r.errors.push(format!("cycle involving link '{cur}'"));
                break;
            }
            if let Some(j) = robot.joints.iter().find(|j| j.name == *jname) {
                cur = j.parent.as_str();
            } else {
                break;
            }
        }
    }

    r.finish()
}

/// Lightweight URDF XML checks (tags present).
pub fn validate_urdf_xml(xml: &str) -> ValidationReport {
    let mut r = ValidationReport {
        ok: false,
        kind: "urdf_xml".into(),
        errors: vec![],
        warnings: vec![],
    };
    if !xml.contains("<robot") {
        r.errors.push("missing <robot>".into());
    }
    if !xml.contains("<link") {
        r.errors.push("missing <link>".into());
    }
    if xml.matches("<link").count() != xml.matches("</link>").count() {
        r.errors.push("unbalanced <link> tags".into());
    }
    if xml.matches("<joint").count() != xml.matches("</joint>").count() {
        r.errors.push("unbalanced <joint> tags".into());
    }
    r.finish()
}

pub fn validate_sdf_xml(xml: &str) -> ValidationReport {
    let mut r = ValidationReport {
        ok: false,
        kind: "sdf_xml".into(),
        errors: vec![],
        warnings: vec![],
    };
    if !xml.contains("<sdf") {
        r.errors.push("missing <sdf>".into());
    }
    if !xml.contains("<model") {
        r.errors.push("missing <model>".into());
    }
    if !xml.contains("<link") {
        r.errors.push("missing <link>".into());
    }
    r.finish()
}

/// SRDF joint/link names must exist in robot.
pub fn validate_srdf_against_urdf(srdf: &SrdfSpec, robot: &RobotSpec) -> ValidationReport {
    let mut r = ValidationReport {
        ok: false,
        kind: "srdf_vs_urdf".into(),
        errors: vec![],
        warnings: vec![],
    };
    if srdf.name != robot.name {
        r.warnings.push(format!(
            "name mismatch srdf='{}' urdf='{}'",
            srdf.name, robot.name
        ));
    }
    let joints: HashSet<_> = robot.joints.iter().map(|j| j.name.as_str()).collect();
    let links: HashSet<_> = robot.links.iter().map(|l| l.name.as_str()).collect();
    for g in &srdf.groups {
        for j in &g.joints {
            if !joints.contains(j.as_str()) {
                r.errors
                    .push(format!("group '{}': unknown joint '{}'", g.name, j));
            }
        }
    }
    for ee in &srdf.end_effectors {
        if !links.contains(ee.parent_link.as_str()) {
            r.errors.push(format!(
                "end_effector '{}': unknown parent_link '{}'",
                ee.name, ee.parent_link
            ));
        }
    }
    r.finish()
}

/// Full pipeline: validate spec → URDF → optional urdf-rs.
pub fn emit_and_validate(robot: &RobotSpec) -> (String, ValidationReport) {
    let mut report = validate_robot(robot);
    let xml = crate::write_urdf(robot);
    let xml_rep = validate_urdf_xml(&xml);
    report.errors.extend(xml_rep.errors);
    report.warnings.extend(xml_rep.warnings);
    // parse with urdf-rs when dep present
    match parse_urdf_rs(&xml) {
        Ok(()) => {}
        Err(e) => report.errors.push(e),
    }
    (xml, report.finish())
}

/// Parse URDF XML with urdf-rs (ROS-ecosystem parser).
pub fn parse_urdf_xml(xml: &str) -> Result<(), String> {
    urdf_rs::read_from_string(xml)
        .map(|_| ())
        .map_err(|e| format!("urdf-rs: {e}"))
}

fn parse_urdf_rs(xml: &str) -> Result<(), String> {
    parse_urdf_xml(xml)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inertial::box_inertial;
    use crate::model::*;
    use crate::srdf::srdf_from_robot;

    fn simple_arm() -> RobotSpec {
        let density = 7800.0; // steel-ish kg/m3
                              // links in meters
        let base = Link {
            name: "base_link".into(),
            inertial: box_inertial([0.1, 0.1, 0.05], density),
            visual: vec![Visual {
                origin: Origin::default(),
                geometry: Geometry::Box {
                    size: [0.1, 0.1, 0.05],
                },
                material: Some(Material {
                    name: "grey".into(),
                    rgba: Some([0.5, 0.5, 0.5, 1.0]),
                }),
            }],
            collision: vec![Collision {
                origin: Origin::default(),
                geometry: Geometry::Box {
                    size: [0.1, 0.1, 0.05],
                },
            }],
        };
        let link1 = Link {
            name: "link1".into(),
            inertial: box_inertial([0.04, 0.04, 0.2], density),
            visual: vec![Visual {
                origin: Origin {
                    xyz: [0.0, 0.0, 0.1],
                    rpy: [0.0, 0.0, 0.0],
                },
                geometry: Geometry::Box {
                    size: [0.04, 0.04, 0.2],
                },
                material: None,
            }],
            collision: vec![],
        };
        let link2 = Link {
            name: "link2".into(),
            inertial: box_inertial([0.03, 0.03, 0.15], density),
            visual: vec![],
            collision: vec![],
        };
        RobotSpec {
            name: "simple_arm".into(),
            links: vec![base, link1, link2],
            joints: vec![
                Joint {
                    name: "joint1".into(),
                    joint_type: JointType::Revolute,
                    parent: "base_link".into(),
                    child: "link1".into(),
                    origin: Origin {
                        xyz: [0.0, 0.0, 0.025],
                        rpy: [0.0, 0.0, 0.0],
                    },
                    axis: [0.0, 0.0, 1.0],
                    lower: Some(-3.14),
                    upper: Some(3.14),
                    effort: Some(10.0),
                    velocity: Some(1.0),
                },
                Joint {
                    name: "joint2".into(),
                    joint_type: JointType::Revolute,
                    parent: "link1".into(),
                    child: "link2".into(),
                    origin: Origin {
                        xyz: [0.0, 0.0, 0.2],
                        rpy: [0.0, 0.0, 0.0],
                    },
                    axis: [0.0, 1.0, 0.0],
                    lower: Some(-1.57),
                    upper: Some(1.57),
                    effort: Some(5.0),
                    velocity: Some(1.0),
                },
            ],
        }
    }

    #[test]
    fn arm_validates_and_parses() {
        let robot = simple_arm();
        let (xml, rep) = emit_and_validate(&robot);
        assert!(rep.ok, "{rep:?}");
        assert!(xml.contains("simple_arm"));
        let srdf = srdf_from_robot(&robot, "arm");
        let srep = validate_srdf_against_urdf(&srdf, &robot);
        assert!(srep.ok, "{srep:?}");
        let sdf = crate::write_sdf(&robot);
        assert!(validate_sdf_xml(&sdf).ok);
    }
}
