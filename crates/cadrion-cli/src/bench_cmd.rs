//! `cadrion bench`

use cadrion_bench::{default_parity_root, run_suite};
use serde_json::json;

use crate::cli::{BenchArgs, BenchCmd, Cli};
use crate::output::{emit, ExitCode};

pub fn run(cli: &Cli, args: &BenchArgs) -> ExitCode {
    match &args.cmd {
        BenchCmd::Run(a) => {
            let root = a.parity_root.clone().unwrap_or_else(default_parity_root);
            match run_suite(&root, &a.suite) {
                Ok(report) => {
                    let body = json!({
                        "ok": report.ok,
                        "suite": report.suite,
                        "passed": report.passed,
                        "failed": report.failed,
                        "wall_ms": report.wall_ms,
                        "parts": report.parts,
                        "parity_root": root,
                    });
                    emit(cli.json, &body, report.ok);
                    if report.ok {
                        if !cli.json && !cli.quiet {
                            eprintln!(
                                "parity {}: {}/{} passed in {} ms",
                                report.suite,
                                report.passed,
                                report.passed + report.failed,
                                report.wall_ms
                            );
                        }
                        ExitCode::Ok
                    } else {
                        ExitCode::Validation
                    }
                }
                Err(e) => {
                    let body = json!({
                        "ok": false,
                        "diagnostics": [{
                            "code": "CADRION-E-BENCH",
                            "severity": "error",
                            "message": e,
                        }]
                    });
                    emit(cli.json, &body, false);
                    ExitCode::Usage
                }
            }
        }
    }
}
