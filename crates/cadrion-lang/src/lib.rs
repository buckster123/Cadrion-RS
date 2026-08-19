//! Cadrion Starlark host — hermetic evaluation to feature IR.
//!
//! ```
//! use cadrion_lang::{evaluate, EvalOptions};
//!
//! let src = r#"
//! def gen_step():
//!     return solid(box(10.0, 20.0, 30.0, at=CENTER), label="b")
//! "#;
//! let r = evaluate(src, &EvalOptions::new("t.cad.star"));
//! assert!(r.ok, "{:?}", r.diagnostics);
//! assert_eq!(r.ir.as_ref().unwrap().label.as_deref(), Some("b"));
//! ```

#![deny(unsafe_code)]

#[macro_use]
extern crate starlark;

mod diagnostic;
mod eval;
mod execute;
mod ir;
mod migrate;
mod stdlib;

pub use diagnostic::{Diagnostic, Severity, Span};
pub use eval::{evaluate, EvalMeta, EvalOptions, EvalResult};
pub use execute::execute_ir;
pub use ir::{BooleanKind, FeatureIr, IrNode, NodeId, IR_VERSION};
pub use migrate::{migrate_build123d_skeleton, MigrateReport};

/// Crate version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
