//! `cadrion parts search|fetch|lock` — local filesystem catalog (H6-1).

use std::path::{Path, PathBuf};

use cadrion_parts::{
    upsert_lock_entry, verify_lock_entry, LocalFsProvider, PartProvider, PartsLockEntry,
};
use serde_json::json;

use crate::cli::{Cli, PartsArgs, PartsCmd, PartsIdArgs, PartsLockCliArgs, PartsSearchArgs};
use crate::output::{emit, ExitCode};

pub fn run(cli: &Cli, args: &PartsArgs) -> ExitCode {
    match &args.cmd {
        PartsCmd::Search(a) => search(cli, a),
        PartsCmd::Fetch(a) | PartsCmd::Show(a) => fetch(cli, a),
        PartsCmd::Lock(a) => lock(cli, a),
    }
}

fn project_root(cli: &Cli) -> PathBuf {
    cli.project
        .clone()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
}

fn parts_root(cli: &Cli, root: &Option<PathBuf>) -> PathBuf {
    root.clone()
        .unwrap_or_else(|| project_root(cli).join("parts"))
}

fn rel_to_project(project: &Path, file: &Path) -> String {
    let file_c = file.canonicalize().unwrap_or_else(|_| file.to_path_buf());
    let proj_c = project
        .canonicalize()
        .unwrap_or_else(|_| project.to_path_buf());
    file_c
        .strip_prefix(&proj_c)
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|_| file.to_string_lossy().replace('\\', "/"))
}

fn search(cli: &Cli, a: &PartsSearchArgs) -> ExitCode {
    let root = parts_root(cli, &a.root);
    let prov = LocalFsProvider::new(&root);
    match prov.search(a.query.as_deref().unwrap_or("")) {
        Ok(results) => {
            emit(
                cli.json,
                &json!({
                    "ok": true,
                    "provider": prov.id(),
                    "parts_root": root,
                    "results": results,
                    "storefront": false,
                }),
                true,
            );
            ExitCode::Ok
        }
        Err(e) => {
            emit(
                cli.json,
                &json!({"ok": false, "diagnostics": [{"code": "CADRION-E-IO", "message": e.to_string()}]}),
                false,
            );
            ExitCode::Io
        }
    }
}

fn fetch(cli: &Cli, a: &PartsIdArgs) -> ExitCode {
    let root = parts_root(cli, &a.root);
    let prov = LocalFsProvider::new(&root);
    match prov.fetch(&a.id) {
        Ok(meta) => {
            emit(
                cli.json,
                &json!({
                    "ok": true,
                    "provider": prov.id(),
                    "meta": meta,
                    "downloaded": false,
                    "storefront": false,
                }),
                true,
            );
            ExitCode::Ok
        }
        Err(cadrion_parts::ProviderError::NotFound(id)) => {
            emit(
                cli.json,
                &json!({
                    "ok": false,
                    "diagnostics": [{
                        "code": "CADRION-E-PARTS-NOT-FOUND",
                        "message": format!("no local STEP for {id}"),
                        "hint": "catalog is a directory of .step/.stp files; id = stem"
                    }],
                    "downloaded": false,
                    "storefront": false,
                }),
                false,
            );
            ExitCode::Validation
        }
        Err(e) => {
            emit(
                cli.json,
                &json!({"ok": false, "diagnostics": [{"code": "CADRION-E-IO", "message": e.to_string()}]}),
                false,
            );
            ExitCode::Io
        }
    }
}

fn lock(cli: &Cli, a: &PartsLockCliArgs) -> ExitCode {
    let project = project_root(cli);
    let root = parts_root(cli, &a.root);
    let lock_path = a.lock.clone().unwrap_or_else(|| project.join("parts.lock"));
    let key = a.key.clone().unwrap_or_else(|| a.id.clone());
    let prov = LocalFsProvider::new(&root);
    let meta = match prov.fetch(&a.id) {
        Ok(m) => m,
        Err(cadrion_parts::ProviderError::NotFound(id)) => {
            emit(
                cli.json,
                &json!({
                    "ok": false,
                    "diagnostics": [{
                        "code": "CADRION-E-PARTS-NOT-FOUND",
                        "message": format!("cannot lock missing part {id}"),
                    }],
                    "storefront": false,
                }),
                false,
            );
            return ExitCode::Validation;
        }
        Err(e) => {
            emit(
                cli.json,
                &json!({"ok": false, "diagnostics": [{"code": "CADRION-E-IO", "message": e.to_string()}]}),
                false,
            );
            return ExitCode::Io;
        }
    };
    let entry = PartsLockEntry {
        provider: prov.id().into(),
        id: meta.id.clone(),
        version: None,
        sha256: meta.sha256.clone(),
        path: rel_to_project(&project, &meta.path),
        license: meta.license.clone(),
    };
    let written = match upsert_lock_entry(&lock_path, &key, entry.clone()) {
        Ok(l) => l,
        Err(e) => {
            emit(
                cli.json,
                &json!({"ok": false, "diagnostics": [{"code": "CADRION-E-PARTS-LOCK", "message": e.to_string()}]}),
                false,
            );
            return ExitCode::Io;
        }
    };
    if let Err(e) = verify_lock_entry(&written, &key, &project) {
        emit(
            cli.json,
            &json!({"ok": false, "diagnostics": [{"code": "CADRION-E-PARTS-LOCK", "message": e.to_string()}]}),
            false,
        );
        return ExitCode::Validation;
    }
    emit(
        cli.json,
        &json!({
            "ok": true,
            "key": key,
            "entry": entry,
            "lock": lock_path,
            "verified": true,
            "storefront": false,
        }),
        true,
    );
    ExitCode::Ok
}
