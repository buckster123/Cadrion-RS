//! H3-4: assembly → kinematics packet / robot IR (OQ-4 partial).
//!
//! **Not AP242.** Labels + placements + joint envelope only. Units: assembly mm/deg →
//! robot IR meters/radians for URDF consumers.

use serde::{Deserialize, Serialize};

use crate::assembly::{validate_assembly, AssemblySpec, JointSpec};

/// Sidecar schema for joint/placement facts next to STEP or assembly JSON.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssemblyKinematics {
    pub ok: bool,
    pub schema: String,
    pub version: u32,
    pub name: String,
    /// Honesty fence.
    pub notes: Vec<String>,
    pub links: Vec<KinematicLink>,
    pub joints: Vec<KinematicJoint>,
    /// mm placements preserved for CAD consumers.
    pub placements_mm: Vec<PlacementRecord>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlacementRecord {
    pub component: String,
    pub origin_mm: [f64; 3],
    pub rpy_deg: [f64; 3],
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KinematicLink {
    pub name: String,
    /// Component source path / lock key (traceability).
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KinematicJoint {
    pub name: String,
    /// fixed | revolute | prismatic
    pub kind: String,
    pub parent: String,
    pub child: String,
    /// Joint origin in parent frame, **meters**.
    pub origin_xyz_m: [f64; 3],
    /// rpy **radians**
    pub origin_rpy_rad: [f64; 3],
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

/// Build kinematics packet. Fails closed if `validate_assembly` fails.
pub fn assembly_kinematics(spec: &AssemblySpec) -> Result<AssemblyKinematics, Vec<String>> {
    let report = validate_assembly(spec);
    if !report.ok {
        return Err(report.errors);
    }

    let placements_mm: Vec<PlacementRecord> = spec
        .components
        .iter()
        .map(|c| PlacementRecord {
            component: c.name.clone(),
            origin_mm: c.placement.origin_mm,
            rpy_deg: c.placement.rpy_deg,
            source: c.source.clone(),
        })
        .collect();

    let links: Vec<KinematicLink> = spec
        .components
        .iter()
        .map(|c| KinematicLink {
            name: c.name.clone(),
            source: c.source.clone(),
        })
        .collect();

    // Map joints: a=parent, b=child. Origin: joint.origin_mm if non-zero else child placement.
    let joints: Vec<KinematicJoint> = spec.joints.iter().map(|j| map_joint(spec, j)).collect();

    Ok(AssemblyKinematics {
        ok: true,
        schema: "cadrion.assembly_kinematics".into(),
        version: 1,
        name: spec.name.clone(),
        notes: vec![
            "H3-4 OQ-4 partial: labels+placements+joint envelope — not AP242 STEP kinematics"
                .into(),
            "units: origin_xyz_m meters, origin_rpy_rad radians, revolute limits radians, prismatic mm"
                .into(),
            "inertials/meshes not in this packet — use emit-robot for placeholder URDF path".into(),
        ],
        links,
        joints,
        placements_mm,
    })
}

fn map_joint(spec: &AssemblySpec, j: &JointSpec) -> KinematicJoint {
    let child = spec.components.iter().find(|c| c.name == j.b);
    let origin_mm = if j.origin_mm != [0.0, 0.0, 0.0] {
        j.origin_mm
    } else if let Some(c) = child {
        c.placement.origin_mm
    } else {
        [0.0, 0.0, 0.0]
    };
    let rpy_deg = child
        .map(|c| c.placement.rpy_deg)
        .unwrap_or([0.0, 0.0, 0.0]);
    KinematicJoint {
        name: j.name.clone(),
        kind: j.kind.to_ascii_lowercase(),
        parent: j.a.clone(),
        child: j.b.clone(),
        origin_xyz_m: [
            origin_mm[0] / 1000.0,
            origin_mm[1] / 1000.0,
            origin_mm[2] / 1000.0,
        ],
        origin_rpy_rad: [
            rpy_deg[0].to_radians(),
            rpy_deg[1].to_radians(),
            rpy_deg[2].to_radians(),
        ],
        axis: j.axis,
        lower: j.lower,
        upper: j.upper,
        effort: j.effort,
        velocity: j.velocity,
    }
}

/// Emit a **minimal** robot JSON object (serde_json::Value) compatible with `RobotSpec`.
/// Placeholder 50mm cube visuals + density-based inertials — not real CAD solids.
pub fn assembly_to_robot_json(spec: &AssemblySpec) -> Result<serde_json::Value, Vec<String>> {
    let kin = assembly_kinematics(spec)?;
    let density = 1000.0_f64; // kg/m³ water-ish placeholder
    let size = [0.05_f64, 0.05, 0.05]; // 50 mm cube
    let mass = density * size[0] * size[1] * size[2];
    let ixx = mass * (size[1] * size[1] + size[2] * size[2]) / 12.0;
    let iyy = mass * (size[0] * size[0] + size[2] * size[2]) / 12.0;
    let izz = mass * (size[0] * size[0] + size[1] * size[1]) / 12.0;

    let links: Vec<serde_json::Value> = kin
        .links
        .iter()
        .map(|l| {
            serde_json::json!({
                "name": l.name,
                "inertial": {
                    "origin_xyz": [0.0, 0.0, 0.0],
                    "origin_rpy": [0.0, 0.0, 0.0],
                    "mass": mass,
                    "ixx": ixx,
                    "ixy": 0.0,
                    "ixz": 0.0,
                    "iyy": iyy,
                    "iyz": 0.0,
                    "izz": izz,
                },
                "visual": [{
                    "origin": { "xyz": [0.0, 0.0, 0.0], "rpy": [0.0, 0.0, 0.0] },
                    "geometry": { "box": { "size": size } },
                    "material": { "name": "placeholder", "rgba": [0.6, 0.6, 0.7, 1.0] }
                }],
                "collision": [{
                    "origin": { "xyz": [0.0, 0.0, 0.0], "rpy": [0.0, 0.0, 0.0] },
                    "geometry": { "box": { "size": size } }
                }]
            })
        })
        .collect();

    let joints: Vec<serde_json::Value> = kin
        .joints
        .iter()
        .map(|j| {
            serde_json::json!({
                "name": j.name,
                "type": j.kind,
                "parent": j.parent,
                "child": j.child,
                "origin": {
                    "xyz": j.origin_xyz_m,
                    "rpy": j.origin_rpy_rad,
                },
                "axis": j.axis,
                "lower": j.lower,
                "upper": j.upper,
                "effort": j.effort,
                "velocity": j.velocity,
            })
        })
        .collect();

    Ok(serde_json::json!({
        "name": kin.name,
        "links": links,
        "joints": joints,
        "_cadrion": {
            "from_assembly": true,
            "kinematics_schema": "cadrion.assembly_kinematics",
            "note": "placeholder geometry/inertial — not CAD mesh; H3-4 OQ-4 partial"
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assembly::{ComponentSpec, JointSpec, PlacementSpec};
    use std::path::PathBuf;

    fn lid_hinge() -> AssemblySpec {
        AssemblySpec {
            name: "lid_hinge_assy".into(),
            version: 2,
            components: vec![
                ComponentSpec {
                    name: "base".into(),
                    source: "cad/plate.cad.star".into(),
                    from_lock: false,
                    placement: PlacementSpec::default(),
                    datums: Default::default(),
                },
                ComponentSpec {
                    name: "lid".into(),
                    source: "cad/plate.cad.star".into(),
                    from_lock: false,
                    placement: PlacementSpec {
                        origin_mm: [0.0, 0.0, 20.0],
                        rpy_deg: [0.0, 0.0, 0.0],
                    },
                    datums: Default::default(),
                },
            ],
            joints: vec![JointSpec {
                name: "lid_hinge".into(),
                a: "base".into(),
                b: "lid".into(),
                kind: "revolute".into(),
                axis: [0.0, 1.0, 0.0],
                origin_mm: [0.0, 0.0, 20.0],
                lower: Some(0.0),
                upper: Some(1.57079632679),
                effort: Some(2.0),
                velocity: Some(1.0),
            }],
        }
    }

    #[test]
    fn kinematics_mm_to_m() {
        let k = assembly_kinematics(&lid_hinge()).unwrap();
        assert!(k.ok);
        assert_eq!(k.schema, "cadrion.assembly_kinematics");
        assert_eq!(k.joints.len(), 1);
        let j = &k.joints[0];
        assert!((j.origin_xyz_m[2] - 0.02).abs() < 1e-12);
        assert_eq!(j.parent, "base");
        assert_eq!(j.child, "lid");
    }

    #[test]
    fn robot_json_has_links_and_joints() {
        let v = assembly_to_robot_json(&lid_hinge()).unwrap();
        assert_eq!(v["name"], "lid_hinge_assy");
        assert_eq!(v["links"].as_array().unwrap().len(), 2);
        assert_eq!(v["joints"].as_array().unwrap().len(), 1);
        assert_eq!(v["joints"][0]["type"], "revolute");
    }

    #[test]
    fn fixture_lid_hinge_file() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/assembly");
        let text = std::fs::read_to_string(root.join("lid_hinge.assy.json")).unwrap();
        let spec: AssemblySpec = serde_json::from_str(&text).unwrap();
        let k = assembly_kinematics(&spec).unwrap();
        assert_eq!(k.joints[0].name, "lid_hinge");
    }
}
