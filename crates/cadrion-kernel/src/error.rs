//! Shared error type for kernel operations.
//!
//! Codes are stable strings agents branch on (`CADRION-E-…`). Keep them aligned with
//! `cadrion schema errors` once that surface exists.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::handles::{EdgeRef, ShapeId};

/// Kernel-level failure. Never a fake success — callers map this into CLI/MCP diagnostics.
#[derive(Debug, Error, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum KernelError {
    /// Structured diagnostic with stable `CADRION-E-*` code.
    #[error("{code}: {message}")]
    Diagnostic {
        /// Stable code, e.g. `CADRION-E-FILLET-RADIUS`.
        code: String,
        /// Human-readable message.
        message: String,
        /// Actionable fix hint when known.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        hint: Option<String>,
        /// Related shape when applicable.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        shape: Option<ShapeId>,
        /// Selector tokens involved.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        refs: Vec<String>,
    },

    /// Handle not present in this kernel instance.
    #[error("shape {id} not found in this kernel instance")]
    UnknownShape {
        /// Missing shape.
        id: ShapeId,
    },

    /// Edge ref out of range for shape.
    #[error("edge {edge} not found on shape {shape}")]
    UnknownEdge {
        /// Missing edge.
        edge: EdgeRef,
        /// Shape that was queried.
        shape: ShapeId,
    },

    /// Backend deliberately does not implement the op.
    #[error("operation not supported by backend `{backend}`: {op}")]
    Unsupported {
        /// Backend id.
        backend: String,
        /// Operation name.
        op: String,
    },

    /// Caller passed a bad dimension or argument.
    #[error("invalid argument: {message}")]
    InvalidArg {
        /// Detail.
        message: String,
    },

    /// Filesystem or encoding failure.
    #[error("I/O: {message}")]
    Io {
        /// Detail.
        message: String,
    },
}

impl KernelError {
    /// Stable machine code when present (`CADRION-E-…`), else a coarse class name.
    pub fn code(&self) -> &str {
        match self {
            Self::Diagnostic { code, .. } => code.as_str(),
            Self::UnknownShape { .. } => "CADRION-E-UNKNOWN-SHAPE",
            Self::UnknownEdge { .. } => "CADRION-E-UNKNOWN-EDGE",
            Self::Unsupported { .. } => "CADRION-E-UNSUPPORTED",
            Self::InvalidArg { .. } => "CADRION-E-INVALID-ARG",
            Self::Io { .. } => "CADRION-E-IO",
        }
    }

    /// Build a diagnostic error.
    pub fn diagnostic(
        code: impl Into<String>,
        message: impl Into<String>,
        hint: impl Into<Option<String>>,
    ) -> Self {
        Self::Diagnostic {
            code: code.into(),
            message: message.into(),
            hint: hint.into(),
            shape: None,
            refs: Vec::new(),
        }
    }

    /// Attach a shape id to a diagnostic (no-op for other variants).
    pub fn with_shape(mut self, id: ShapeId) -> Self {
        if let Self::Diagnostic { shape, .. } = &mut self {
            *shape = Some(id);
        }
        self
    }

    /// Attach selector refs to a diagnostic (no-op for other variants).
    pub fn with_refs(mut self, refs: impl IntoIterator<Item = impl Into<String>>) -> Self {
        if let Self::Diagnostic { refs: slot, .. } = &mut self {
            slot.extend(refs.into_iter().map(Into::into));
        }
        self
    }

    /// Convenience constructors used by backends.
    pub fn unknown_shape(id: ShapeId) -> Self {
        Self::UnknownShape { id }
    }

    /// Invalid argument helper.
    pub fn invalid_arg(message: impl Into<String>) -> Self {
        Self::InvalidArg {
            message: message.into(),
        }
    }

    /// I/O helper.
    pub fn io(message: impl Into<String>) -> Self {
        Self::Io {
            message: message.into(),
        }
    }

    /// Unsupported op helper.
    pub fn unsupported(backend: impl Into<String>, op: impl Into<String>) -> Self {
        Self::Unsupported {
            backend: backend.into(),
            op: op.into(),
        }
    }
}

/// Result alias for kernel methods.
pub type KernelResult<T> = Result<T, KernelError>;
