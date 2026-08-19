//! SRDF (MoveIt planning) generation.

use serde::{Deserialize, Serialize};

use crate::model::RobotSpec;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SrdfSpec {
    pub name: String,
    pub groups: Vec<SrdfGroup>,
    #[serde(default)]
    pub end_effectors: Vec<SrdfEndEffector>,
    #[serde(default)]
    pub disable_collisions: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SrdfGroup {
    pub name: String,
    pub joints: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SrdfEndEffector {
    pub name: String,
    pub parent_link: String,
    pub group: String,
}

pub fn write_srdf(srdf: &SrdfSpec) -> String {
    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\"?>\n");
    out.push_str(&format!("<robot name=\"{}\">\n", escape(&srdf.name)));
    for g in &srdf.groups {
        out.push_str(&format!("  <group name=\"{}\">\n", escape(&g.name)));
        for j in &g.joints {
            out.push_str(&format!("    <joint name=\"{}\"/>\n", escape(j)));
        }
        out.push_str("  </group>\n");
    }
    for ee in &srdf.end_effectors {
        out.push_str(&format!(
            "  <end_effector name=\"{}\" parent_link=\"{}\" group=\"{}\"/>\n",
            escape(&ee.name),
            escape(&ee.parent_link),
            escape(&ee.group)
        ));
    }
    for (a, b) in &srdf.disable_collisions {
        out.push_str(&format!(
            "  <disable_collisions link1=\"{}\" link2=\"{}\" reason=\"Adjacent\"/>\n",
            escape(a),
            escape(b)
        ));
    }
    out.push_str("</robot>\n");
    out
}

/// Build a default arm group from all non-fixed joints.
pub fn srdf_from_robot(robot: &RobotSpec, group_name: &str) -> SrdfSpec {
    let joints: Vec<_> = robot
        .joints
        .iter()
        .filter(|j| j.joint_type != crate::model::JointType::Fixed)
        .map(|j| j.name.clone())
        .collect();
    let ee_link = robot
        .links
        .last()
        .map(|l| l.name.clone())
        .unwrap_or_else(|| "tool0".into());
    SrdfSpec {
        name: robot.name.clone(),
        groups: vec![crate::srdf::SrdfGroup {
            name: group_name.into(),
            joints: joints.clone(),
        }],
        end_effectors: if joints.is_empty() {
            vec![]
        } else {
            vec![SrdfEndEffector {
                name: "ee".into(),
                parent_link: ee_link,
                group: group_name.into(),
            }]
        },
        disable_collisions: vec![],
    }
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
