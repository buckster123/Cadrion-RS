//! `cadrion engine info|install` — honest kernel inventory (H4-2 / D4).
//!
//! `install` does **not** fetch a tarball. OCCT/truck-brep are compile-time
//! features. If the backend is already in this binary, we say so. Otherwise we
//! refuse with `CADRION-E-ENGINE-MISSING` and a rebuild hint.

use serde_json::{json, Value};

use crate::cli::Cli;
use crate::cli::{EngineBackend, EngineCmd, EngineInstallArgs};
use crate::output::{emit, ExitCode};

const HONESTY: &str =
    "no checksummed engine tarball this slice — install is compile-into-binary or refuse";

pub fn run(cli: &Cli, cmd: &EngineCmd) -> ExitCode {
    match cmd {
        EngineCmd::Info => {
            emit(cli.json, &info_json(), true);
            ExitCode::Ok
        }
        EngineCmd::Install(args) => run_install(cli, args),
    }
}

pub fn info_json() -> Value {
    let occt = cfg!(feature = "occt");
    let truck_brep = cfg!(feature = "truck-brep");
    json!({
        "ok": true,
        "compiled": {
            "mock": true,
            "occt": occt,
            "truck": true,
            "truck_brep": truck_brep,
        },
        "default": crate::kernel_pick::default_kernel_id(),
        "engine_dir": std::env::var("CADRION_ENGINE_DIR").ok(),
        "prebuilt_fetch": false,
        "honesty": HONESTY,
        "install": {
            "occt": component("occt", occt),
            "truck_brep": component("truck-brep", truck_brep),
        },
        "kernels": {
            "truck_parity_eligible": false,
            "truck_note": "seed = analytic CSG; truck-brep = optional H3-6 spike; both NON-PARITY",
            "occt_cone": "unsupported (no silent cylinder stand-in; H3-1)",
        },
    })
}

fn component(id: &str, compiled: bool) -> Value {
    json!({
        "id": id,
        "compiled": compiled,
        "method": method(id),
        "prebuilt_fetch": false,
    })
}

fn method(id: &str) -> &'static str {
    match id {
        "occt" => {
            "CMAKE_POLICY_VERSION_MINIMUM=3.5 cargo build -p cadrion-cli --release --features occt"
        }
        "truck-brep" => "cargo build -p cadrion-cli --release --features truck-brep",
        _ => "compiled into this binary",
    }
}

fn run_install(cli: &Cli, args: &EngineInstallArgs) -> ExitCode {
    let (id, compiled) = match args.backend {
        EngineBackend::Occt => ("occt", cfg!(feature = "occt")),
        EngineBackend::TruckBrep => ("truck-brep", cfg!(feature = "truck-brep")),
    };
    if compiled {
        let v = json!({
            "ok": true,
            "backend": id,
            "already_present": true,
            "prebuilt_fetch": false,
            "honesty": HONESTY,
            "method": method(id),
        });
        emit(cli.json, &v, true);
        return ExitCode::Ok;
    }
    let v = json!({
        "ok": false,
        "backend": id,
        "already_present": false,
        "prebuilt_fetch": false,
        "honesty": HONESTY,
        "diagnostics": [{
            "code": "CADRION-E-ENGINE-MISSING",
            "severity": "error",
            "message": format!("{id} is not compiled into this binary; prebuilt fetch is not shipped"),
            "hint": format!("rebuild with: {}; see docs/occt-binding.md", method(id)),
        }],
    });
    emit(cli.json, &v, false);
    ExitCode::Usage
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn info_always_lists_mock() {
        let v = info_json();
        assert_eq!(v["ok"], true);
        assert_eq!(v["compiled"]["mock"], true);
        assert_eq!(v["prebuilt_fetch"], false);
        assert_eq!(v["compiled"]["occt"], cfg!(feature = "occt"));
    }
}
