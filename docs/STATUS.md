# Cadrion-RS — live status

> AI-oriented status block. Prefer this + `docs/METRICS.md` + `BACKLOG.md` + Horizon boards.

**As of:** 2026-08-21 · **tip:** H6-1 parts search/fetch/lock · **version:** 0.1.0  
**agent_id:** `CADRION` · **repo:** https://github.com/buckster123/Cadrion-RS  
**kernels:** mock (default CI) · occt (`--features occt`) · truck (experimental NON-PARITY)

## Ship state
- **v1 surface (M0–M6 / S0–S12): COMPLETE**
- **Horizon-1 (H1–H10): COMPLETE** (PRs #24–#34)
- **Horizon-2 (H2-1…H2-10): COMPLETE** (PRs #35–#46)
- **Horizon-3:** H3-1…H3-10 **complete** (H3-2 live 4.0/10, PR #60)
- **Horizon-4:** COMPLETE (PRs #57–#59) — [`HORIZON4.md`](HORIZON4.md)
- **Horizon-5:** COMPLETE (H5-1…H5-10) — [`HORIZON5.md`](HORIZON5.md)
- **Horizon-6:** ACTIVE (H6-1…H6-10) — [`HORIZON6.md`](HORIZON6.md)
- **Name (OQ-1):** Cadrion / Cadrion-RS — [`NAME_OQ1.md`](NAME_OQ1.md)
- CI: ubuntu + windows + wasm job; OCCT-free default workspace
- Binary: `cadrion` (`~/.local/bin/cadrion` for Hermes MCP). **Not** `cargo install cadre`.

## Next board
**Active:** Horizon-6 — default next **H6-2** (viewer_open).  
**Archive:** H5 + H4 + H3 + H2 + H1. Live harness re-score is parked (ApexRouter + spend).

## Crate map (as-built)
| Crate | Role |
|-------|------|
| cadrion | facade |
| cadrion-kernel | GeomKernel + MockKernel |
| cadrion-occt | OCCT backend (LGPL, non-default CI) |
| cadrion-lang | hermetic Starlark → IR + execute_ir (+ migrate H8) |
| cadrion-model | selectors + BuildCache |
| cadrion-inspect | refs / measure / align / frame / diff |
| cadrion-render | software z-buffer PNG + orbit GIF |
| cadrion-bench | parity parts1-10 mock + OCCT lanes |
| cadrion-mcp | stdio (NDJSON/CL) + streamable HTTP MCP |
| cadrion-api | Axum `/v1/*` + jobs/SSE/OpenAPI |
| cadrion-parts | parts.lock + LocalFsProvider + AssemblySpec |
| cadrion-robot | URDF/SRDF/SDF + urdf-rs + jog |
| cadrion-fab | DXF, DFM (laser/pcb/waterjet), slicer, Bambu/Klipper/OctoPrint |
| cadrion-harness | agent10 scripted + live `--cmd` / `@oracle` |
| cadrion-truck | experimental kernel: seed CSG + optional H3-6 truck-brep |
| cadrion-sdf | experimental secondary SDF sample (never modeling) |
| cadrion-wasm | WASM mock IR escape hatch (H2-1) |
| cadrion-cli | clap binary (first crates.io install crate, when published) |

## CLI surface (high signal)
```
build | inspect refs|measure|align|frame|diff | export | migrate | parts search|fetch|lock
snapshot | view | bench run | harness run
mcp | serve api|mcp | skills export [--all]
robot gen|validate
fab dxf|dxf-face|check|profiles|slicers|slice|gcode-check
printer status|dry-run|start [--live] [--backend bambu|klipper|octoprint]
engine info|install
schema [cli|mcp|api|errors]
version --json
```

## Hermes MCP
- Config: `~/.hermes/config.yaml` → `mcp_servers.cadrion` · binary `~/.local/bin/cadrion mcp`
- Tools: 18 — see [`HERMES_MCP.md`](HERMES_MCP.md) (H6-1 added parts)
- Docs: [`HERMES_MCP.md`](HERMES_MCP.md) · framing auto-detect NDJSON (Hermes) / Content-Length

## Examples
- `parity/parts/01..13` — geometry fixtures
- `harness/tasks/` + `harness/scores/` — agent10 + published live control
- `examples/assembly/` · `examples/robots/` · `examples/fab/` · `examples/studio/` · `examples/wasm/`

## Honesty defaults
- Mock ≠ OCCT; STEP needs `--features occt` + `--kernel occt`
- Snapshot cut/polar preview is approximate (mesh notes)
- Printer: allowlist + sha256 + `confirm=START` + optional `--live`
- DFM = profile-version truth, not vendor API
- Harness `@oracle` ≠ frontier LLM score (`docs/HARNESS_LIVE.md`)
- Truck / WASM = non-parity escape hatches, never default
- `cargo install cadre` is Modal’s archived config server, not this repo
