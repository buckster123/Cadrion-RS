//! Tessellation output (secondary mesh path — not the modeling medium).

use serde::{Deserialize, Serialize};

/// Triangle mesh in model space (mm).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Mesh {
    /// Interleaved xyz positions.
    pub positions: Vec<f32>,
    /// Interleaved xyz normals (same length as positions when present).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub normals: Option<Vec<f32>>,
    /// Triangle indices into the vertex list (not float components).
    pub indices: Vec<u32>,
}

impl Mesh {
    pub fn vertex_count(&self) -> usize {
        self.positions.len() / 3
    }

    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }
}
