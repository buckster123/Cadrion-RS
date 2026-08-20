//! Parts catalog + lockfile + assembly specs.

#![deny(unsafe_code)]

mod assembly;
mod kinematics;
mod lock;
mod provider;

pub use assembly::{
    align_check, validate_assembly, AlignExpect, AlignReport, AssemblySpec,
    AssemblyValidationReport, ComponentSpec, JointSpec, PlacementSpec,
};
pub use kinematics::{
    assembly_kinematics, assembly_to_robot_json, AssemblyKinematics, KinematicJoint, KinematicLink,
    PlacementRecord,
};
pub use lock::{
    load_parts_lock, upsert_lock_entry, verify_lock_entry, write_parts_lock, PartsLock,
    PartsLockEntry, PartsLockError,
};
pub use provider::{
    LocalFsProvider, PartCandidate, PartMeta, PartProvider, PartRef, ProviderError,
};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
