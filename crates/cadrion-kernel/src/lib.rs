//! Cadrion geometry kernel contract.
//!
//! Pure Rust types + [`GeomKernel`] trait. No Open CASCADE linkage here — that lives in
//! `cadrion-occt` (see `docs/occt-binding.md`).
//!
//! # Example
//!
//! ```
//! use cadrion_kernel::{GeomKernel, MockKernel, Placement};
//!
//! let mut k = MockKernel::new();
//! let id = k.box_solid(10.0, 20.0, 30.0, Placement::IDENTITY).unwrap();
//! let facts = k.facts(id).unwrap();
//! assert!((facts.volume_mm3 - 6000.0).abs() < 1e-9);
//! assert_eq!(k.backend_id(), "mock");
//! assert!(!k.parity_eligible());
//! ```

#![deny(unsafe_code)]
// missing_docs is nice-to-have; clippy CI is -D warnings and field noise isn't the gate.

mod env;
mod error;
mod facts;
mod handles;
mod kernel;
mod mesh;
mod mock;
mod step;
mod types;

pub use env::{env_var, schema_matches};
pub use error::{KernelError, KernelResult};
pub use facts::{ShapeFacts, ValidityReport};
pub use handles::{EdgeRef, FaceRef, ShapeId, ShapeLabel};
pub use kernel::GeomKernel;
pub use mesh::Mesh;
pub use mock::MockKernel;
pub use step::{StepReadKind, StepReadOpts, StepSchema, StepWriteOpts};
pub use types::{BBox, BooleanOp, Density, Placement, Point3, TessTol, Vec3};

/// Crate version (build meta / `engine_info`).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
