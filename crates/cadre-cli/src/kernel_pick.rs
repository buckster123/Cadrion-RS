//! Kernel selection.

use cadre_kernel::{GeomKernel, MockKernel};

use crate::cli::KernelId;
use crate::output::ExitCode;

pub fn default_kernel_id() -> &'static str {
    if cfg!(feature = "occt") {
        "occt"
    } else {
        "mock"
    }
}

pub enum KernelBox {
    Mock(MockKernel),
    #[cfg(feature = "occt")]
    Occt(cadre_occt::OcctKernel),
    Truck(cadre_truck::TruckKernel),
    #[cfg(feature = "truck-brep")]
    TruckBrep(cadre_truck::TruckBrepKernel),
}

impl KernelBox {
    pub fn as_mut(&mut self) -> &mut dyn GeomKernel {
        match self {
            Self::Mock(k) => k,
            #[cfg(feature = "occt")]
            Self::Occt(k) => k,
            Self::Truck(k) => k,
            #[cfg(feature = "truck-brep")]
            Self::TruckBrep(k) => k,
        }
    }

    pub fn id(&self) -> &'static str {
        match self {
            Self::Mock(_) => "mock",
            #[cfg(feature = "occt")]
            Self::Occt(_) => "occt",
            Self::Truck(_) => "truck",
            #[cfg(feature = "truck-brep")]
            Self::TruckBrep(_) => "truck-brep",
        }
    }

    pub fn version(&self) -> String {
        match self {
            Self::Mock(k) => k.backend_version().to_string(),
            #[cfg(feature = "occt")]
            Self::Occt(k) => k.backend_version().to_string(),
            Self::Truck(k) => k.backend_version().to_string(),
            #[cfg(feature = "truck-brep")]
            Self::TruckBrep(k) => k.backend_version().to_string(),
        }
    }
}

pub fn open_kernel(id: KernelId) -> Result<KernelBox, (ExitCode, serde_json::Value)> {
    match id {
        KernelId::Mock => Ok(KernelBox::Mock(MockKernel::new())),
        KernelId::Truck => Ok(KernelBox::Truck(cadre_truck::TruckKernel::new())),
        KernelId::TruckBrep => {
            #[cfg(feature = "truck-brep")]
            {
                Ok(KernelBox::TruckBrep(cadre_truck::TruckBrepKernel::new()))
            }
            #[cfg(not(feature = "truck-brep"))]
            {
                Err((
                    ExitCode::Usage,
                    serde_json::json!({
                        "ok": false,
                        "diagnostics": [{
                            "code": "CADRE-E-KERNEL-UNAVAILABLE",
                            "severity": "error",
                            "message": "truck-brep kernel not compiled into this binary",
                            "hint": "rebuild with: cargo build -p cadre-cli --features truck-brep"
                        }]
                    }),
                ))
            }
        }
        KernelId::Occt => {
            #[cfg(feature = "occt")]
            {
                Ok(KernelBox::Occt(cadre_occt::OcctKernel::new()))
            }
            #[cfg(not(feature = "occt"))]
            {
                Err((
                    ExitCode::Usage,
                    serde_json::json!({
                        "ok": false,
                        "diagnostics": [{
                            "code": "CADRE-E-KERNEL-UNAVAILABLE",
                            "severity": "error",
                            "message": "occt kernel not compiled into this binary",
                            "hint": "rebuild with: cargo build -p cadre-cli --features occt (and CMAKE_POLICY_VERSION_MINIMUM=3.5 on CMake ≥ 4)"
                        }]
                    }),
                ))
            }
        }
    }
}
