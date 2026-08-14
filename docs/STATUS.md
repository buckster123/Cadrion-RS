# Cadre-RS — live status

> AI-oriented status block. Prefer this + `docs/METRICS.md` + `BACKLOG.md` + Horizon boards.

**As of:** 2026-08-06 · **tip:** `main` @ H2-4 + MCP NDJSON (#35–#39) · **version:** 0.1.0  
**agent_id:** `CADRE` · **repo:** https://github.com/buckster123/Cadre-RS  
**kernels:** mock (default CI) · occt (`--features occt`) · truck (experimental NON-PARITY)

## Ship state
- **v1 surface (M0–M6 / S0–S12): COMPLETE**
- **Horizon-1 (H1–H10): COMPLETE** (PRs #24–#34)
- **Horizon-2:** H2-1…H2-4 done (#35–#38); MCP Hermes wire (#39 NDJSON)
- CI: ubuntu + windows + wasm job; OCCT-free default workspace
- Binary: `cadre` (`~/.local/bin/cadre` for Hermes MCP)

## Next board
**Active:** [`docs/HORIZON3.md`](HORIZON3.md). Default next: **H3-8** (H3-2 blocked backends).  
**Archive:** H2 + H1 boards.

H3-1…H3-7 · fillet-occt +13 filleted L · next DFM governance …  

## Crate map (as-built)
| Crate | Role |
|-------|------|
| cadre | facade |
| cadre-kernel | GeomKernel + MockKernel |
| cadre-occt | OCCT backend (LGPL, non-default CI) |
| cadre-lang | hermetic Starlark → IR + execute_ir (+ migrate H8) |
| cadre-model | selectors + BuildCache |
| cadre-inspect | refs / measure / align / frame / diff |
| cadre-render | software z-buffer PNG + orbit GIF |
| cadre-bench | parity parts1-10 mock + OCCT lanes |
| cadre-mcp | stdio (NDJSON/CL) + streamable HTTP MCP |
| cadre-api | Axum `/v1/*` + jobs/SSE/OpenAPI |
| cadre-parts | parts.lock + LocalFsProvider + AssemblySpec |
| cadre-robot | URDF/SRDF/SDF + urdf-rs + jog |
| cadre-fab | DXF, DFM (laser/pcb/waterjet), slicer, Bambu/Klipper/OctoPrint |
| cadre-harness | agent10 scripted + live `--cmd` / `@oracle` |
| cadre-truck | experimental kernel: seed CSG + optional H3-6 truck-brep |
| cadre-sdf | experimental secondary SDF sample (never modeling) |
| cadre-wasm | WASM mock IR escape hatch (H2-1) |
| cadre-cli | clap binary |

## CLI surface (high signal)
```
build | inspect refs|measure|align|frame|diff | export | migrate
snapshot | view | bench run | harness run
mcp | serve api|mcp | skills export [--all]
robot gen|validate
fab dxf|dxf-face|check|profiles|slicers|slice|gcode-check
printer status|dry-run|start [--live] [--backend bambu|klipper|octoprint]
version --json
```

## Hermes MCP
- Config: `~/.hermes/config.yaml` → `mcp_servers.cadre` · binary `~/.local/bin/cadre mcp`
- Tools: build · write_source · read_source · inspect_refs · measure · snapshot
- Docs: [`HERMES_MCP.md`](HERMES_MCP.md) · framing auto-detect NDJSON (Hermes) / Content-Length

## Examples
- `parity/parts/01..12` — geometry fixtures
- `harness/tasks/` + `harness/scores/` — agent10 + published live control
- `examples/assembly/` · `examples/robots/` · `examples/fab/` · `examples/studio/` · `examples/wasm/`

## Honesty defaults
- Mock ≠ OCCT; STEP needs `--features occt` + `--kernel occt`
- Snapshot cut/polar preview is approximate (mesh notes)
- Printer: allowlist + sha256 + `confirm=START` + optional `--live`
- DFM = profile-version truth, not vendor API
- Harness `@oracle` ≠ frontier LLM score (`docs/HARNESS_LIVE.md`)
- Truck / WASM = non-parity escape hatches, never default
