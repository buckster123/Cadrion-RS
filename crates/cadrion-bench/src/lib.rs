//! Deterministic parity suite — reference parts + expect.json assertions.

#![deny(unsafe_code)]

mod expect;
mod runner;

pub use expect::{Expect, FindFace, MeasureExpect};
pub use runner::{
    default_parity_root, run_part, run_part_with, run_suite, run_suite_with, KernelKind,
    PartResult, RunOpts, SuiteReport,
};

/// Crate version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Suite id for parts 1–4 (M1 exit) — mock kernel + expect.json.
pub const SUITE_PARTS_1_4: &str = "parts1-4";

/// Same parts under OCCT with expect.occt.json (optional local / feature).
pub const SUITE_PARTS_1_4_OCCT: &str = "parts1-4-occt";

/// Parts 5–10 (stdlib rotate/translate era).
pub const SUITE_PARTS_5_10: &str = "parts5-10";

/// Parts 5–10 under OCCT + expect.occt.json.
pub const SUITE_PARTS_5_10_OCCT: &str = "parts5-10-occt";

/// Full Parity-10 mock suite.
pub const SUITE_PARTS_1_10: &str = "parts1-10";

/// OCCT-only fillet/chamfer parts (H4) — no mock expect.
pub const SUITE_FILLET_OCCT: &str = "fillet-occt";
