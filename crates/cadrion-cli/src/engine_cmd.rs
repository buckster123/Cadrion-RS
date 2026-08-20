//! `cadrion engine info|install` — honest kernel inventory (H4-2 / H5-5 / D4).
//!
//! Logic lives in `cadrion-mcp` so CLI / MCP / HTTP share one fail-closed story.
//! `install` does **not** fetch a tarball.

use serde_json::json;

use crate::cli::Cli;
use crate::cli::{EngineBackend, EngineCmd, EngineInstallArgs};
use crate::output::{emit, ExitCode};

pub fn run(cli: &Cli, cmd: &EngineCmd) -> ExitCode {
    match cmd {
        EngineCmd::Info => {
            emit(cli.json, &cadrion_mcp::engine_info(), true);
            ExitCode::Ok
        }
        EngineCmd::Install(args) => run_install(cli, args),
    }
}

fn run_install(cli: &Cli, args: &EngineInstallArgs) -> ExitCode {
    let backend = match args.backend {
        EngineBackend::Occt => "occt",
        EngineBackend::TruckBrep => "truck-brep",
    };
    match cadrion_mcp::engine_install(backend) {
        Ok(v) => {
            emit(cli.json, &v, true);
            ExitCode::Ok
        }
        Err(e) => {
            let missing = e.contains("CADRION-E-ENGINE-MISSING");
            let v = if missing {
                json!({
                    "ok": false,
                    "backend": backend,
                    "already_present": false,
                    "prebuilt_fetch": false,
                    "honesty": "no checksummed engine tarball this slice — install is compile-into-binary or refuse",
                    "diagnostics": [{
                        "code": "CADRION-E-ENGINE-MISSING",
                        "severity": "error",
                        "message": e,
                    }],
                })
            } else {
                json!({"ok": false, "diagnostics": [{"code": "CADRION-E-USAGE", "message": e}]})
            };
            emit(cli.json, &v, false);
            ExitCode::Usage
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn info_always_lists_mock() {
        let v = cadrion_mcp::engine_info();
        assert_eq!(v["ok"], true);
        assert_eq!(v["compiled"]["mock"], true);
        assert_eq!(v["prebuilt_fetch"], false);
        assert_eq!(v["compiled"]["occt"], cfg!(feature = "occt"));
    }
}
