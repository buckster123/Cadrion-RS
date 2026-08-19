//! Snapshot packet writer: multi-view PNG + optional orbit GIF + manifest.

use std::fs::{self, File};
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::time::Instant;

use cadrion_kernel::Mesh;
use serde::{Deserialize, Serialize};

use crate::gifenc::write_orbit_gif;
use crate::raster::render_mesh;
use crate::views::{bounds_center_radius, camera_for_view, mesh_bounds, ViewName};

#[derive(Debug, Clone)]
pub struct SnapshotOptions {
    pub views: Vec<ViewName>,
    pub width: u32,
    pub height: u32,
    pub gif: bool,
    pub gif_frames: u32,
    pub gif_delay_cs: u16,
    pub notes: Vec<String>,
}

impl Default for SnapshotOptions {
    fn default() -> Self {
        Self {
            views: vec![
                ViewName::Iso,
                ViewName::Front,
                ViewName::Top,
                ViewName::Right,
            ],
            width: 512,
            height: 512,
            gif: true,
            gif_frames: 24,
            gif_delay_cs: 6,
            notes: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotManifest {
    pub ok: bool,
    pub out_dir: PathBuf,
    pub views: Vec<ViewFile>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gif: Option<PathBuf>,
    pub width: u32,
    pub height: u32,
    pub triangles: usize,
    pub preview_mesh: bool,
    pub notes: Vec<String>,
    pub wall_ms: u64,
    pub renderer: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewFile {
    pub name: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotResult {
    pub manifest: SnapshotManifest,
}

/// Write snapshot packet under `out_dir` (created).
pub fn write_snapshot_packet(
    mesh: &Mesh,
    out_dir: &Path,
    opts: &SnapshotOptions,
) -> Result<SnapshotResult, String> {
    let started = Instant::now();
    fs::create_dir_all(out_dir).map_err(|e| e.to_string())?;

    let (min, max) = mesh_bounds(mesh);
    let (center, radius) = bounds_center_radius(min, max);

    let mut views = Vec::new();
    for v in &opts.views {
        let cam = camera_for_view(*v, center, radius);
        let fb = render_mesh(mesh, &cam, opts.width, opts.height);
        let name = format!("{}.png", v.as_str());
        let path = out_dir.join(&name);
        write_png(&path, &fb.pixels, opts.width, opts.height)?;
        views.push(ViewFile {
            name: v.as_str().into(),
            path: path.clone(),
        });
    }

    let gif_path = if opts.gif {
        let p = out_dir.join("orbit.gif");
        write_orbit_gif(
            mesh,
            &p,
            opts.width.min(256),
            opts.height.min(256),
            opts.gif_frames,
            opts.gif_delay_cs,
        )?;
        Some(p)
    } else {
        None
    };

    let manifest = SnapshotManifest {
        ok: true,
        out_dir: out_dir.to_path_buf(),
        views,
        gif: gif_path,
        width: opts.width,
        height: opts.height,
        triangles: mesh.triangle_count(),
        preview_mesh: true,
        notes: opts.notes.clone(),
        wall_ms: started.elapsed().as_millis() as u64,
        renderer: format!("cadrion-render/{} software-zbuffer", crate::VERSION),
    };
    let man_path = out_dir.join("manifest.json");
    let text = serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())?;
    fs::write(man_path, text).map_err(|e| e.to_string())?;

    Ok(SnapshotResult { manifest })
}

fn write_png(path: &Path, rgba: &[u8], width: u32, height: u32) -> Result<(), String> {
    let file = File::create(path).map_err(|e| e.to_string())?;
    let w = BufWriter::new(file);
    let mut encoder = png::Encoder::new(w, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
    writer.write_image_data(rgba).map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh_ir::mesh_from_ir;
    use cadrion_lang::{evaluate, EvalOptions};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn packet_writes_png_and_gif() {
        let src = r#"
def gen_step():
    return solid(box(40.0, 20.0, 10.0, at=CENTER), label="p")
"#;
        let r = evaluate(src, &EvalOptions::new("p.cad.star"));
        let (mesh, notes) = mesh_from_ir(r.ir.as_ref().unwrap()).unwrap();
        let mut dir = std::env::temp_dir();
        dir.push(format!(
            "cadrion-snap-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut opts = SnapshotOptions {
            width: 128,
            height: 128,
            gif_frames: 8,
            ..SnapshotOptions::default()
        };
        opts.notes = notes;
        let res = write_snapshot_packet(&mesh, &dir, &opts).unwrap();
        assert!(res.manifest.ok);
        assert_eq!(res.manifest.views.len(), 4);
        for v in &res.manifest.views {
            assert!(v.path.is_file(), "{}", v.path.display());
        }
        assert!(res.manifest.gif.as_ref().unwrap().is_file());
        let _ = fs::remove_dir_all(&dir);
    }
}
