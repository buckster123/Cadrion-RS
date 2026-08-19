//! Robot description gen/validate (URDF / SRDF / SDF).

#![deny(unsafe_code)]

mod inertial;
mod jog;
mod model;
mod sdf;
mod srdf;
mod urdf;
mod validate;

pub use inertial::{box_inertial, cylinder_inertial, Inertial};
pub use jog::{jog_payload, JogJoint, JogLink, JogPayload};
pub use model::{Collision, Geometry, Joint, JointType, Link, Material, Origin, RobotSpec, Visual};
pub use sdf::write_sdf;
pub use srdf::{srdf_from_robot, write_srdf, SrdfEndEffector, SrdfGroup, SrdfSpec};
pub use urdf::write_urdf;
pub use validate::{
    emit_and_validate, parse_urdf_xml, validate_robot, validate_sdf_xml,
    validate_srdf_against_urdf, validate_urdf_xml, ValidationReport,
};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
