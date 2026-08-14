//! DFM engine + versioned vendor profiles (data-driven).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DfmSeverity {
    Pass,
    Warn,
    Fail,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DfmFinding {
    pub rule: String,
    pub severity: DfmSeverity,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub measured: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DfmReport {
    pub ok: bool,
    pub profile_id: String,
    pub profile_version: String,
    pub findings: Vec<DfmFinding>,
}

pub const DFM_PROFILE_SCHEMA: &str = "cadre.dfm_profile";
pub const DFM_OVERRIDE_SCHEMA: &str = "cadre.dfm_override";
pub const DFM_SCHEMA_VERSION: u32 = 1;

fn default_profile_schema() -> String {
    DFM_PROFILE_SCHEMA.into()
}

fn default_schema_version() -> u32 {
    DFM_SCHEMA_VERSION
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DfmProfile {
    #[serde(default = "default_profile_schema")]
    pub schema: String,
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    pub id: String,
    pub version: String,
    pub vendor: String,
    /// mm
    pub materials: Vec<MaterialOption>,
    pub rules: DfmRules,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialOption {
    pub name: String,
    /// available thicknesses mm
    pub thicknesses_mm: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DfmRules {
    /// min hole diameter as multiple of thickness (e.g. 1.0 => d >= t)
    pub min_hole_dia_vs_thickness: f64,
    /// absolute min hole diameter mm
    pub min_hole_dia_mm: f64,
    /// min web/bridge between holes or to edge mm
    pub min_web_mm: f64,
    /// min feature overall size mm
    pub min_part_size_mm: f64,
}

/// Abstract flat part for checks (from DXF/projection facts).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlatPart {
    pub width_mm: f64,
    pub height_mm: f64,
    pub thickness_mm: f64,
    pub material: String,
    /// hole diameters mm
    pub holes_dia_mm: Vec<f64>,
    /// optional min edge distance for holes mm
    #[serde(default)]
    pub min_hole_edge_mm: Option<f64>,
    /// optional min hole-hole spacing mm
    #[serde(default)]
    pub min_hole_spacing_mm: Option<f64>,
}

/// Built-in SendCutSend-style laser profile (profile-version truth, not live vendor API).
pub fn sendcutsend_laser_v1() -> DfmProfile {
    DfmProfile {
        schema: DFM_PROFILE_SCHEMA.into(),
        schema_version: DFM_SCHEMA_VERSION,
        id: "sendcutsend.laser".into(),
        version: "1.0.0".into(),
        vendor: "SendCutSend-style (bundled profile)".into(),
        materials: vec![
            MaterialOption {
                name: "Aluminum 5052".into(),
                thicknesses_mm: vec![1.0, 1.5, 2.0, 3.0, 4.0, 6.0],
            },
            MaterialOption {
                name: "Stainless 304".into(),
                thicknesses_mm: vec![1.0, 1.5, 2.0, 3.0],
            },
            MaterialOption {
                name: "Mild Steel".into(),
                thicknesses_mm: vec![1.5, 2.0, 3.0, 6.0],
            },
        ],
        rules: DfmRules {
            min_hole_dia_vs_thickness: 1.0,
            min_hole_dia_mm: 1.0,
            min_web_mm: 1.0,
            min_part_size_mm: 6.0,
        },
    }
}

/// Bundled PCB outline profile (generic fab house, not a live API).
/// Stricter holes / smaller webs typical of FR4 routing.
pub fn pcb_outline_v1() -> DfmProfile {
    DfmProfile {
        schema: DFM_PROFILE_SCHEMA.into(),
        schema_version: DFM_SCHEMA_VERSION,
        id: "pcb.outline".into(),
        version: "1.0.0".into(),
        vendor: "Generic PCB outline (bundled profile)".into(),
        materials: vec![
            MaterialOption {
                name: "FR4".into(),
                thicknesses_mm: vec![0.8, 1.0, 1.2, 1.6, 2.0],
            },
            MaterialOption {
                name: "Aluminum PCB".into(),
                thicknesses_mm: vec![1.0, 1.5, 2.0],
            },
        ],
        rules: DfmRules {
            min_hole_dia_vs_thickness: 0.25,
            min_hole_dia_mm: 0.3,
            min_web_mm: 0.25,
            min_part_size_mm: 5.0,
        },
    }
}

/// Bundled waterjet / abrasive-cut style profile (generic, not a live vendor API).
pub fn waterjet_v1() -> DfmProfile {
    DfmProfile {
        schema: DFM_PROFILE_SCHEMA.into(),
        schema_version: DFM_SCHEMA_VERSION,
        id: "waterjet.generic".into(),
        version: "1.0.0".into(),
        vendor: "Generic waterjet (bundled profile)".into(),
        materials: vec![
            MaterialOption {
                name: "Aluminum 6061".into(),
                thicknesses_mm: vec![1.5, 3.0, 6.0, 12.0, 25.0],
            },
            MaterialOption {
                name: "Stainless 304".into(),
                thicknesses_mm: vec![1.5, 3.0, 6.0, 12.0],
            },
            MaterialOption {
                name: "Mild Steel".into(),
                thicknesses_mm: vec![3.0, 6.0, 10.0, 20.0],
            },
            MaterialOption {
                name: "HDPE".into(),
                thicknesses_mm: vec![3.0, 6.0, 12.0, 25.0],
            },
        ],
        rules: DfmRules {
            // Waterjet tolerates smaller holes vs thickness than laser in some shops
            min_hole_dia_vs_thickness: 0.5,
            min_hole_dia_mm: 1.5,
            min_web_mm: 1.5,
            min_part_size_mm: 10.0,
        },
    }
}

/// All bundled profiles (id + version).
pub fn bundled_profiles() -> Vec<DfmProfile> {
    vec![sendcutsend_laser_v1(), pcb_outline_v1(), waterjet_v1()]
}

pub fn resolve_bundled_profile(id: &str) -> Option<DfmProfile> {
    match id {
        "sendcutsend.laser" | "sendcutsend.laser@1" | "scs" => Some(sendcutsend_laser_v1()),
        "pcb.outline" | "pcb.outline@1" | "pcb" | "jlcpcb.outline" => Some(pcb_outline_v1()),
        "waterjet.generic" | "waterjet.generic@1" | "waterjet" | "wj" => Some(waterjet_v1()),
        _ => None,
    }
}

pub fn check_dfm(profile: &DfmProfile, part: &FlatPart) -> DfmReport {
    let mut findings = Vec::new();

    // material + thickness availability
    let mat = profile
        .materials
        .iter()
        .find(|m| m.name.eq_ignore_ascii_case(&part.material));
    match mat {
        None => findings.push(DfmFinding {
            rule: "material_available".into(),
            severity: DfmSeverity::Fail,
            message: format!(
                "material '{}' not in profile {}@{}",
                part.material, profile.id, profile.version
            ),
            measured: None,
            limit: None,
        }),
        Some(m) => {
            let ok_t = m
                .thicknesses_mm
                .iter()
                .any(|t| (*t - part.thickness_mm).abs() < 1e-6);
            if ok_t {
                findings.push(DfmFinding {
                    rule: "material_available".into(),
                    severity: DfmSeverity::Pass,
                    message: format!(
                        "{} @ {} mm available in profile",
                        part.material, part.thickness_mm
                    ),
                    measured: Some(part.thickness_mm),
                    limit: None,
                });
            } else {
                findings.push(DfmFinding {
                    rule: "thickness_available".into(),
                    severity: DfmSeverity::Fail,
                    message: format!(
                        "thickness {} mm not listed for {} (have {:?})",
                        part.thickness_mm, part.material, m.thicknesses_mm
                    ),
                    measured: Some(part.thickness_mm),
                    limit: None,
                });
            }
        }
    }

    // part size
    let min_dim = part.width_mm.min(part.height_mm);
    if min_dim + 1e-9 < profile.rules.min_part_size_mm {
        findings.push(DfmFinding {
            rule: "min_part_size".into(),
            severity: DfmSeverity::Fail,
            message: format!(
                "min dimension {min_dim} mm < {}",
                profile.rules.min_part_size_mm
            ),
            measured: Some(min_dim),
            limit: Some(profile.rules.min_part_size_mm),
        });
    } else {
        findings.push(DfmFinding {
            rule: "min_part_size".into(),
            severity: DfmSeverity::Pass,
            message: format!("min dimension {min_dim} mm ok"),
            measured: Some(min_dim),
            limit: Some(profile.rules.min_part_size_mm),
        });
    }

    // holes
    for (i, d) in part.holes_dia_mm.iter().enumerate() {
        let need = profile
            .rules
            .min_hole_dia_mm
            .max(profile.rules.min_web_mm) // keep simple
            .max(part.thickness_mm * profile.rules.min_hole_dia_vs_thickness);
        if *d + 1e-9 < need {
            findings.push(DfmFinding {
                rule: format!("hole_dia[{i}]"),
                severity: DfmSeverity::Fail,
                message: format!("hole dia {d} mm < required {need} mm (t-based)"),
                measured: Some(*d),
                limit: Some(need),
            });
        } else {
            findings.push(DfmFinding {
                rule: format!("hole_dia[{i}]"),
                severity: DfmSeverity::Pass,
                message: format!("hole dia {d} mm ok (>= {need})"),
                measured: Some(*d),
                limit: Some(need),
            });
        }
    }

    if let Some(edge) = part.min_hole_edge_mm {
        if edge + 1e-9 < profile.rules.min_web_mm {
            findings.push(DfmFinding {
                rule: "hole_edge_web".into(),
                severity: DfmSeverity::Fail,
                message: format!(
                    "hole-edge distance {edge} mm < min web {}",
                    profile.rules.min_web_mm
                ),
                measured: Some(edge),
                limit: Some(profile.rules.min_web_mm),
            });
        } else {
            findings.push(DfmFinding {
                rule: "hole_edge_web".into(),
                severity: DfmSeverity::Pass,
                message: format!("hole-edge {edge} mm ok"),
                measured: Some(edge),
                limit: Some(profile.rules.min_web_mm),
            });
        }
    }

    if let Some(sp) = part.min_hole_spacing_mm {
        if sp + 1e-9 < profile.rules.min_web_mm {
            findings.push(DfmFinding {
                rule: "hole_spacing_web".into(),
                severity: DfmSeverity::Warn,
                message: format!(
                    "hole spacing {sp} mm < min web {}",
                    profile.rules.min_web_mm
                ),
                measured: Some(sp),
                limit: Some(profile.rules.min_web_mm),
            });
        } else {
            findings.push(DfmFinding {
                rule: "hole_spacing_web".into(),
                severity: DfmSeverity::Pass,
                message: format!("hole spacing {sp} mm ok"),
                measured: Some(sp),
                limit: Some(profile.rules.min_web_mm),
            });
        }
    }

    let ok = !findings.iter().any(|f| f.severity == DfmSeverity::Fail);
    DfmReport {
        ok,
        profile_id: profile.id.clone(),
        profile_version: profile.version.clone(),
        findings,
    }
}

pub fn load_profile_json(text: &str) -> Result<DfmProfile, String> {
    let p: DfmProfile = serde_json::from_str(text).map_err(|e| e.to_string())?;
    validate_profile(&p)?;
    Ok(p)
}

/// Fail-closed profile schema check (H3-8).
pub fn validate_profile(p: &DfmProfile) -> Result<(), String> {
    if p.schema != DFM_PROFILE_SCHEMA {
        return Err(format!(
            "unknown DFM schema '{}' (want {DFM_PROFILE_SCHEMA})",
            p.schema
        ));
    }
    if p.schema_version != DFM_SCHEMA_VERSION {
        return Err(format!(
            "unsupported DFM schema_version {} (want {DFM_SCHEMA_VERSION})",
            p.schema_version
        ));
    }
    if p.id.trim().is_empty() {
        return Err("profile id empty".into());
    }
    if !is_semver_like(&p.version) {
        return Err(format!(
            "profile version '{}' is not semver-like N.N.N",
            p.version
        ));
    }
    if p.materials.is_empty() {
        return Err("profile has no materials".into());
    }
    for m in &p.materials {
        if m.name.trim().is_empty() {
            return Err("material name empty".into());
        }
        if m.thicknesses_mm.is_empty() {
            return Err(format!("material '{}' has no thicknesses", m.name));
        }
        if m.thicknesses_mm.iter().any(|t| !t.is_finite() || *t <= 0.0) {
            return Err(format!("material '{}' has non-positive thickness", m.name));
        }
    }
    let r = &p.rules;
    for (name, v) in [
        ("min_hole_dia_vs_thickness", r.min_hole_dia_vs_thickness),
        ("min_hole_dia_mm", r.min_hole_dia_mm),
        ("min_web_mm", r.min_web_mm),
        ("min_part_size_mm", r.min_part_size_mm),
    ] {
        if !v.is_finite() || v < 0.0 {
            return Err(format!("rule {name} must be finite >= 0 (got {v})"));
        }
    }
    Ok(())
}

fn is_semver_like(v: &str) -> bool {
    let mut parts = v.split('.');
    let a = parts.next().and_then(|s| s.parse::<u32>().ok());
    let b = parts.next().and_then(|s| s.parse::<u32>().ok());
    let c = parts.next().and_then(|s| s.parse::<u32>().ok());
    a.is_some() && b.is_some() && c.is_some() && parts.next().is_none()
}

/// Community overlay: pin a bundled base version, then patch rules. Fail-closed on drift.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DfmOverride {
    #[serde(default = "default_override_schema")]
    pub schema: String,
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    pub base: String,
    pub base_version: String,
    #[serde(default)]
    pub rules: DfmRulesPatch,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

fn default_override_schema() -> String {
    DFM_OVERRIDE_SCHEMA.into()
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DfmRulesPatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_hole_dia_vs_thickness: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_hole_dia_mm: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_web_mm: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_part_size_mm: Option<f64>,
}

pub fn load_override_json(text: &str) -> Result<DfmOverride, String> {
    let o: DfmOverride = serde_json::from_str(text).map_err(|e| e.to_string())?;
    if o.schema != DFM_OVERRIDE_SCHEMA {
        return Err(format!(
            "unknown override schema '{}' (want {DFM_OVERRIDE_SCHEMA})",
            o.schema
        ));
    }
    if o.schema_version != DFM_SCHEMA_VERSION {
        return Err(format!(
            "unsupported override schema_version {} (want {DFM_SCHEMA_VERSION})",
            o.schema_version
        ));
    }
    if o.base.trim().is_empty() {
        return Err("override base empty".into());
    }
    if !is_semver_like(&o.base_version) {
        return Err(format!(
            "override base_version '{}' is not semver-like N.N.N",
            o.base_version
        ));
    }
    Ok(o)
}

/// Apply overlay to a bundled profile. **Refuse** if `base_version` ≠ profile.version.
pub fn apply_override(base: &DfmProfile, ov: &DfmOverride) -> Result<DfmProfile, String> {
    if ov.base != base.id {
        return Err(format!(
            "override base '{}' does not match profile '{}'",
            ov.base, base.id
        ));
    }
    if ov.base_version != base.version {
        return Err(format!(
            "DFM drift: override pins {}@{} but bundled is {}@{} — bump override or profile explicitly",
            ov.base, ov.base_version, base.id, base.version
        ));
    }
    let mut out = base.clone();
    if let Some(v) = ov.rules.min_hole_dia_vs_thickness {
        out.rules.min_hole_dia_vs_thickness = v;
    }
    if let Some(v) = ov.rules.min_hole_dia_mm {
        out.rules.min_hole_dia_mm = v;
    }
    if let Some(v) = ov.rules.min_web_mm {
        out.rules.min_web_mm = v;
    }
    if let Some(v) = ov.rules.min_part_size_mm {
        out.rules.min_part_size_mm = v;
    }
    if let Some(n) = &ov.note {
        out.vendor = format!("{} [override: {n}]", out.vendor);
    }
    validate_profile(&out)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn good_plate_passes() {
        let p = sendcutsend_laser_v1();
        let part = FlatPart {
            width_mm: 100.0,
            height_mm: 50.0,
            thickness_mm: 3.0,
            material: "Aluminum 5052".into(),
            holes_dia_mm: vec![6.0, 6.0],
            min_hole_edge_mm: Some(5.0),
            min_hole_spacing_mm: Some(10.0),
        };
        let r = check_dfm(&p, &part);
        assert!(r.ok, "{r:?}");
    }

    #[test]
    fn tiny_hole_fails() {
        let p = sendcutsend_laser_v1();
        let part = FlatPart {
            width_mm: 40.0,
            height_mm: 40.0,
            thickness_mm: 3.0,
            material: "Aluminum 5052".into(),
            holes_dia_mm: vec![1.5],
            min_hole_edge_mm: Some(5.0),
            min_hole_spacing_mm: None,
        };
        let r = check_dfm(&p, &part);
        assert!(!r.ok);
        assert!(r
            .findings
            .iter()
            .any(|f| f.rule.starts_with("hole_dia") && f.severity == DfmSeverity::Fail));
    }

    #[test]
    fn pcb_profile_allows_small_via() {
        let p = pcb_outline_v1();
        assert_eq!(p.id, "pcb.outline");
        let part = FlatPart {
            width_mm: 50.0,
            height_mm: 40.0,
            thickness_mm: 1.6,
            material: "FR4".into(),
            holes_dia_mm: vec![0.4],
            min_hole_edge_mm: Some(1.0),
            min_hole_spacing_mm: Some(0.5),
        };
        let r = check_dfm(&p, &part);
        assert!(r.ok, "{r:?}");
    }

    #[test]
    fn resolve_bundled_ids() {
        assert!(resolve_bundled_profile("scs").is_some());
        assert!(resolve_bundled_profile("pcb").is_some());
        assert!(resolve_bundled_profile("waterjet").is_some());
        assert!(resolve_bundled_profile("nope").is_none());
        assert_eq!(bundled_profiles().len(), 3);
    }

    #[test]
    fn waterjet_plate_passes() {
        let p = waterjet_v1();
        assert_eq!(p.id, "waterjet.generic");
        let part = FlatPart {
            width_mm: 120.0,
            height_mm: 80.0,
            thickness_mm: 6.0,
            material: "Aluminum 6061".into(),
            holes_dia_mm: vec![4.0],
            min_hole_edge_mm: Some(5.0),
            min_hole_spacing_mm: Some(8.0),
        };
        let r = check_dfm(&p, &part);
        assert!(r.ok, "{r:?}");
    }

    #[test]
    fn validate_bundled() {
        for p in bundled_profiles() {
            validate_profile(&p).unwrap();
        }
    }

    #[test]
    fn reject_bad_schema() {
        let mut p = sendcutsend_laser_v1();
        p.schema = "nope".into();
        assert!(validate_profile(&p)
            .unwrap_err()
            .contains("unknown DFM schema"));
    }

    #[test]
    fn override_pins_version() {
        let base = waterjet_v1();
        let ov = DfmOverride {
            schema: DFM_OVERRIDE_SCHEMA.into(),
            schema_version: 1,
            base: "waterjet.generic".into(),
            base_version: "1.0.0".into(),
            rules: DfmRulesPatch {
                min_web_mm: Some(2.5),
                ..Default::default()
            },
            note: Some("shop tighter web".into()),
        };
        let p = apply_override(&base, &ov).unwrap();
        assert!((p.rules.min_web_mm - 2.5).abs() < 1e-12);
        assert!(p.vendor.contains("override"));
    }

    #[test]
    fn override_drift_is_error() {
        let base = waterjet_v1();
        let ov = DfmOverride {
            schema: DFM_OVERRIDE_SCHEMA.into(),
            schema_version: 1,
            base: "waterjet.generic".into(),
            base_version: "9.9.9".into(),
            rules: DfmRulesPatch::default(),
            note: None,
        };
        let err = apply_override(&base, &ov).unwrap_err();
        assert!(err.contains("DFM drift"), "{err}");
    }

    #[test]
    fn load_legacy_json_gets_schema_defaults() {
        let raw = r#"{
            "id":"custom.shop","version":"1.2.3","vendor":"shop",
            "materials":[{"name":"Al","thicknesses_mm":[3.0]}],
            "rules":{"min_hole_dia_vs_thickness":1.0,"min_hole_dia_mm":1.0,"min_web_mm":1.0,"min_part_size_mm":6.0}
        }"#;
        let p = load_profile_json(raw).unwrap();
        assert_eq!(p.schema, DFM_PROFILE_SCHEMA);
        assert_eq!(p.schema_version, 1);
    }
}
