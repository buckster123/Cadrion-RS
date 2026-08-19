//! `cadrion harness`

use cadrion_harness::{run_suite, LiveOpts, RunOpts};
use serde_json::json;

use crate::cli::{Cli, HarnessArgs, HarnessCmd};
use crate::output::{emit, ExitCode};

pub fn run(cli: &Cli, args: &HarnessArgs) -> ExitCode {
    match &args.cmd {
        HarnessCmd::Run(a) => {
            let live = a.cmd.as_ref().map(|cmd| LiveOpts {
                cmd: cmd.clone(),
                timeout_secs: a.timeout,
                part_rel: "part.cad.star".into(),
                snapshot: !a.no_snapshot,
            });
            let opts = RunOpts {
                tasks_root: a.tasks_root.clone(),
                live,
            };
            match run_suite(&a.suite, &opts) {
                Ok(card) => {
                    let body = json!({
                        "ok": card.meets_target,
                        "scorecard": card,
                    });
                    emit(cli.json, &body, card.meets_target);
                    if !cli.json && !cli.quiet {
                        eprintln!(
                            "harness {} [{}]: {:.1}/10 (target ≥ {:.0}) — {}/{} tasks, median_loops={:.1}, {} ms",
                            card.suite,
                            card.mode,
                            card.score_over_10,
                            card.target,
                            card.passed,
                            card.total,
                            card.median_loops,
                            card.wall_ms
                        );
                        for t in &card.tasks {
                            let mark = if t.ok { "ok" } else { "FAIL" };
                            eprintln!(
                                "  [{mark}] {} loops={}/{} — {}",
                                t.id, t.loops_used, t.max_loops, t.detail
                            );
                        }
                    }
                    if card.meets_target {
                        ExitCode::Ok
                    } else {
                        ExitCode::Validation
                    }
                }
                Err(e) => {
                    emit(
                        cli.json,
                        &json!({
                            "ok": false,
                            "diagnostics": [{
                                "code": "CADRION-E-HARNESS",
                                "severity": "error",
                                "message": e,
                            }]
                        }),
                        false,
                    );
                    ExitCode::Usage
                }
            }
        }
    }
}
