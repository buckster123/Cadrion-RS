//! Software mesh rendering for snapshot packets (no GPU required).

#![deny(unsafe_code)]

mod gifenc;
mod mesh_export;
mod mesh_ir;
mod packet;
mod raster;
mod views;

pub use gifenc::write_orbit_gif;
pub use mesh_export::{write_gltf_json, write_stl_ascii};
pub use mesh_ir::mesh_from_ir;
pub use packet::{write_snapshot_packet, SnapshotManifest, SnapshotOptions, SnapshotResult};
pub use raster::{render_mesh, Framebuffer, Rgba};
pub use views::{camera_for_view, ViewName};

/// Crate version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
