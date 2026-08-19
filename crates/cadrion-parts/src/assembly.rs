//! Assembly specification (FR-106 data model) + joint validation + align check.

use cadrion_kernel::{Point3, Vec3};
use serde::{Deserialize, Serialize};

/// Explicit placement: origin + axis-aligned for v0 (rotation as ZYX degrees later).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlacementSpec {
    pub origin_mm: [f64; 3],
    /// Euler XYZ degrees (applied X then Y then Z) — identity default.
    #[serde(default)]
    pub rpy_deg: [f64; 3],
}

impl Default for PlacementSpec {
    fn default() -> Self {
        Self {
            origin_mm: [0.0, 0.0, 0.0],
            rpy_deg: [0.0, 0.0, 0.0],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentSpec {
    pub name: String,
    /// Path to `.cad.star` or lock key for catalog part.
    pub source: String,
    #[serde(default)]
    pub from_lock: bool,
    #[serde(default)]
    pub placement: PlacementSpec,
    #[serde(default)]
    pub datums: std::collections::BTreeMap<String, [f64; 3]>,
}

/// Assembly joint (H2-5): fixed | revolute | prismatic with optional limits.
///
/// Not full AP242 kinematics — labels + axis + limit envelope for fail-closed checks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JointSpec {
    pub name: String,
    pub a: String,
    pub b: String,
    /// Kind: fixed | revolute | prismatic.
    #[serde(default = "fixed_kind")]
    pub kind: String,
    /// Axis in parent (a) frame. Default +Z.
    #[serde(default = "default_axis")]
    pub axis: [f64; 3],
    /// Joint origin in parent frame (mm).
    #[serde(default)]
    pub origin_mm: [f64; 3],
    /// Lower limit: radians (revolute) or mm (prismatic).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lower: Option<f64>,
    /// Upper limit: radians (revolute) or mm (prismatic).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upper: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub velocity: Option<f64>,
}

fn fixed_kind() -> String {
    "fixed".into()
}

fn default_axis() -> [f64; 3] {
    [0.0, 0.0, 1.0]
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssemblySpec {
    pub name: String,
    pub version: u32,
    pub components: Vec<ComponentSpec>,
    #[serde(default)]
    pub joints: Vec<JointSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssemblyValidationReport {
    pub ok: bool,
    pub name: String,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub joint_count: usize,
    pub component_count: usize,
}

/// Fail-closed joint/component validation (H2-5).
pub fn validate_assembly(spec: &AssemblySpec) -> AssemblyValidationReport {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    if spec.name.is_empty() {
        errors.push("assembly name empty".into());
    }
    if spec.components.is_empty() {
        errors.push("no components".into());
    }

    let mut names = std::collections::HashSet::new();
    for c in &spec.components {
        if c.name.is_empty() {
            errors.push("component with empty name".into());
            continue;
        }
        if !names.insert(c.name.clone()) {
            errors.push(format!("duplicate component '{}'", c.name));
        }
        if c.source.is_empty() {
            errors.push(format!("component '{}': empty source", c.name));
        }
    }

    let mut joint_names = std::collections::HashSet::new();
    for j in &spec.joints {
        if j.name.is_empty() {
            errors.push("joint with empty name".into());
            continue;
        }
        if !joint_names.insert(j.name.clone()) {
            errors.push(format!("duplicate joint '{}'", j.name));
        }
        if !names.contains(&j.a) {
            errors.push(format!("joint '{}': unknown component a='{}'", j.name, j.a));
        }
        if !names.contains(&j.b) {
            errors.push(format!("joint '{}': unknown component b='{}'", j.name, j.b));
        }
        if j.a == j.b {
            errors.push(format!("joint '{}': a==b", j.name));
        }

        let kind = j.kind.to_ascii_lowercase();
        match kind.as_str() {
            "fixed" => {
                if j.lower.is_some() || j.upper.is_some() {
                    warnings.push(format!(
                        "joint '{}': fixed joint ignores lower/upper limits",
                        j.name
                    ));
                }
            }
            "revolute" | "prismatic" => {
                let axis_n =
                    (j.axis[0] * j.axis[0] + j.axis[1] * j.axis[1] + j.axis[2] * j.axis[2]).sqrt();
                if axis_n < 1e-9 {
                    errors.push(format!("joint '{}': zero axis", j.name));
                }
                match (j.lower, j.upper) {
                    (None, None) => errors.push(format!(
                        "joint '{}': {} requires lower and upper limits",
                        j.name, kind
                    )),
                    (None, Some(_)) | (Some(_), None) => errors.push(format!(
                        "joint '{}': {} requires both lower and upper",
                        j.name, kind
                    )),
                    (Some(lo), Some(hi)) if lo > hi => {
                        errors.push(format!("joint '{}': lower ({lo}) > upper ({hi})", j.name))
                    }
                    (Some(_), Some(_)) => {}
                }
                if let Some(e) = j.effort {
                    if e < 0.0 {
                        errors.push(format!("joint '{}': negative effort", j.name));
                    }
                }
                if let Some(v) = j.velocity {
                    if v < 0.0 {
                        errors.push(format!("joint '{}': negative velocity", j.name));
                    }
                }
            }
            other => errors.push(format!(
                "joint '{}': unknown kind '{}' (expected fixed|revolute|prismatic)",
                j.name, other
            )),
        }
    }

    AssemblyValidationReport {
        ok: errors.is_empty(),
        name: spec.name.clone(),
        errors,
        warnings,
        joint_count: spec.joints.len(),
        component_count: spec.components.len(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlignExpect {
    Coplanar,
    Coaxial,
    Distance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlignReport {
    pub ok: bool,
    pub expect: String,
    pub translation_err_mm: f64,
    pub angular_err_deg: f64,
    pub distance_mm: Option<f64>,
    pub tol_mm: f64,
    pub tol_deg: f64,
    pub detail: String,
}

/// Simple align between two world-space points + optional normals.
#[allow(clippy::too_many_arguments)]
pub fn align_check(
    a_origin: Point3,
    a_normal: Option<Vec3>,
    b_origin: Point3,
    b_normal: Option<Vec3>,
    expect: AlignExpect,
    expect_distance: Option<f64>,
    tol_mm: f64,
    tol_deg: f64,
) -> AlignReport {
    let dx = b_origin.x - a_origin.x;
    let dy = b_origin.y - a_origin.y;
    let dz = b_origin.z - a_origin.z;
    let dist = (dx * dx + dy * dy + dz * dz).sqrt();
    let ang = match (a_normal, b_normal) {
        (Some(na), Some(nb)) => {
            let dot = (na.x * nb.x + na.y * nb.y + na.z * nb.z).clamp(-1.0, 1.0);
            dot.abs().acos().to_degrees() // 0 = parallel (same or opposite)
        }
        _ => 0.0,
    };

    let (ok, detail) = match expect {
        AlignExpect::Coplanar => {
            let ok = ang <= tol_deg && dist <= tol_mm;
            (ok, format!("coplanar check ang={ang:.4} dist={dist:.4}"))
        }
        AlignExpect::Coaxial => {
            let ok = ang <= tol_deg;
            (ok, format!("coaxial/parallel normals ang={ang:.4}"))
        }
        AlignExpect::Distance => {
            let want = expect_distance.unwrap_or(0.0);
            let err = (dist - want).abs();
            (
                err <= tol_mm,
                format!("distance got={dist:.4} want={want:.4}"),
            )
        }
    };

    AlignReport {
        ok,
        expect: format!("{expect:?}").to_ascii_lowercase(),
        translation_err_mm: dist,
        angular_err_deg: ang,
        distance_mm: Some(dist),
        tol_mm,
        tol_deg,
        detail,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distance_align() {
        let r = align_check(
            Point3::ORIGIN,
            None,
            Point3::new(10.0, 0.0, 0.0),
            None,
            AlignExpect::Distance,
            Some(10.0),
            0.1,
            1.0,
        );
        assert!(r.ok);
    }

    #[test]
    fn assembly_json_roundtrip() {
        let a = AssemblySpec {
            name: "bracket_assy".into(),
            version: 1,
            components: vec![
                ComponentSpec {
                    name: "plate".into(),
                    source: "plate.cad.star".into(),
                    from_lock: false,
                    placement: PlacementSpec::default(),
                    datums: Default::default(),
                },
                ComponentSpec {
                    name: "bolt".into(),
                    source: "m6_bolt".into(),
                    from_lock: true,
                    placement: PlacementSpec {
                        origin_mm: [0.0, 0.0, 5.0],
                        rpy_deg: [0.0, 0.0, 0.0],
                    },
                    datums: Default::default(),
                },
            ],
            joints: vec![JointSpec {
                name: "bolt_to_plate".into(),
                a: "plate".into(),
                b: "bolt".into(),
                kind: "fixed".into(),
                axis: default_axis(),
                origin_mm: [0.0, 0.0, 0.0],
                lower: None,
                upper: None,
                effort: None,
                velocity: None,
            }],
        };
        let j = serde_json::to_string(&a).unwrap();
        let b: AssemblySpec = serde_json::from_str(&j).unwrap();
        assert_eq!(a, b);
        assert!(validate_assembly(&a).ok);
    }

    #[test]
    fn revolute_requires_limits() {
        let a = AssemblySpec {
            name: "hinge".into(),
            version: 1,
            components: vec![
                ComponentSpec {
                    name: "base".into(),
                    source: "a.cad.star".into(),
                    from_lock: false,
                    placement: PlacementSpec::default(),
                    datums: Default::default(),
                },
                ComponentSpec {
                    name: "lid".into(),
                    source: "b.cad.star".into(),
                    from_lock: false,
                    placement: PlacementSpec::default(),
                    datums: Default::default(),
                },
            ],
            joints: vec![JointSpec {
                name: "hinge".into(),
                a: "base".into(),
                b: "lid".into(),
                kind: "revolute".into(),
                axis: [0.0, 1.0, 0.0],
                origin_mm: [0.0, 0.0, 10.0],
                lower: None,
                upper: None,
                effort: None,
                velocity: None,
            }],
        };
        let r = validate_assembly(&a);
        assert!(!r.ok);
        assert!(r.errors.iter().any(|e| e.contains("requires lower")));
    }

    #[test]
    fn inverted_limits_fail_closed() {
        let a = AssemblySpec {
            name: "bad".into(),
            version: 1,
            components: vec![
                ComponentSpec {
                    name: "a".into(),
                    source: "a".into(),
                    from_lock: false,
                    placement: PlacementSpec::default(),
                    datums: Default::default(),
                },
                ComponentSpec {
                    name: "b".into(),
                    source: "b".into(),
                    from_lock: false,
                    placement: PlacementSpec::default(),
                    datums: Default::default(),
                },
            ],
            joints: vec![JointSpec {
                name: "slide".into(),
                a: "a".into(),
                b: "b".into(),
                kind: "prismatic".into(),
                axis: [1.0, 0.0, 0.0],
                origin_mm: [0.0, 0.0, 0.0],
                lower: Some(10.0),
                upper: Some(0.0),
                effort: None,
                velocity: None,
            }],
        };
        let r = validate_assembly(&a);
        assert!(!r.ok);
        assert!(r
            .errors
            .iter()
            .any(|e| e.contains("lower") && e.contains("upper")));
    }

    #[test]
    fn good_revolute_ok() {
        let a = AssemblySpec {
            name: "lid_assy".into(),
            version: 2,
            components: vec![
                ComponentSpec {
                    name: "base".into(),
                    source: "base.cad.star".into(),
                    from_lock: false,
                    placement: PlacementSpec::default(),
                    datums: Default::default(),
                },
                ComponentSpec {
                    name: "lid".into(),
                    source: "lid.cad.star".into(),
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
                upper: Some(1.57),
                effort: Some(2.0),
                velocity: Some(1.0),
            }],
        };
        let r = validate_assembly(&a);
        assert!(r.ok, "{r:?}");
    }
}
