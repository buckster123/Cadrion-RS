//! `cadrion mcp` + `cadrion skills`

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::json;

use crate::cli::{Cli, McpArgs, SkillsArgs, SkillsCmd};
use crate::output::{emit, ExitCode};

pub fn run_mcp(_cli: &Cli, _args: &McpArgs) -> ExitCode {
    // stdout is JSON-RPC only — do not emit human/json wrapper
    match cadrion_mcp::run_stdio() {
        Ok(()) => ExitCode::Ok,
        Err(e) => {
            eprintln!("cadrion mcp error: {e}");
            ExitCode::Internal
        }
    }
}

pub fn run_skills(cli: &Cli, args: &SkillsArgs) -> ExitCode {
    match &args.cmd {
        SkillsCmd::Export(a) => {
            if a.all {
                export_all_agents(cli, a.out.clone())
            } else {
                export_skills(cli, a.out.clone(), &a.agent)
            }
        }
    }
}

const AGENTS: &[&str] = &["claude-code", "codex", "hermes"];

fn export_all_agents(cli: &Cli, out: Option<PathBuf>) -> ExitCode {
    let root = out.unwrap_or_else(|| PathBuf::from("dist/skills"));
    let mut packs = Vec::new();
    for agent in AGENTS {
        let dest = root.join(agent).join("cadrion");
        match export_one(&dest, agent) {
            Ok(files) => packs.push(json!({
                "agent": agent,
                "out": dest,
                "files": files,
            })),
            Err(e) => {
                emit(
                    cli.json,
                    &json!({"ok": false, "diagnostics":[{"code":"CADRION-E-SKILLS","message": e}]}),
                    false,
                );
                return ExitCode::Io;
            }
        }
    }
    emit(
        cli.json,
        &json!({"ok": true, "all": true, "root": root, "packs": packs}),
        true,
    );
    ExitCode::Ok
}

fn export_skills(cli: &Cli, out: Option<PathBuf>, agent: &str) -> ExitCode {
    let dest = out.unwrap_or_else(|| PathBuf::from("dist/skills/cadrion"));
    match export_one(&dest, agent) {
        Ok(files) => {
            emit(
                cli.json,
                &json!({"ok": true, "agent": agent, "out": dest, "files": files}),
                true,
            );
            ExitCode::Ok
        }
        Err(e) => {
            emit(
                cli.json,
                &json!({"ok": false, "diagnostics":[{"code":"CADRION-E-SKILLS","message": e}]}),
                false,
            );
            ExitCode::Io
        }
    }
}

fn export_one(dest: &Path, agent: &str) -> Result<Vec<String>, String> {
    let src = skill_source_dir();
    if !src.join("SKILL.md").is_file() {
        return Err(format!("bundled skill missing at {}", src.display()));
    }
    copy_dir(&src, dest)?;
    let note = install_notes(agent);
    fs::write(dest.join("INSTALL.md"), note).map_err(|e| e.to_string())?;
    // agent stamp for tooling
    fs::write(
        dest.join("AGENT.txt"),
        format!("{agent}\nexported_by=cadrion-cli\n"),
    )
    .map_err(|e| e.to_string())?;
    Ok(list_files(dest))
}

fn install_notes(agent: &str) -> String {
    match agent {
        "claude-code" => r#"# Install — Claude Code

Copy this folder to one of:
- Project: `.claude/skills/cadrion/`
- User skills directory (see Claude Code docs)

Reload the session / restart Claude Code.

Prefer MCP: run `cadrion mcp` and register as a stdio MCP server when available.
"#
        .into(),
        "codex" => r#"# Install — Codex

Copy this folder into your Codex skills/plugins path (see current OpenAI Codex docs
for the active skills directory).

Name the skill `cadrion`. Restart Codex after install.

For tool calls, prefer the Cadrion MCP server (`cadrion mcp`) if your Codex build supports MCP.
"#
        .into(),
        "hermes" => r#"# Install — Hermes Agent

Copy to:
- `~/.hermes/skills/cadrion/` (default profile), or
- `~/.hermes/profiles/<name>/skills/cadrion/`

Then `/reload-skills` or restart the session.

Hermes can also run `cadrion mcp` via MCP config (`hermes mcp add` / config.yaml).
"#
        .into(),
        other => format!(
            "# Install notes ({other})\n\nCopy this folder into your agent's skills directory.\n\n\
             Prefer MCP `cadrion mcp` for tool calls when supported.\n"
        ),
    }
}

fn skill_source_dir() -> PathBuf {
    let from_cli = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../skills/cadrion");
    if from_cli.join("SKILL.md").is_file() {
        return from_cli.canonicalize().unwrap_or(from_cli);
    }
    PathBuf::from("skills/cadrion")
}

fn copy_dir(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|e| e.to_string())?;
    for ent in fs::read_dir(src).map_err(|e| e.to_string())? {
        let ent = ent.map_err(|e| e.to_string())?;
        let ty = ent.file_type().map_err(|e| e.to_string())?;
        let to = dst.join(ent.file_name());
        if ty.is_dir() {
            copy_dir(&ent.path(), &to)?;
        } else {
            fs::copy(ent.path(), to).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn list_files(dir: &Path) -> Vec<String> {
    let mut out = Vec::new();
    fn walk2(root: &Path, d: &Path, acc: &mut Vec<String>) {
        if let Ok(rd) = fs::read_dir(d) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() {
                    walk2(root, &p, acc);
                } else if let Ok(rel) = p.strip_prefix(root) {
                    acc.push(rel.display().to_string());
                }
            }
        }
    }
    walk2(dir, dir, &mut out);
    out.sort();
    out
}
