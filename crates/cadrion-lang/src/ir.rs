//! Feature IR — the hashed, diffable representation between Starlark and the kernel.
//!
//! Node ids are dense `u32` indexes into `FeatureIr::nodes`.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Stable IR schema version (bump when node shapes change incompatibly).
pub const IR_VERSION: u32 = 2;

/// Opaque node id within one IR document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NodeId(pub u32);

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "n{}", self.0)
    }
}

/// Placement as `[x, y, z]` mm (rotation deferred to assemblies).
pub type At3 = [f64; 3];

/// One feature operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum IrNode {
    /// Axis-aligned box centered at `at`, extents `dx/dy/dz`.
    Box { dx: f64, dy: f64, dz: f64, at: At3 },
    /// Cylinder along +Z; base center at `at`, height `height`.
    Cylinder { radius: f64, height: f64, at: At3 },
    /// Sphere centered at `at`.
    Sphere { radius: f64, at: At3 },
    /// Cone along +Z; base radius at `at`, tip at `at.z + height` (radius 0).
    Cone { radius: f64, height: f64, at: At3 },
    /// Boolean combination of two prior nodes.
    Boolean {
        kind: BooleanKind,
        a: NodeId,
        b: NodeId,
    },
    /// Fillet; empty `edges` means all edges (kernel-defined order).
    Fillet {
        of: NodeId,
        radius: f64,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        edges: Vec<u32>,
    },
    /// Chamfer; empty `edges` means all edges.
    Chamfer {
        of: NodeId,
        distance: f64,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        edges: Vec<u32>,
    },
    /// Product-structure label on a shape (may share geometry with `of`).
    Label { of: NodeId, name: String },
    /// Translate child by mm offset.
    Translate { of: NodeId, by: At3 },
    /// Rotate child about world origin axis x|y|z by degrees.
    Rotate { of: NodeId, axis: String, deg: f64 },
    /// Mirror child through a coordinate plane: `xy` | `yz` | `zx`.
    Mirror { of: NodeId, plane: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BooleanKind {
    Union,
    Cut,
    Intersect,
}

/// Complete feature graph produced by one `gen_step()` evaluation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FeatureIr {
    pub version: u32,
    /// Declared params after overrides (name → f64).
    pub params: BTreeMap<String, f64>,
    pub nodes: Vec<IrNode>,
    /// Root solid returned by `gen_step()`.
    pub root: NodeId,
    /// Optional product label from `solid(..., label=)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

impl FeatureIr {
    pub fn node(&self, id: NodeId) -> Option<&IrNode> {
        self.nodes.get(id.0 as usize)
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
}

/// In-evaluation builder.
#[derive(Debug, Default)]
pub struct IrBuilder {
    pub params: BTreeMap<String, f64>,
    nodes: Vec<IrNode>,
    pub label: Option<String>,
}

impl IrBuilder {
    pub fn push(&mut self, node: IrNode) -> NodeId {
        let id = NodeId(self.nodes.len() as u32);
        self.nodes.push(node);
        id
    }

    pub fn get(&self, id: NodeId) -> Option<&IrNode> {
        self.nodes.get(id.0 as usize)
    }

    pub fn finish(self, root: NodeId) -> Result<FeatureIr, String> {
        if root.0 as usize >= self.nodes.len() {
            return Err(format!(
                "gen_step returned unknown shape id {} (have {} nodes)",
                root.0,
                self.nodes.len()
            ));
        }
        Ok(FeatureIr {
            version: IR_VERSION,
            params: self.params,
            nodes: self.nodes,
            root,
            label: self.label,
        })
    }
}
