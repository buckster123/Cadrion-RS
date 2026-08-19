//! Fabrication path: DXF, DFM, slicer orch, gcode-check, printers.

#![deny(unsafe_code)]

mod dfm;
mod dxf;
mod face_dxf;
mod gcode;
mod gcode_path;
mod printer;
mod slicer;

pub use dfm::{
    apply_override, bundled_profiles, check_dfm, load_override_json, load_profile_json,
    pcb_outline_v1, resolve_bundled_profile, sendcutsend_laser_v1, validate_profile, waterjet_v1,
    DfmFinding, DfmOverride, DfmProfile, DfmReport, DfmRulesPatch, DfmSeverity, FlatPart,
    DFM_OVERRIDE_SCHEMA, DFM_PROFILE_SCHEMA, DFM_SCHEMA_VERSION,
};
pub use dxf::{plate_with_holes_dxf, write_dxf_r12, DxfEntity, DxfLayer};
pub use face_dxf::{face_to_dxf, FaceDxfReport, FacePick};
pub use gcode::{check_gcode, GcodeFlavor, GcodeReport, PrinterVolume};
pub use gcode_path::{extract_gcode_path, GcodeLayer, GcodePath, GcodePoint};
pub use printer::{
    evaluate_start_gates, hex_sha256, BambuAdapter, BambuTransport, DryRunReport,
    ExternalLiveTransport, ExternalMoonrakerTransport, ExternalOctoPrintTransport, KlipperAdapter,
    MoonrakerTransport, NullMoonrakerTransport, NullOctoPrintTransport, NullTransport,
    OctoPrintAdapter, OctoPrintTransport, Printer, PrinterError, PrinterInfo,
    RecordingMoonrakerTransport, RecordingOctoPrintTransport, RecordingTransport, StartGate,
    StartRequest, CONFIRM_START,
};
pub use slicer::{
    discover_slicers, run_slice, slice_command_preview, SliceGate, SliceReport, SliceRequest,
    SlicerInfo, SlicerKind, CONFIRM_SLICE,
};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
