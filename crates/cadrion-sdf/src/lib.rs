//! Experimental **secondary** signed-distance field sampling (H2-9).
//!
//! **Not a modeling path.** STEP / B-rep remains primary. This crate samples
//! analytic box/cylinder SDFs (and optional mesh distance later) for research /
//! viz / ML — never default build output.

#![deny(unsafe_code)]

use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// Crate version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Analytic primitive for SDF evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum SdfPrim {
    /// Axis-aligned box centered at origin, full extents (dx,dy,dz) mm.
    Box { dx: f64, dy: f64, dz: f64 },
    /// Cylinder along +Z, radius r, height h, centered at origin.
    Cylinder { r: f64, h: f64 },
}

/// Axis-aligned sample grid in mm.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GridSpec {
    pub origin_mm: [f64; 3],
    pub spacing_mm: [f64; 3],
    pub dims: [usize; 3],
}

/// Dense SDF volume (row-major X fastest, then Y, then Z).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SdfVolume {
    pub grid: GridSpec,
    /// Signed distance mm (negative = inside).
    pub values: Vec<f32>,
    pub prim: SdfPrim,
    pub notes: Vec<String>,
}

/// Sample analytic SDF on a regular grid.
pub fn sample_analytic(prim: SdfPrim, grid: &GridSpec) -> Result<SdfVolume, SdfError> {
    let [nx, ny, nz] = grid.dims;
    if nx == 0 || ny == 0 || nz == 0 {
        return Err(SdfError::Msg("grid dims must be > 0".into()));
    }
    if nx * ny * nz > 16_000_000 {
        return Err(SdfError::Msg("grid too large (>16M voxels)".into()));
    }
    let mut values = Vec::with_capacity(nx * ny * nz);
    let [ox, oy, oz] = grid.origin_mm;
    let [sx, sy, sz] = grid.spacing_mm;
    for iz in 0..nz {
        for iy in 0..ny {
            for ix in 0..nx {
                let p = [
                    ox + (ix as f64) * sx,
                    oy + (iy as f64) * sy,
                    oz + (iz as f64) * sz,
                ];
                values.push(eval_sdf(prim, p) as f32);
            }
        }
    }
    Ok(SdfVolume {
        grid: grid.clone(),
        values,
        prim,
        notes: vec![
            "H2-9 experimental secondary SDF — not a modeling path".into(),
            "STEP/B-rep remains primary (CHARTER)".into(),
            "analytic box/cylinder only in this slice".into(),
        ],
    })
}

/// Signed distance at point (mm). Negative inside.
pub fn eval_sdf(prim: SdfPrim, p: [f64; 3]) -> f64 {
    match prim {
        SdfPrim::Box { dx, dy, dz } => sd_box(p, [dx * 0.5, dy * 0.5, dz * 0.5]),
        SdfPrim::Cylinder { r, h } => sd_cylinder_z(p, r, h * 0.5),
    }
}

/// Default grid that pads a box/cyl by `pad_mm` with `res` samples along longest axis.
pub fn grid_for_prim(prim: SdfPrim, res: usize, pad_mm: f64) -> GridSpec {
    let res = res.max(4);
    let (hx, hy, hz) = match prim {
        SdfPrim::Box { dx, dy, dz } => (dx * 0.5, dy * 0.5, dz * 0.5),
        SdfPrim::Cylinder { r, h } => (r, r, h * 0.5),
    };
    let half = [hx + pad_mm, hy + pad_mm, hz + pad_mm];
    let span = [half[0] * 2.0, half[1] * 2.0, half[2] * 2.0];
    let longest = span[0].max(span[1]).max(span[2]).max(1e-6);
    let spacing = longest / (res as f64 - 1.0);
    let dims = [
        ((span[0] / spacing).round() as usize).max(2) + 1,
        ((span[1] / spacing).round() as usize).max(2) + 1,
        ((span[2] / spacing).round() as usize).max(2) + 1,
    ];
    GridSpec {
        origin_mm: [-half[0], -half[1], -half[2]],
        spacing_mm: [spacing, spacing, spacing],
        dims,
    }
}

/// Write little-endian f32 raw + sidecar JSON meta. Returns paths written.
pub fn write_raw(
    vol: &SdfVolume,
    dir: &Path,
    stem: &str,
) -> Result<(std::path::PathBuf, std::path::PathBuf), SdfError> {
    fs::create_dir_all(dir).map_err(|e| SdfError::Io(e.to_string()))?;
    let raw_path = dir.join(format!("{stem}.sdf.f32"));
    let meta_path = dir.join(format!("{stem}.sdf.json"));
    let mut f = File::create(&raw_path).map_err(|e| SdfError::Io(e.to_string()))?;
    for v in &vol.values {
        f.write_all(&v.to_le_bytes())
            .map_err(|e| SdfError::Io(e.to_string()))?;
    }
    let meta = serde_json::json!({
        "schema": "cadrion.sdf_volume",
        "version": 1,
        "format": "raw_f32_le",
        "endian": "little",
        "layout": "x_fastest",
        "unit": "mm",
        "grid": vol.grid,
        "prim": vol.prim,
        "voxel_count": vol.values.len(),
        "notes": vol.notes,
        "raw": raw_path.file_name().and_then(|s| s.to_str()),
    });
    fs::write(
        &meta_path,
        serde_json::to_string_pretty(&meta).map_err(|e| SdfError::Io(e.to_string()))?,
    )
    .map_err(|e| SdfError::Io(e.to_string()))?;
    Ok((raw_path, meta_path))
}

/// Write a minimal NRRD (detached data) + `.raw` for the same volume.
pub fn write_nrrd(
    vol: &SdfVolume,
    dir: &Path,
    stem: &str,
) -> Result<(std::path::PathBuf, std::path::PathBuf), SdfError> {
    fs::create_dir_all(dir).map_err(|e| SdfError::Io(e.to_string()))?;
    let raw_name = format!("{stem}.raw");
    let raw_path = dir.join(&raw_name);
    let nrrd_path = dir.join(format!("{stem}.nrrd"));
    let mut f = File::create(&raw_path).map_err(|e| SdfError::Io(e.to_string()))?;
    for v in &vol.values {
        f.write_all(&v.to_le_bytes())
            .map_err(|e| SdfError::Io(e.to_string()))?;
    }
    let [nx, ny, nz] = vol.grid.dims;
    let [sx, sy, sz] = vol.grid.spacing_mm;
    let [ox, oy, oz] = vol.grid.origin_mm;
    // NRRD: sizes are X Y Z; space directions diagonal spacing
    let header = format!(
        "NRRD0004\n\
         # cadrion-sdf H2-9 experimental — not a modeling path\n\
         type: float\n\
         dimension: 3\n\
         sizes: {nx} {ny} {nz}\n\
         encoding: raw\n\
         endian: little\n\
         space dimension: 3\n\
         space units: \"mm\" \"mm\" \"mm\"\n\
         space directions: ({sx},0,0) (0,{sy},0) (0,0,{sz})\n\
         space origin: ({ox},{oy},{oz})\n\
         data file: {raw_name}\n\
         "
    );
    fs::write(&nrrd_path, header).map_err(|e| SdfError::Io(e.to_string()))?;
    Ok((nrrd_path, raw_path))
}

// --- analytic SDFs (Inigo Quilez style, mm) ---

fn sd_box(p: [f64; 3], b: [f64; 3]) -> f64 {
    let q = [p[0].abs() - b[0], p[1].abs() - b[1], p[2].abs() - b[2]];
    let outside = [q[0].max(0.0), q[1].max(0.0), q[2].max(0.0)];
    let out_len =
        (outside[0] * outside[0] + outside[1] * outside[1] + outside[2] * outside[2]).sqrt();
    let inside = q[0].max(q[1]).max(q[2]).min(0.0);
    out_len + inside
}

fn sd_cylinder_z(p: [f64; 3], r: f64, half_h: f64) -> f64 {
    let d_xy = (p[0] * p[0] + p[1] * p[1]).sqrt() - r;
    let d_z = p[2].abs() - half_h;
    let outside = [d_xy.max(0.0), d_z.max(0.0)];
    let out_len = (outside[0] * outside[0] + outside[1] * outside[1]).sqrt();
    let inside = d_xy.max(d_z).min(0.0);
    out_len + inside
}

#[derive(Debug, thiserror::Error)]
pub enum SdfError {
    #[error("{0}")]
    Msg(String),
    #[error("io: {0}")]
    Io(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn box_center_inside() {
        let d = eval_sdf(
            SdfPrim::Box {
                dx: 10.0,
                dy: 10.0,
                dz: 10.0,
            },
            [0.0, 0.0, 0.0],
        );
        assert!(d < 0.0, "center should be inside, d={d}");
        assert!((d + 5.0).abs() < 1e-9, "half-extent 5, d={d}");
    }

    #[test]
    fn box_outside() {
        let d = eval_sdf(
            SdfPrim::Box {
                dx: 10.0,
                dy: 10.0,
                dz: 10.0,
            },
            [20.0, 0.0, 0.0],
        );
        assert!(d > 0.0);
        assert!((d - 15.0).abs() < 1e-9, "d={d}");
    }

    #[test]
    fn cyl_axis() {
        let d = eval_sdf(SdfPrim::Cylinder { r: 5.0, h: 20.0 }, [0.0, 0.0, 0.0]);
        assert!(d < 0.0);
    }

    #[test]
    fn sample_and_write() {
        let prim = SdfPrim::Box {
            dx: 20.0,
            dy: 10.0,
            dz: 8.0,
        };
        let grid = grid_for_prim(prim, 16, 2.0);
        let vol = sample_analytic(prim, &grid).unwrap();
        assert_eq!(vol.values.len(), grid.dims[0] * grid.dims[1] * grid.dims[2]);
        let dir = std::env::temp_dir().join("cadrion-sdf-test");
        let _ = fs::remove_dir_all(&dir);
        let (raw, meta) = write_raw(&vol, &dir, "box").unwrap();
        assert!(raw.is_file());
        assert!(meta.is_file());
        let (nrrd, raw2) = write_nrrd(&vol, &dir, "box").unwrap();
        assert!(nrrd.is_file());
        assert!(raw2.is_file());
        let text = fs::read_to_string(nrrd).unwrap();
        assert!(text.contains("NRRD0004"));
        assert!(text.contains("float"));
    }
}
