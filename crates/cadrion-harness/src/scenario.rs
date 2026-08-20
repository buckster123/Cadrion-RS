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

    /// Every `assert` in scripted loops, plus optional `success`.
    pub fn all_asserts(&self) -> Vec<AssertSpec> {
        let mut out = Vec::new();
        for loop_steps in &self.loops {
            for step in loop_steps {
                if let Step::Assert {
                    volume_min,
                    volume_max,
                    faces_min,
                    label,
                    has_selector_prefix,
                    snapshot_ok,
                } = step
                {
                    out.push(AssertSpec {
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
        if let Some(s) = &self.success {
            out.push(s.clone());
        }
        out
    }

    /// Live agents only see `prompt`. It must name every asserted label / selector
    /// and give a size or volume band when volume is checked (H5-1).
    pub fn prompt_covers_asserts(&self) -> Result<(), String> {
        let p = &self.prompt;
        for a in self.all_asserts() {
            if let Some(lab) = &a.label {
                if !p.contains(lab) {
                    return Err(format!("{}: prompt missing label {lab:?}", self.id));
                }
            }
            if let Some(pref) = &a.has_selector_prefix {
                if !p.contains(pref) {
                    return Err(format!("{}: prompt missing selector {pref}", self.id));
                }
            }
        }
        let needs_vol = self
            .all_asserts()
            .iter()
            .any(|a| a.volume_min.is_some() || a.volume_max.is_some());
        if needs_vol {
            let nums = numeric_tokens(p);
            let has_k = p.contains('k') || p.contains('K');
            if nums.len() < 2 && !has_k {
                return Err(format!(
                    "{}: volume asserted but prompt has no size/volume numbers",
                    self.id
                ));
            }
        }
        Ok(())
    }
}

fn numeric_tokens(s: &str) -> Vec<f64> {
    let mut out = Vec::new();
    let mut buf = String::new();
    for c in s.chars() {
        if c.is_ascii_digit() || (c == '.' && !buf.is_empty() && !buf.contains('.')) {
            buf.push(c);
        } else if !buf.is_empty() {
            if let Ok(n) = buf.parse::<f64>() {
                out.push(n);
            }
            buf.clear();
        }
    }
    if !buf.is_empty() {
        if let Ok(n) = buf.parse::<f64>() {
            out.push(n);
        }
    }
    out
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

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_step(label: Option<&str>, volume_min: Option<f64>) -> Step {
        Step::Assert {
            volume_min,
            volume_max: volume_min.map(|v| v + 200.0),
            faces_min: None,
            label: label.map(str::to_string),
            has_selector_prefix: None,
            snapshot_ok: false,
        }
    }

    #[test]
    fn prompt_must_name_label_and_size() {
        let miss = Task {
            id: "x".into(),
            prompt: "make a box".into(),
            max_loops: 1,
            loops: vec![vec![assert_step(Some("block"), Some(8000.0))]],
            success: None,
        };
        assert!(miss.prompt_covers_asserts().is_err());

        let ok = Task {
            id: "x".into(),
            prompt: "40×20×10 mm block labeled block".into(),
            max_loops: 1,
            loops: vec![vec![assert_step(Some("block"), Some(8000.0))]],
            success: None,
        };
        ok.prompt_covers_asserts().expect("fair prompt");
    }
}
