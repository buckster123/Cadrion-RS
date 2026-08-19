<div align="center">

<img src="assets/banner.jpg" alt="Cadre-RS" width="100%">

<h1>Cadre-RS</h1>

<p><strong>CAD runtime for AI agents — hermetic Starlark in, verified STEP out.</strong><br>
Rust-native toolkit: build, inspect, snapshot, export, source parts, describe robots, and
hand off to fabrication through CLI, MCP, and local HTTP. Clean-room peer to text-to-cad skills.</p>

<p>
<img alt="license" src="https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue">
<img alt="rust" src="https://img.shields.io/badge/rust-2021-orange?logo=rust&logoColor=white">
<img alt="ci" src="https://img.shields.io/github/actions/workflow/status/buckster123/Cadre-RS/ci.yml?label=ci">
<img alt="status" src="https://img.shields.io/badge/status-v1%20surface%20·%20S0–S12-brightgreen">
</p>

</div>

---

> [!NOTE]
> Model code has zero ambient authority (no clock, net, or filesystem). Hardware and vendor
> effects are dry-run-first and consent-gated on every surface — nothing prints by default.

## Status

**v1 surface complete (S0–S12 / M0–M6)** plus **Horizon-1 H1–H10** and **Horizon-2 H2-1…H2-10**.
Default path is mock-kernel CI-green on Linux + Windows (+ wasm job).
**Active board:** [`docs/HORIZON3.md`](docs/HORIZON3.md) — H3-1…H3-10 cooked except
**H3-2** (frontier harness; blocked on a live backend). Name: **Cadre-RS**
([`docs/NAME_OQ1.md`](docs/NAME_OQ1.md)).
Archives: [`docs/HORIZON2.md`](docs/HORIZON2.md) · [`docs/HORIZON.md`](docs/HORIZON.md).  
Truck bid NO-GO: [`docs/TRUCK_PARITY_BID.md`](docs/TRUCK_PARITY_BID.md). Compact status:
[`docs/STATUS.md`](docs/STATUS.md) · scorecard: [`docs/METRICS.md`](docs/METRICS.md).
Hermes MCP: [`docs/HERMES_MCP.md`](docs/HERMES_MCP.md).

## What it is

Cadre is a single workspace that turns agent-written parametric CAD (Starlark) into B-rep
geometry via an optional OCCT-backed kernel, then gives the agent numeric facts, stable
selectors, mandatory visual review packets, and paths to parts catalogs, robot descriptions,
and fab tools. Prompt-ware (exported skill packs) is half the product: doctrine for the loop,
not just binaries.

## Install

```sh
git clone https://github.com/buckster123/Cadre-RS
cd Cadre-RS
cargo build -p cadre-cli --release
# optional OCCT kernel (local; long first build):
# CMAKE_POLICY_VERSION_MINIMUM=3.5 cargo build -p cadre-cli --release --features occt
```

`cargo install cadre` is **not** this project (crates.io `cadre` is Modal’s archived
config server). Build `cadre-cli` from this repo, or later `cargo install cadre-cli`.
See [`docs/NAME_OQ1.md`](docs/NAME_OQ1.md).

## Use

```sh
cargo run -p cadre-cli -- build parity/parts/01_calibration_block/part.cad.star --json
cargo run -p cadre-cli -- inspect refs parity/parts/01_calibration_block/part.cad.star --facts --json
cargo run -p cadre-cli -- snapshot parity/parts/01_calibration_block/part.cad.star --json
# STEP/STL with OCCT:
# cargo run -p cadre-cli --features occt -- --kernel occt build … --json
```

### Shipped slices (S0–S12)

| Slice | Surface |
|-------|---------|
| S0–S5 | kernel · Starlark · OCCT · selectors · CLI build/inspect/export |
| S6 | parity parts 1–4 (`cadre bench`) |
| S7 | `snapshot` / `view` (PNG + orbit GIF) |
| S8 | `mcp` + `skills export` |
| S9 | `serve api` + parts.lock + assembly |
| S10 | `robot gen\|validate` (URDF/SRDF/SDF) |
| S11 | `fab` / `printer` (DXF, DFM, gcode, gated dry-run) |
| S12 | metrics · licensing · Windows CI · `skills export --all` |

### Quick tests

```sh
cargo run -p cadre-cli -- version --json
cargo run -p cadre-cli -- snapshot parity/parts/01_calibration_block/part.cad.star --json
cargo run -p cadre-cli -- serve api --port 7410 --project examples/assembly --token dev
# second terminal:
curl -s -H "Authorization: Bearer dev" -H 'content-type: application/json' \
  -d '{"path":"plate_bolt.assy.json"}' http://127.0.0.1:7410/v1/assembly/validate
cargo run -p cadre-cli -- robot gen examples/robots/simple_arm.robot.json -o /tmp/arm --json
cargo run -p cadre-cli -- fab check --part-json examples/fab/plate.flat.json --json
cargo run -p cadre-cli -- fab gcode-check examples/fab/sample.gcode --json
cargo run -p cadre-cli -- printer dry-run examples/fab/sample.gcode --json
cargo run -p cadre-cli -- skills export --all -o dist/skills --json
```

## How it works

```
agent/human ──CLI/MCP/HTTP──▶ cadre-lang (Starlark) → IR → GeomKernel (mock | OCCT)
                              inspect · snapshot · export · parts · robot · fab
```

Contract: [`docs/design.md`](docs/design.md). Binding decisions: [`docs/CHARTER.md`](docs/CHARTER.md).
Full PRD: [`docs/cadre-prd.md`](docs/cadre-prd.md). Live status: [`docs/STATUS.md`](docs/STATUS.md).

## Docs

| File | What's in it |
|------|--------------|
| [`docs/STATUS.md`](docs/STATUS.md) | Live as-built status (start here after pull) |
| [`docs/design.md`](docs/design.md) | The contract — wire format, API, invariants |
| [`docs/CHARTER.md`](docs/CHARTER.md) | Binding decisions, phases, scope fence |
| [`docs/METRICS.md`](docs/METRICS.md) | v1 exit metrics scorecard |
| [`docs/LICENSING.md`](docs/LICENSING.md) | Dual-license + OCCT LGPL fence |
| [`docs/WINDOWS.md`](docs/WINDOWS.md) | Windows build notes |
| [`docs/gotchas.md`](docs/gotchas.md) | Operator pitfalls |
| [`docs/occt-binding.md`](docs/occt-binding.md) | OCCT backend strategy |
| [`docs/occt-depth.md`](docs/occt-depth.md) | Live topology + known cut abort |
| [`docs/cadre-prd.md`](docs/cadre-prd.md) | Product requirements, parity matrix, NFRs |
| [`BACKLOG.md`](BACKLOG.md) | Slice ledger — S0–S12 done; post-v1 parking |

## License

MIT OR Apache-2.0 — see [LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE).
The optional OCCT engine component is LGPL-2.1 with the OCCT exception and is distributed
separately (see [`docs/LICENSING.md`](docs/LICENSING.md)).

<sub>Banner generated with <a href="https://github.com/buckster123/Imaginarium-RS">Imaginarium-RS</a> (job <code>01KZ94QZ21JH73Y7J64A2ENW90</code>).</sub>
