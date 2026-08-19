//! Numeric interrogation results (facts / validity / mass).

use serde::{Deserialize, Serialize};

use crate::types::{BBox, Point3};

/// Geometry facts returned after build / `inspect refs --facts`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShapeFacts {
    pub bbox_mm: BBox,
    pub volume_mm3: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub area_mm2: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub centroid_mm: Option<Point3>,
    pub solids: u32,
    pub faces: u32,
    pub edges: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vertices: Option<u32>,
    /// Mass in grams when density was supplied.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mass_g: Option<f64>,
}

/// Validity / heal report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidityReport {
    pub closed: bool,
    pub positive_volume: bool,
    pub shells: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

impl ValidityReport {
    pub fn ok_solid() -> Self {
        Self {
            closed: true,
            positive_volume: true,
            shells: 1,
            notes: Vec::new(),
        }
    }
}
