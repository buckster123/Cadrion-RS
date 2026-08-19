//! Task / step schema.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    /// Natural-language prompt (what an agent would see).
    pub prompt: String,
    #[serde(default = "default_max_loops")]
    pub max_loops: u32,
    /// Ordered loops for **scripted** mode. Live mode ignores step bodies and only
    /// uses prompts + success criteria.
    #[serde(default)]
    pub loops: Vec<Vec<Step>>,
    /// Explicit success criteria for live mode (optional; falls back to last assert).
    #[serde(default)]
    pub success: Option<AssertSpec>,
}

impl Task {
    /// Success criteria for live verify: `success` field or last `assert` in last loop.
    pub fn success_assert(&self) -> Option<AssertSpec> {
        if let Some(s) = &self.success {
            return Some(s.clone());
        }
        for loop_steps in self.loops.iter().rev() {
            for step in loop_steps.iter().rev() {
                if let Step::Assert {
                    volume_min,
                    volume_max,
                    faces_min,
                    label,
                    has_selector_prefix,
                    snapshot_ok,
                } = step
                {
                    return Some(AssertSpec {
                        volume_min: *volume_min,
                        volume_max: *volume_max,
                        faces_min: *faces_min,
                        label: label.clone(),
                        has_selector_prefix: has_selector_prefix.clone(),
                        snapshot_ok: *snapshot_ok,
                    });
                }
            }
        }
        None
    }
}

fn default_max_loops() -> u32 {
    3
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Step {
    /// Write Starlark source relative to task workdir.
    Write { path: String, content: String },
    /// Evaluate + execute mock kernel; stash facts.
    Build { path: String },
    /// inspect refs; optional facts.
    InspectRefs {
        path: String,
        #[serde(default)]
        facts: bool,
    },
    /// Software snapshot packet (no images required for score).
    Snapshot {
        path: String,
        #[serde(default = "default_snap_size")]
        size: u32,
    },
    /// Assertions against last build facts / inspect.
    Assert {
        #[serde(default)]
        volume_min: Option<f64>,
        #[serde(default)]
        volume_max: Option<f64>,
        #[serde(default)]
        faces_min: Option<u32>,
        #[serde(default)]
        label: Option<String>,
        #[serde(default)]
        has_selector_prefix: Option<String>,
        #[serde(default)]
        snapshot_ok: bool,
    },
}

fn default_snap_size() -> u32 {
    64
}

/// Shared assert payload (for docs / external tools / live success).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AssertSpec {
    #[serde(default)]
    pub volume_min: Option<f64>,
    #[serde(default)]
    pub volume_max: Option<f64>,
    #[serde(default)]
    pub faces_min: Option<u32>,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub has_selector_prefix: Option<String>,
    #[serde(default)]
    pub snapshot_ok: bool,
}
