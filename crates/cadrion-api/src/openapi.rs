//! Minimal OpenAPI 3.1 document (hand-maintained alpha; schema CI later).

use serde_json::{json, Value};

pub fn openapi_doc() -> Value {
    json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Cadrion API",
            "version": env!("CARGO_PKG_VERSION"),
            "description": "Local Cadrion HTTP surface — mirrors MCP tools under /v1"
        },
        "servers": [{"url": "http://127.0.0.1:7410"}],
        "paths": {
            "/v1/health": {
                "get": {
                    "summary": "Health",
                    "responses": {"200": {"description": "ok"}}
                }
            },
            "/v1/build": {
                "post": {
                    "summary": "Build .cad.star (IR + mock facts)",
                    "requestBody": {"required": true},
                    "responses": {"200": {"description": "build result"}}
                }
            },
            "/v1/inspect/refs": {
                "post": {
                    "summary": "Inspect refs",
                    "responses": {"200": {"description": "refs report"}}
                }
            },
            "/v1/inspect/measure": {
                "post": {
                    "summary": "Measure between selectors (H4-3 / MCP measure)",
                    "responses": {"200": {"description": "measure result"}}
                }
            },
            "/v1/inspect/dims": {
                "post": {
                    "summary": "PMI drawing packet (H4-3 / MCP inspect_dims)",
                    "responses": {"200": {"description": "drawing packet"}}
                }
            },
            "/v1/inspect/align": {
                "post": {
                    "summary": "Align two selectors (H5-2 / MCP align_check)",
                    "responses": {"200": {"description": "align report"}}
                }
            },
            "/v1/inspect/frame": {
                "post": {
                    "summary": "Local frame for a selector (H5-2 / MCP frame)",
                    "responses": {"200": {"description": "frame report"}}
                }
            },
            "/v1/inspect/diff": {
                "post": {
                    "summary": "Diff two builds (H5-2 / MCP diff)",
                    "responses": {"200": {"description": "diff report"}}
                }
            },
            "/v1/sdf/sample": {
                "post": {
                    "summary": "Secondary SDF sample (H4-3 / MCP sdf_sample). Not modeling.",
                    "responses": {"200": {"description": "raw + NRRD paths"}}
                }
            },
            "/v1/export": {
                "post": {
                    "summary": "Export stl/gltf preview or STEP (H5-3 / MCP export). Mock STEP is Unsupported.",
                    "responses": {"200": {"description": "export result"}, "400": {"description": "unsupported format/kernel"}}
                }
            },
            "/v1/fab/check": {
                "post": {
                    "summary": "DFM preflight on a FlatPart JSON (H5-4 / MCP fab_check). No printer start.",
                    "responses": {"200": {"description": "dfm report"}, "400": {"description": "bad part or unknown profile"}}
                }
            },
            "/v1/engine": {
                "post": {
                    "summary": "Kernel inventory (H5-5 / MCP engine). install is fail-closed; no tarball.",
                    "responses": {"200": {"description": "engine info or already_present"}, "400": {"description": "ENGINE-MISSING"}}
                }
            },
            "/v1/schema": {
                "post": {
                    "summary": "Live MCP/error/OpenAPI dump (H5-5 / MCP schema). clap CLI remains `cadrion schema`.",
                    "responses": {"200": {"description": "face dump"}, "400": {"description": "unknown face"}}
                }
            },
            "/v1/robot/gen": {
                "post": {
                    "summary": "Generate URDF/SRDF/SDF from robot JSON (H5-8 / MCP robot gen). Inertials not invented.",
                    "responses": {"200": {"description": "files + report"}, "400": {"description": "bad spec"}}
                }
            },
            "/v1/robot/validate": {
                "post": {
                    "summary": "Validate robot JSON or URDF/SRDF/SDF (H5-8 / MCP robot validate). Does not write.",
                    "responses": {"200": {"description": "validation report"}, "400": {"description": "bad spec"}}
                }
            },
            "/v1/snapshot": {
                "post": {
                    "summary": "Snapshot packet",
                    "responses": {"200": {"description": "snapshot result"}}
                }
            },
            "/v1/parts/search": {
                "post": {
                    "summary": "Search local STEP catalog (H6-1 / MCP parts search). Not a storefront.",
                    "responses": {"200": {"description": "candidates"}}
                }
            },
            "/v1/parts/fetch": {
                "post": {
                    "summary": "Resolve local part id → path + sha256. Does not download.",
                    "responses": {"200": {"description": "meta"}, "400": {"description": "not found"}}
                }
            },
            "/v1/viewer/open": {
                "post": {
                    "summary": "Viewer deep links (H6-2 / MCP viewer_open). once=true only — no accept loop, not wgpu.",
                    "responses": {"200": {"description": "links"}, "400": {"description": "missing path or once=false"}}
                }
            },
            "/v1/parts/lock": {
                "post": {
                    "summary": "Pin a local part into parts.lock and verify sha256.",
                    "responses": {"200": {"description": "lock entry"}, "400": {"description": "missing or checksum"}}
                }
            },
            "/v1/assembly/validate": {
                "post": {
                    "summary": "Validate assembly spec + parts.lock keys",
                    "responses": {"200": {"description": "validation"}}
                }
            },
            "/v1/jobs": {
                "post": {
                    "summary": "Create async job",
                    "responses": {"200": {"description": "job"}}
                }
            },
            "/v1/jobs/{id}": {
                "get": {
                    "summary": "Get job",
                    "responses": {"200": {"description": "job"}}
                }
            },
            "/v1/jobs/{id}/events": {
                "get": {
                    "summary": "SSE job events",
                    "responses": {"200": {"description": "text/event-stream"}}
                }
            },
            "/v1/openapi.json": {
                "get": {
                    "summary": "This document",
                    "responses": {"200": {"description": "openapi"}}
                }
            }
        },
        "components": {
            "securitySchemes": {
                "bearer": {
                    "type": "http",
                    "scheme": "bearer"
                }
            }
        }
    })
}
