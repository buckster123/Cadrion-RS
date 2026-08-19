//! GIF encoding for orbit turntables.

use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use cadrion_kernel::Mesh;
use gif::{Encoder, Frame, Repeat};

use crate::raster::render_mesh;
use crate::views::{bounds_center_radius, camera_orbit, mesh_bounds};

/// Write an orbit GIF around +Z.
pub fn write_orbit_gif(
    mesh: &Mesh,
    path: &Path,
    width: u32,
    height: u32,
    frames: u32,
    delay_cs: u16,
) -> Result<(), String> {
    let (min, max) = mesh_bounds(mesh);
    let (center, radius) = bounds_center_radius(min, max);
    let elev = 0.45_f64;

    let file = File::create(path).map_err(|e| e.to_string())?;
    let mut enc = Encoder::new(BufWriter::new(file), width as u16, height as u16, &[])
        .map_err(|e| e.to_string())?;
    enc.set_repeat(Repeat::Infinite)
        .map_err(|e| e.to_string())?;

    let n = frames.max(4);
    for i in 0..n {
        let theta = std::f64::consts::TAU * (i as f64) / (n as f64);
        let cam = camera_orbit(center, radius, theta, elev);
        let fb = render_mesh(mesh, &cam, width, height);
        let rgb = fb.to_rgb8();
        let mut frame = Frame::from_rgb(width as u16, height as u16, &rgb);
        frame.delay = delay_cs;
        enc.write_frame(&frame).map_err(|e| e.to_string())?;
    }
    Ok(())
}
