//! Enumerable `CADRION-E-*` codes (D13 / `cadrion schema errors`).
//!
//! Agents branch on these strings. Adding a code here is required when a surface
//! starts emitting it — do not invent a diagnostic code only at a call site.

/// One stable diagnostic code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ErrorCode {
    /// Machine id, e.g. `CADRION-E-UNSUPPORTED`.
    pub code: &'static str,
    /// One-line meaning (not a full hint).
    pub meaning: &'static str,
}

/// Sorted unique catalog. `cadrion schema errors` dumps this list.
pub const ERROR_CATALOG: &[ErrorCode] = &[
    ErrorCode {
        code: "CADRION-E-ALIGN",
        meaning: "alignment / mating check failed",
    },
    ErrorCode {
        code: "CADRION-E-BAD-RETURN",
        meaning: "gen_step() did not return a solid handle",
    },
    ErrorCode {
        code: "CADRION-E-BENCH",
        meaning: "parity bench runner failure",
    },
    ErrorCode {
        code: "CADRION-E-CHAMFER-FAILED",
        meaning: "chamfer could not be applied (distance / edges)",
    },
    ErrorCode {
        code: "CADRION-E-DFM-DRIFT",
        meaning: "DFM override base_version does not match bundled profile",
    },
    ErrorCode {
        code: "CADRION-E-DFM-OVERRIDE",
        meaning: "DFM override file invalid",
    },
    ErrorCode {
        code: "CADRION-E-DXF-FACE",
        meaning: "planar face → DXF projection failed",
    },
    ErrorCode {
        code: "CADRION-E-ENGINE-MISSING",
        meaning: "requested engine component is not installed (see cadrion engine)",
    },
    ErrorCode {
        code: "CADRION-E-EVAL",
        meaning: "Starlark evaluation failed",
    },
    ErrorCode {
        code: "CADRION-E-EXPLICIT-TARGET",
        meaning: "directory-wide or ambient target refused",
    },
    ErrorCode {
        code: "CADRION-E-FAB",
        meaning: "fabrication / slicer handoff failed",
    },
    ErrorCode {
        code: "CADRION-E-FILLET-FAILED",
        meaning: "fillet could not be applied (radius / edges)",
    },
    ErrorCode {
        code: "CADRION-E-FILLET-RADIUS",
        meaning: "fillet radius vs adjacent edge (PRD example; prefer FILLET-FAILED in OCCT)",
    },
    ErrorCode {
        code: "CADRION-E-FRAME",
        meaning: "local frame query failed",
    },
    ErrorCode {
        code: "CADRION-E-HARNESS",
        meaning: "agent harness driver failure",
    },
    ErrorCode {
        code: "CADRION-E-HERMETIC-LOAD",
        meaning: "load() refused — model code has no filesystem",
    },
    ErrorCode {
        code: "CADRION-E-INTERNAL",
        meaning: "internal serialize / unexpected failure",
    },
    ErrorCode {
        code: "CADRION-E-INVALID-ARG",
        meaning: "caller passed a bad dimension or argument",
    },
    ErrorCode {
        code: "CADRION-E-IO",
        meaning: "filesystem or encoding failure",
    },
    ErrorCode {
        code: "CADRION-E-IR",
        meaning: "feature IR construction failed",
    },
    ErrorCode {
        code: "CADRION-E-IR-REF",
        meaning: "IR node / shape ref missing",
    },
    ErrorCode {
        code: "CADRION-E-KERNEL",
        meaning: "geometry kernel operation failed",
    },
    ErrorCode {
        code: "CADRION-E-KERNEL-UNAVAILABLE",
        meaning: "kernel not compiled into this binary",
    },
    ErrorCode {
        code: "CADRION-E-MEASURE",
        meaning: "measure request failed",
    },
    ErrorCode {
        code: "CADRION-E-NO-ENTRY",
        meaning: "module must define gen_step()",
    },
    ErrorCode {
        code: "CADRION-E-RENDER",
        meaning: "snapshot / raster failed",
    },
    ErrorCode {
        code: "CADRION-E-ROBOT",
        meaning: "URDF / SRDF / SDF gen or validate failed",
    },
    ErrorCode {
        code: "CADRION-E-SKILLS",
        meaning: "skill-pack export failed",
    },
    ErrorCode {
        code: "CADRION-E-TOPO",
        meaning: "topology / selector mapping failed",
    },
    ErrorCode {
        code: "CADRION-E-UNKNOWN-EDGE",
        meaning: "edge handle out of range",
    },
    ErrorCode {
        code: "CADRION-E-UNKNOWN-SHAPE",
        meaning: "shape handle missing in this kernel instance",
    },
    ErrorCode {
        code: "CADRION-E-UNSUPPORTED",
        meaning: "backend deliberately does not implement the op",
    },
    ErrorCode {
        code: "CADRION-E-USAGE",
        meaning: "bad flags or arguments",
    },
    ErrorCode {
        code: "CADRION-E-VIEW",
        meaning: "local viewer failed",
    },
];

/// Look up a catalogued code.
pub fn error_code(code: &str) -> Option<&'static ErrorCode> {
    ERROR_CATALOG.iter().find(|c| c.code == code)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::KernelError;
    use crate::handles::{EdgeRef, ShapeId};

    #[test]
    fn catalog_sorted_unique() {
        let codes: Vec<&str> = ERROR_CATALOG.iter().map(|c| c.code).collect();
        let mut sorted = codes.clone();
        sorted.sort_unstable();
        assert_eq!(codes, sorted, "ERROR_CATALOG must stay sorted by code");
        for w in codes.windows(2) {
            assert_ne!(w[0], w[1], "duplicate error code {}", w[0]);
        }
        assert!(ERROR_CATALOG
            .iter()
            .all(|c| c.code.starts_with("CADRION-E-")));
    }

    #[test]
    fn kernel_error_codes_are_catalogued() {
        let samples = [
            KernelError::unsupported("mock", "fillet"),
            KernelError::unknown_shape(ShapeId(1)),
            KernelError::UnknownEdge {
                edge: EdgeRef(0),
                shape: ShapeId(1),
            },
            KernelError::invalid_arg("nope"),
            KernelError::io("disk"),
            KernelError::diagnostic("CADRION-E-FILLET-FAILED", "x", None),
        ];
        for err in samples {
            let code = err.code();
            assert!(
                error_code(code).is_some(),
                "KernelError code {code} missing from ERROR_CATALOG"
            );
        }
    }
}
