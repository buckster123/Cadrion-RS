//! Kernel inventory for MCP/HTTP (H5-5 / D4).
//!
//! `install` does **not** fetch a tarball. OCCT/truck-brep are compile-time
//! features forwarded from `cadrion-cli`. Missing backend →
//! `CADRION-E-ENGINE-MISSING`.

use serde_json::{json, Value};

const HONESTY: &str =
    "no checksummed engine tarball this slice — install is compile-into-binary or refuse";

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
        "default": default_kernel_id(),
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

pub fn default_kernel_id() -> &'static str {
    if cfg!(feature = "occt") {
        "occt"
    } else {
        "mock"
    }
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

/// Fail-closed install. `Ok` = already compiled in. `Err` includes
/// `CADRION-E-ENGINE-MISSING` and is not a fetch.
pub fn install(backend: &str) -> Result<Value, String> {
    let (id, compiled) = match backend {
        "occt" => ("occt", cfg!(feature = "occt")),
        "truck-brep" | "truck_brep" => ("truck-brep", cfg!(feature = "truck-brep")),
        other => {
            return Err(format!(
                "unknown backend {other:?} (occt|truck-brep); prebuilt_fetch=false"
            ))
        }
    };
    if compiled {
        return Ok(json!({
            "ok": true,
            "backend": id,
            "already_present": true,
            "prebuilt_fetch": false,
            "honesty": HONESTY,
            "method": method(id),
        }));
    }
    Err(format!(
        "CADRION-E-ENGINE-MISSING: {id} is not compiled into this binary; prebuilt fetch is not shipped. rebuild with: {}; see docs/occt-binding.md",
        method(id)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn info_always_lists_mock_and_never_fetches() {
        let v = info_json();
        assert_eq!(v["ok"], true);
        assert_eq!(v["compiled"]["mock"], true);
        assert_eq!(v["prebuilt_fetch"], false);
        assert_eq!(v["compiled"]["occt"], cfg!(feature = "occt"));
        assert_eq!(v["compiled"]["truck_brep"], cfg!(feature = "truck-brep"));
    }

    #[test]
    fn install_occt_is_fail_closed_in_default_ci() {
        match install("occt") {
            Ok(v) => {
                assert_eq!(v["already_present"], true);
                assert_eq!(v["prebuilt_fetch"], false);
            }
            Err(e) => assert!(e.contains("CADRION-E-ENGINE-MISSING"), "{e}"),
        }
    }
}
