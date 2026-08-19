//! Exit codes and JSON/human emission.

use serde::Serialize;
use std::io::Write;

/// Stable process exit codes (design.md).
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ExitCode {
    Ok = 0,
    Usage = 2,
    Eval = 3,
    Validation = 4,
    Kernel = 5,
    Io = 6,
    Network = 7,
    Safety = 8,
    Internal = 9,
}

pub fn emit(json: bool, value: &impl Serialize, ok: bool) {
    if json {
        match serde_json::to_string_pretty(value) {
            Ok(s) => {
                let _ = writeln!(std::io::stdout(), "{s}");
            }
            Err(e) => {
                let _ = writeln!(
                    std::io::stderr(),
                    "{{\"ok\":false,\"diagnostics\":[{{\"code\":\"CADRION-E-INTERNAL\",\"message\":\"json serialize: {e}\"}}]}}"
                );
            }
        }
    } else if ok {
        // Human: compact one-liner summary from JSON fields when possible
        if let Ok(v) = serde_json::to_value(value) {
            human_ok(&v);
        }
    } else if let Ok(v) = serde_json::to_value(value) {
        human_err(&v);
    }
}

fn human_ok(v: &serde_json::Value) {
    if let Some(arts) = v.get("artifacts").and_then(|a| a.as_array()) {
        for a in arts {
            if let Some(p) = a.get("path").and_then(|p| p.as_str()) {
                let kind = a.get("kind").and_then(|k| k.as_str()).unwrap_or("?");
                let _ = writeln!(std::io::stdout(), "wrote {kind}: {p}");
            }
        }
    }
    if let Some(f) = v.get("facts") {
        if let Some(vol) = f.get("volume_mm3") {
            let _ = writeln!(std::io::stdout(), "volume_mm3: {vol}");
        }
    }
    if let Some(cache) = v.get("cache").and_then(|c| c.get("hit")) {
        if cache.as_bool() == Some(true) {
            let _ = writeln!(std::io::stdout(), "cache: hit");
        }
    }
    if let Some(refs) = v.get("refs").and_then(|r| r.as_array()) {
        let _ = writeln!(std::io::stdout(), "{} refs", refs.len());
        for r in refs.iter().take(12) {
            if let Some(s) = r.get("selector").and_then(|s| s.as_str()) {
                let kind = r.get("kind").and_then(|k| k.as_str()).unwrap_or("");
                let _ = writeln!(std::io::stdout(), "  {s}  ({kind})");
            }
        }
        if refs.len() > 12 {
            let _ = writeln!(std::io::stdout(), "  … {} more", refs.len() - 12);
        }
    }
    if let Some(val) = v.get("value") {
        let unit = v.get("unit").and_then(|u| u.as_str()).unwrap_or("");
        let _ = writeln!(std::io::stdout(), "{val} {unit}");
        if let Some(c) = v.get("construction").and_then(|c| c.as_str()) {
            let _ = writeln!(std::io::stdout(), "  ({c})");
        }
    }
    if let Some(p) = v.get("path").and_then(|p| p.as_str()) {
        let _ = writeln!(std::io::stdout(), "{p}");
    }
}

fn human_err(v: &serde_json::Value) {
    if let Some(diags) = v.get("diagnostics").and_then(|d| d.as_array()) {
        for d in diags {
            let code = d.get("code").and_then(|c| c.as_str()).unwrap_or("ERROR");
            let msg = d.get("message").and_then(|m| m.as_str()).unwrap_or("");
            let _ = writeln!(std::io::stderr(), "{code}: {msg}");
            if let Some(h) = d.get("hint").and_then(|h| h.as_str()) {
                let _ = writeln!(std::io::stderr(), "  hint: {h}");
            }
        }
    } else {
        let _ = writeln!(std::io::stderr(), "{v}");
    }
}
