//! MCP resources: `cadrion://doc/**` and `cadrion://artifact/**`.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::tools::ToolError;

/// Built-in documentation resources (content embedded / loaded from repo docs when present).
pub fn list_resources(project_root: &Path) -> Value {
    let mut resources = vec![
        res(
            "cadrion://doc/status",
            "Cadrion live status",
            "text/markdown",
        ),
        res("cadrion://doc/stdlib", "Stdlib depth (H2)", "text/markdown"),
        res(
            "cadrion://doc/viewer",
            "Viewer gcode/robot (H5)",
            "text/markdown",
        ),
        res(
            "cadrion://doc/slicer-dfm",
            "Slicer gates + DFM profiles (H6)",
            "text/markdown",
        ),
        res(
            "cadrion://doc/fillet",
            "Fillet/chamfer doctrine (H4)",
            "text/markdown",
        ),
        res(
            "cadrion://doc/write-source-policy",
            "write_source / read_source policy (H7)",
            "text/markdown",
        ),
        res(
            "cadrion://artifact/index",
            "Index of local IR/snap/gcode artifacts under project",
            "application/json",
        ),
    ];

    // Dynamic artifact files (capped).
    for (uri, name) in scan_artifacts(project_root).into_iter().take(40) {
        resources.push(json!({
            "uri": uri,
            "name": name,
            "mimeType": guess_mime(&name),
        }));
    }

    json!(resources)
}

fn res(uri: &str, name: &str, mime: &str) -> Value {
    json!({ "uri": uri, "name": name, "mimeType": mime })
}

fn guess_mime(name: &str) -> &'static str {
    if name.ends_with(".json") {
        "application/json"
    } else if name.ends_with(".gcode") || name.ends_with(".gco") {
        "text/x-gcode"
    } else if name.ends_with(".md") {
        "text/markdown"
    } else if name.ends_with(".star") {
        "text/plain"
    } else {
        "application/octet-stream"
    }
}

fn strip_uri<'a>(uri: &'a str, rest: &str) -> Option<&'a str> {
    let neu = format!("cadrion://{rest}");
    let old = format!("cadre://{rest}");
    uri.strip_prefix(&neu).or_else(|| uri.strip_prefix(&old))
}

/// Read one resource by URI. `cadre://` is still accepted (OQ-1 rename alias).
pub fn read_resource(project_root: &Path, uri: &str) -> Result<Value, ToolError> {
    if let Some(rest) = strip_uri(uri, "doc/") {
        let text = doc_body(rest, project_root)?;
        return Ok(json!({
            "contents": [{
                "uri": uri,
                "mimeType": "text/markdown",
                "text": text,
            }]
        }));
    }
    if uri == "cadrion://artifact/index" || uri == "cadre://artifact/index" {
        let idx: Vec<_> = scan_artifacts(project_root)
            .into_iter()
            .map(|(u, n)| json!({"uri": u, "name": n}))
            .collect();
        return Ok(json!({
            "contents": [{
                "uri": uri,
                "mimeType": "application/json",
                "text": serde_json::to_string_pretty(&idx).unwrap(),
            }]
        }));
    }
    if let Some(rel) = strip_uri(uri, "artifact/file/") {
        let path = safe_join(project_root, rel)?;
        let text = fs::read_to_string(&path).map_err(|e| ToolError::msg(e.to_string()))?;
        // Cap huge files
        let text = if text.len() > 200_000 {
            format!(
                "{}\n\n… truncated {} bytes total …",
                &text[..200_000],
                text.len()
            )
        } else {
            text
        };
        return Ok(json!({
            "contents": [{
                "uri": uri,
                "mimeType": guess_mime(rel),
                "text": text,
            }]
        }));
    }
    Err(ToolError::msg(format!("unknown resource uri: {uri}")))
}

fn safe_join(root: &Path, rel: &str) -> Result<PathBuf, ToolError> {
    if rel.contains("..") || rel.starts_with('/') || rel.starts_with('\\') {
        return Err(ToolError::msg("refusing path escape in artifact uri"));
    }
    let p = root.join(rel);
    let canon = p.canonicalize().unwrap_or(p.clone());
    let root_c = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    if !canon.starts_with(&root_c) {
        return Err(ToolError::msg("artifact outside project root"));
    }
    Ok(canon)
}

fn scan_artifacts(root: &Path) -> Vec<(String, String)> {
    let mut out = Vec::new();
    fn walk(dir: &Path, root: &Path, out: &mut Vec<(String, String)>, depth: u32) {
        if depth > 4 || out.len() > 80 {
            return;
        }
        let Ok(rd) = fs::read_dir(dir) else {
            return;
        };
        for e in rd.flatten() {
            let p = e.path();
            let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if name.starts_with('.') && name != ".cadrion" {
                continue;
            }
            if p.is_dir() {
                // shallow walk: always recurse a couple levels into project-ish dirs
                walk(&p, root, out, depth + 1);
                continue;
            }
            let rel = p.strip_prefix(root).unwrap_or(&p);
            let rel_s = rel.to_string_lossy().replace('\\', "/");
            if rel_s.ends_with(".ir.json")
                || rel_s.ends_with("manifest.json")
                || rel_s.ends_with(".gcode")
                || rel_s.ends_with(".cad.star")
            {
                out.push((format!("cadrion://artifact/file/{rel_s}"), rel_s.clone()));
            }
        }
    }
    walk(root, root, &mut out, 0);
    out.sort_by(|a, b| a.1.cmp(&b.1));
    out
}

fn doc_body(id: &str, project_root: &Path) -> Result<String, ToolError> {
    let file = match id {
        "status" => Some("docs/STATUS.md"),
        "stdlib" => Some("docs/STDLIB_DEPTH.md"),
        "viewer" => Some("docs/VIEWER.md"),
        "slicer-dfm" => Some("docs/SLICER_DFM.md"),
        "fillet" => Some("docs/FILLET_CHAMFER.md"),
        "write-source-policy" => None,
        other => return Err(ToolError::msg(format!("unknown doc id: {other}"))),
    };
    if id == "write-source-policy" {
        return Ok(WRITE_SOURCE_POLICY_MD.into());
    }
    let rel = file.unwrap();
    // Prefer live file from project / workspace
    for base in [
        project_root.to_path_buf(),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
    ] {
        let p = base.join(rel);
        if p.is_file() {
            return fs::read_to_string(p).map_err(|e| ToolError::msg(e.to_string()));
        }
    }
    Ok(format!(
        "# {id}\n\n_(doc file `{rel}` not found on disk; open the Cadrion-RS repo.)_\n"
    ))
}

const WRITE_SOURCE_POLICY_MD: &str = r#"# write_source / read_source policy (H7)

## Decision (2026-08-05)

| Transport | `write_source` default | Override |
|-----------|------------------------|----------|
| **stdio** (`cadrion mcp`) | **OFF** | `CADRION_MCP_WRITE_SOURCE=1` |
| **HTTP** (`cadrion serve mcp`) | **ON** | `CADRION_MCP_WRITE_SOURCE=0` to disable |

`read_source` remains available on both (agents still need to inspect files they already own).

## Why

- Local stdio agents (Claude Code, Hermes) already have first-class filesystem tools — duplicate write paths invite confusion and accidental overwrites.
- HTTP / remote agents often **cannot** touch the host FS except through MCP; write_source is load-bearing there.
- Aligns with CHARTER OQ-5 resolution.

## Resources

- `cadrion://doc/**` — bundled doctrine markdown
- `cadrion://artifact/index` — scan of `.cad.star` / `.ir.json` / snaps / gcode under project root
- `cadrion://artifact/file/<relpath>` — read one artifact (path-escape refused)
"#;
