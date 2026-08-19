//! expect.json schema for a parity part.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Expect {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub description: String,
    pub volume_mm3: f64,
    #[serde(default = "default_vol_tol")]
    pub volume_tol_frac: f64,
    pub bbox_mm: BBoxExpect,
    #[serde(default = "default_bbox_tol")]
    pub bbox_tol_mm: f64,
    #[serde(default)]
    pub faces_min: u32,
    #[serde(default)]
    pub edges_min: u32,
    #[serde(default)]
    pub required_ops: Vec<String>,
    #[serde(default)]
    pub params: std::collections::BTreeMap<String, f64>,
    #[serde(default)]
    pub measures: Vec<MeasureExpect>,
}

fn default_vol_tol() -> f64 {
    0.005
}
fn default_bbox_tol() -> f64 {
    0.5
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BBoxExpect {
    pub min: [f64; 3],
    pub max: [f64; 3],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeasureExpect {
    pub kind: String,
    pub find_a: FindFace,
    #[serde(default)]
    pub find_b: Option<FindFace>,
    #[serde(default)]
    pub value: Option<f64>,
    /// When set, only check value >= value_min (loose structural check).
    #[serde(default)]
    pub value_min: Option<f64>,
    #[serde(default = "default_meas_tol")]
    pub tol: f64,
}

fn default_meas_tol() -> f64 {
    0.1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindFace {
    pub kind: String,
    #[serde(default)]
    pub normal: Option<[f64; 3]>,
}
