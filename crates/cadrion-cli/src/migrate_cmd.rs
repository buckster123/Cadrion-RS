//! `cadrion migrate` — build123d-style Python → Cadrion skeleton (H8).

use std::fs;

use cadrion_lang::migrate_build123d_skeleton;
use serde_json::json;

use crate::cli::{Cli, MigrateArgs};
use crate::output::{emit, ExitCode};

pub fn run(cli: &Cli, args: &MigrateArgs) -> ExitCode {
    if args.source.is_dir() {
        emit(
            cli.json,
            &json!({"ok": false, "diagnostics":[{"code":"CADRION-E-USAGE","message":"directory migrate refused — pass one .py file"}]}),
            false,
        );
        return ExitCode::Usage;
    }
    let text = match fs::read_to_string(&args.source) {
        Ok(t) => t,
        Err(e) => {
            emit(
                cli.json,
                &json!({"ok": false, "diagnostics":[{"code":"CADRION-E-IO","message": e.to_string()}]}),
                false,
            );
            return ExitCode::Io;
        }
    };

    let report = migrate_build123d_skeleton(&text);
    if report.refused || !report.ok {
        emit(
            cli.json,
            &json!({
                "ok": false,
                "refused": report.refused,
                "report": report,
            }),
            false,
        );
        return if report.refused {
            ExitCode::Validation
        } else {
            ExitCode::Eval
        };
    }

    let out = args.out.clone().unwrap_or_else(|| {
        let stem = args
            .source
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("migrated");
        args.source
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join(format!("{stem}.cad.star"))
    });

    if let Err(e) = fs::write(&out, &report.skeleton) {
        emit(
            cli.json,
            &json!({"ok": false, "diagnostics":[{"code":"CADRION-E-IO","message": e.to_string()}]}),
            false,
        );
        return ExitCode::Io;
    }

    emit(
        cli.json,
        &json!({
            "ok": true,
            "out": out,
            "bytes": report.skeleton.len(),
            "report": {
                "notes": report.notes,
                "params": report.params,
                "solids": report.solids,
                "source_hint": report.source_hint,
                "skeleton_preview": report.skeleton.chars().take(400).collect::<String>(),
            },
            "honesty": "best-effort skeleton — not full build123d semantics",
        }),
        true,
    );
    ExitCode::Ok
}
