//! Agent-loop harness — scripted cycles + live `--cmd` driver + scorecard.

#![deny(unsafe_code)]

mod live;
mod runner;
mod scenario;
mod score;

pub use live::LiveOpts;
pub use runner::{default_tasks_root, run_suite, run_task, RunOpts};
pub use scenario::{AssertSpec, Step, Task};
pub use score::{Scorecard, TaskResult, SUITE_AGENT10};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
