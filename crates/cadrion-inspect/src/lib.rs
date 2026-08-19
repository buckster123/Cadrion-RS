//! Inspect engines: topology snapshot, refs inventory, measurements, align/frame/diff.

#![deny(unsafe_code)]

mod align;
mod diff;
mod dims;
mod frame;
mod lookup;
mod measure;
mod refs;
mod topology;

pub use align::{align_refs, AlignError, AlignExpect, AlignReport};
pub use diff::{diff_snapshots, DiffReport, SelectorRemap};
pub use dims::{build_drawing_packet, DimFact, DimSpec, DrawingPacket};
pub use frame::{frame_of, FrameError, FrameReport};
pub use measure::{measure, MeasureError, MeasureKind, MeasureRequest, MeasureResult};
pub use refs::{inspect_refs, RefEntry, RefsReport};
pub use topology::{
    box_topology, cylinder_topology, EdgeRec, FaceRec, SolidRec, TopologySnapshot, VertexRec,
};

/// Crate version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
