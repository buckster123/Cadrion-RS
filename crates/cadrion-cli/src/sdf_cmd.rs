//! `cadrion sdf` — experimental secondary SDF (H2-9).

use std::path::PathBuf;

use cadrion_sdf::{grid_for_prim, sample_analytic, write_nrrd, write_raw, SdfPrim};
use serde_json::json;

use crate::cli::{Cli, SdfArgs, SdfCmd, SdfPrimArg, SdfSampleArgs};
use crate::output::{emit, ExitCode};

pub fn run(cli: &Cli, args: &SdfArgs) -> ExitCode {
    match &args.cmd {
        SdfCmd::Sample(a) => sample(cli, a),
    }
}

fn sample(cli: &Cli, a: &SdfSampleArgs) -> ExitCode {
    if a.a <= 0.0 || a.b <= 0.0 {
        emit(
            cli.json,
            &json!({"ok": false, "error": "a and b must be > 0"}),
            false,
        );
        return ExitCode::Usage;
    }
    let prim = match a.prim {
        SdfPrimArg::Box => {
            let c = match a.c {
                Some(c) if c > 0.0 => c,
                _ => {
                    emit(
                        cli.json,
                        &json!({"ok": false, "error": "box requires --c > 0 (dz)"}),
                        false,
                    );
                    return ExitCode::Usage;
                }
            };
            SdfPrim::Box {
                dx: a.a,
                dy: a.b,
                dz: c,
            }
        }
        SdfPrimArg::Cylinder => SdfPrim::Cylinder { r: a.a, h: a.b },
    };
    let grid = grid_for_prim(prim, a.res, a.pad);
    let vol = match sample_analytic(prim, &grid) {
        Ok(v) => v,
        Err(e) => {
            emit(
                cli.json,
                &json!({"ok": false, "error": e.to_string()}),
                false,
            );
            return ExitCode::Validation;
        }
    };
    let out = a.out.clone().unwrap_or_else(|| PathBuf::from("sdf_out"));
    let stem = a.stem.clone().unwrap_or_else(|| match a.prim {
        SdfPrimArg::Box => "box".into(),
        SdfPrimArg::Cylinder => "cylinder".into(),
    });
    let (raw, meta) = match write_raw(&vol, &out, &stem) {
        Ok(p) => p,
        Err(e) => {
            emit(
                cli.json,
                &json!({"ok": false, "error": e.to_string()}),
                false,
            );
            return ExitCode::Io;
        }
    };
    let (nrrd, nraw) = match write_nrrd(&vol, &out, &stem) {
        Ok(p) => p,
        Err(e) => {
            emit(
                cli.json,
                &json!({"ok": false, "error": e.to_string()}),
                false,
            );
            return ExitCode::Io;
        }
    };
    emit(
        cli.json,
        &json!({
            "ok": true,
            "secondary": true,
            "note": "experimental SDF — not a modeling path; STEP remains primary",
            "prim": prim,
            "grid": vol.grid,
            "voxel_count": vol.values.len(),
            "raw_f32": raw,
            "meta": meta,
            "nrrd": nrrd,
            "nrrd_raw": nraw,
        }),
        true,
    );
    ExitCode::Ok
}
