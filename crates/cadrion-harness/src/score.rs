//! Scorecard aggregation.

use serde::{Deserialize, Serialize};

/// Suite id for the 10 scripted agent tasks.
pub const SUITE_AGENT10: &str = "agent10";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub id: String,
    pub ok: bool,
    pub loops_used: u32,
    pub max_loops: u32,
    pub wall_ms: u64,
    pub detail: String,
    pub prompt: String,
    /// `scripted` or `live`.
    #[serde(default = "default_mode")]
    pub mode: String,
}

fn default_mode() -> String {
    "scripted".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scorecard {
    pub suite: String,
    /// `scripted` or `live`.
    #[serde(default = "default_mode")]
    pub mode: String,
    pub ok: bool,
    pub passed: u32,
    pub failed: u32,
    pub total: u32,
    pub score_over_10: f64,
    pub target: f64,
    pub meets_target: bool,
    pub median_loops: f64,
    pub wall_ms: u64,
    pub tasks: Vec<TaskResult>,
    /// Live driver command (e.g. `@oracle` or shell). Empty for scripted.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub cmd: String,
    /// Model id when known (oracle / external LLM). Empty if unknown.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub model_id: String,
    /// ISO-8601 UTC date of the run when set by publisher (optional).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub run_date: String,
    /// Free-form honesty notes (frontier not run, backends down, …).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

impl Scorecard {
    pub fn from_tasks(suite: &str, mode: &str, tasks: Vec<TaskResult>, wall_ms: u64) -> Self {
        let total = tasks.len() as u32;
        let passed = tasks.iter().filter(|t| t.ok).count() as u32;
        let failed = total.saturating_sub(passed);
        let score_over_10 = if total == 0 {
            0.0
        } else {
            (passed as f64) * 10.0 / (total as f64)
        };
        let target = 6.0;
        let mut loops: Vec<u32> = tasks
            .iter()
            .filter(|t| t.ok)
            .map(|t| t.loops_used)
            .collect();
        loops.sort_unstable();
        let median_loops = if loops.is_empty() {
            0.0
        } else if loops.len() % 2 == 1 {
            loops[loops.len() / 2] as f64
        } else {
            let a = loops[loops.len() / 2 - 1] as f64;
            let b = loops[loops.len() / 2] as f64;
            (a + b) / 2.0
        };
        Self {
            suite: suite.into(),
            mode: mode.into(),
            ok: failed == 0,
            passed,
            failed,
            total,
            score_over_10,
            target,
            meets_target: score_over_10 + 1e-9 >= target,
            median_loops,
            wall_ms,
            tasks,
            cmd: String::new(),
            model_id: String::new(),
            run_date: String::new(),
            notes: Vec::new(),
        }
    }

    /// Attach live-run provenance for publication (H2-4).
    pub fn with_provenance(mut self, cmd: impl Into<String>, model_id: impl Into<String>) -> Self {
        self.cmd = cmd.into();
        self.model_id = model_id.into();
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }
}
