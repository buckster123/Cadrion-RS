# Cadrion-RS — the contract

> **Contract first** (house doctrine #1). This document is pinned **before** the code it
> describes. Code follows this doc; a PR that changes behaviour updates this doc in the same
> commit. When the two disagree, that is a bug in one of them — find out which, don't guess.
>
> Full product requirements live in [`cadrion-prd.md`](cadrion-prd.md). This file is the
> **implementer contract**: surfaces, types, lifecycle, env, invariants. Charter decisions
> D1–Dn bind when this doc and the PRD disagree on scope.

## Scope

Covers the agent/hardware-design loop Cadrion exposes:

- hermetic evaluation of `*.cad.star` / `*.dxf.star` / `*.urdf.star` / `*.srdf.star` / `*.sdf.star`
- kernel execution to primary STEP (and secondary exports)
- numeric inspection (refs / measure / align / frame / diff)
- snapshots and local viewer
- parts catalog client + `parts.lock`
- robot description gen/validate
- fab preflight, slicer orchestration, gated printer handoff
- CLI · MCP · local HTTP · skill-pack export

Does **not** cover: GUI sketcher, CAM/FEA/BIM, multi-tenant SaaS, running Python CAD sources,
or treating meshes as the modeling medium (see charter non-goals).

## Architecture (crate boundaries)

Crate boundaries are requirements (charter D16). Internal module layout is free.

| Crate | Responsibility | v1 status |
|-------|----------------|-----------|
| `cadrion` | Thin facade / re-exports | shipped |
| `cadrion-kernel` | `GeomKernel` trait + `MockKernel` | shipped |
| `cadrion-occt` | OCCT backend (FFI); separate LGPL engine | shipped (local/opt-in) |
| `cadrion-lang` | Starlark host, CAD stdlib, IR emission, `execute_ir` | shipped |
| `cadrion-model` | Selectors, content hashing, build cache | shipped |
| `cadrion-inspect` | refs / measure (+ topology snapshot helpers) | shipped |
| `cadrion-render` | **Software** z-buffer: multi-view PNG, orbit GIF | shipped (wgpu parked) |
| `cadrion-bench` | Parity suite runner (parts 1–4) | shipped |
| `cadrion-parts` | `PartProvider`, `parts.lock`, assembly specs | shipped |
| `cadrion-robot` | URDF/SRDF/SDF gen + validators; analytic inertials | shipped |
| `cadrion-fab` | DFM rulepacks; slicer discovery; G-code checks; `Printer` | shipped |
| `cadrion-mcp` | MCP server (stdio + streamable HTTP) | shipped |
| `cadrion-api` | Axum HTTP API, jobs, SSE, OpenAPI | shipped |
| `cadrion-cli` | Clap front end — the `cadrion` binary | shipped |

**Parked / not crates yet:** `cadrion-truck` (experimental pure-Rust kernel), standalone
`cadrion-export` / `cadrion-viewer` / `cadrion-skills` packages (export/view/skills live in
cli/render/mcp as of S12). See `docs/STATUS.md`.

Bootstrap shipped `crates/cadrion`. **S1** added `crates/cadrion-kernel` (`GeomKernel` +
`MockKernel`). **S2** added `crates/cadrion-lang` (hermetic Starlark → feature IR v0).
**S3** added `crates/cadrion-occt` (LGPL OCCT backend) + `execute_ir`. **S4** added
`crates/cadrion-model` (selectors + content-hash cache) and `crates/cadrion-inspect`
(refs/measure). **S5** added `crates/cadrion-cli` (`cadrion` binary). Default workspace
members exclude OCCT so CI stays fast; see [`occt-binding.md`](occt-binding.md).

### CLI face (v0)

```sh
cargo run -p cadrion-cli -- build part.cad.star --json
cargo run -p cadrion-cli -- inspect refs part.cad.star --facts --json
cargo run -p cadrion-cli -- inspect measure part.cad.star '#o1.1.f1' '#o1.1.f2' --kind thickness --json
cargo run -p cadrion-cli --features occt -- --kernel occt build part.cad.star --json
```

Global: `--json`, `--quiet`, `--project`, `--kernel mock|occt`, `-v`.
Exit codes: 0 ok, 2 usage, 3 eval, 4 validation, 5 kernel, 6 io, 9 internal.
`build` refuses directories. Mock kernel writes IR + facts; STEP needs `--kernel occt`.
S5 `export --format glb` writes JSON glTF (embedded buffers) when tessellation is available.

### Parity suite (parts 1–4)

Fixtures live under `parity/parts/NN_name/{part.cad.star,expect.json}`.
Runner: `cadrion-bench` / `cadrion bench run --suite parts1-4 --json`.

Checks per part: eval · label · params · IR ops · mock execute · volume · bbox ·
faces/edges min · selector stability · optional measures.

Volumes are **MockKernel-calibrated** for default CI (no OCCT). True B-rep goldens
are a later `parity-geom` lane.

### Snapshots & viewer (v0 / S7)

```sh
cargo run -p cadrion-cli -- snapshot part.cad.star --json
# → part.snap/{iso,front,top,right}.png + orbit.gif + manifest.json

cargo run -p cadrion-cli -- view part.snap --json
# → http://127.0.0.1:7411/v/0/  (Ctrl-C to stop)

cargo run -p cadrion-cli -- view part.cad.star --once --json
# CI-friendly: builds snap packet, no server
```

Renderer is a **software z-buffer** (no GPU / wgpu yet) so CI stays headless-green.
IR→mesh is analytic (box/cylinder); boolean **cut/intersect keep operand A** in the
preview mesh and record that in `manifest.notes` / `preview_mesh: true`.

### MCP + skills (v0 / S8)

```sh
cargo run -p cadrion-cli -- mcp                 # stdio, Content-Length framing
cargo run -p cadrion-cli -- serve mcp --port 7420 --token dev   # streamable HTTP
# POST http://127.0.0.1:7420/mcp  Authorization: Bearer dev
# body: {"jsonrpc":"2.0","id":1,"method":"tools/list"}
# GET  http://127.0.0.1:7420/mcp  — SSE heartbeats
# GET  http://127.0.0.1:7420/health

cargo run -p cadrion-cli -- skills export -o dist/skills/cadrion --json
```

Tools (short schemas): `build`, `write_source`, `read_source`, `inspect_refs`, `measure`,
`snapshot` (optional base64 PNG content for iso/front).

Doctrine pack: `skills/cadrion/SKILL.md` + `references/workflow.md`.
Hand-rolled JSON-RPC (OQ-7 SDK deferred). Logs on **stderr** only.

### HTTP API + parts/assembly (v0 / S9)

```sh
cargo run -p cadrion-cli -- serve api --port 7410 --token secret
# GET  /v1/health
# GET  /v1/openapi.json
# POST /v1/build|inspect/refs|inspect/measure|inspect/dims|snapshot|sdf/sample  (Bearer token)
# POST /v1/parts/search
# POST /v1/assembly/validate
# POST /v1/jobs  + GET /v1/jobs/{id}  + GET /v1/jobs/{id}/events (SSE)
```

`parts.lock` pins provider/id/sha256/path; builds referencing lock keys **fail closed** on
missing/mismatched checksums. Assembly JSON: named components, placements, joints.
Example: `examples/assembly/plate_bolt.assy.json`.

### Robots (v0 / S10)

```sh
cargo run -p cadrion-cli -- robot gen examples/robots/simple_arm.robot.json -o /tmp/arm --json
cargo run -p cadrion-cli -- robot validate /tmp/arm/simple_arm.urdf --json
```

`RobotSpec` JSON → URDF (SI units) with analytic box/cylinder inertials; validated for
tree integrity + positive mass/inertia; re-parsed with **urdf-rs**. SRDF group from
non-fixed joints; minimal SDF model emit.

### Face → DXF (S11+)

```sh
# Largest +Z face outline from a part (mock topology)
cargo run -p cadrion-cli -- fab dxf-face parity/parts/01_calibration_block/part.cad.star \
  --normal 0,0,1 -o /tmp/face.dxf --json

# Or pick a selector from `inspect refs`
cargo run -p cadrion-cli -- fab dxf-face part.cad.star --face '#o1.1.f6' -o face.dxf --json
```

Projects coplanar edges with endpoints onto the face plane (R12 DXF, mm).
Plate helper `fab dxf --width …` remains for quick sketches without a model.

### Fabrication (v0 / S11)

```sh
cargo run -p cadrion-cli -- fab dxf --width 100 --height 50 --hole 25,25,6 -o plate.dxf --json
cargo run -p cadrion-cli -- fab check --part-json examples/fab/plate.flat.json --json
cargo run -p cadrion-cli -- fab slicers --json
cargo run -p cadrion-cli -- fab gcode-check examples/fab/sample.gcode --json
cargo run -p cadrion-cli -- printer dry-run examples/fab/sample.gcode --json
```

- DXF: R12 text, mm (`$INSUNITS=4`), outline + circles
- DFM: versioned profile data; findings cite rule + measured/limit; never claims live vendor API
- Slicer: discover PATH CLIs; print command preview (execute deferred)
- G-code: flavor heuristic, bbox vs bed, temp caps
- Printer: Bambu adapter **dry-run only** (no network); `start` needs allow-list + sha256 +
  `confirm=START`, and S11 still refuses live start by design

### Selectors (v0)

Token grammar: `#o{obj}[.{solid}][.f{face}|.e{edge}|.v{vertex}]` (1-based indices).

Ordering (kernel-independent):
- solids: sort by quantized `(cz, cy, cx, volume)`
- faces: sort by quantized `(cz, cy, cx, area)` then normal.z
- edges: sort by quantized midpoint then length

`inspect refs` emits a JSON inventory; `inspect measure` supports distance / angle /
diameter / thickness against that inventory (from a `TopologySnapshot`).

### Build cache (v0)

Key = SHA-256 of `{source_sha, params_sha, cadrion_version, kernel_id, kernel_version, ir_version}`.
Store under `<project>/.cadrion/cache/<key_digest>/` with `entry.json` + artifact; get verifies
artifact hash (corruption → miss). Warm hit is filesystem metadata + hash check only.

### Feature IR (v0, from `cadrion-lang`)

Evaluation returns `EvalResult` JSON:

```json
{
  "ok": true,
  "ir": {
    "version": 0,
    "params": {"width": 100.0},
    "nodes": [{"op": "box", "dx": 100.0, "dy": 60.0, "dz": 20.0, "at": [0,0,0]}, ...],
    "root": 3,
    "label": "calibration_block"
  },
  "diagnostics": [],
  "meta": {"source_name": "block.cad.star", "wall_ms": 12, "ir_version": 0, "node_count": 4}
}
```

On failure `ok: false`, `ir` omitted, `diagnostics[]` filled with `{code, severity, message, target, span?, hint?}`.
Hermetic: `load()` refused; no fs/net/clock in stdlib; host overrides via `EvalOptions.overrides`.

## Project layout (user projects)

```
myproject/
  cadrion.toml            # kernel, viewer port, printer allow-list, providers
  parts.lock            # pinned catalog parts + checksums
  cad/bracket.cad.star  # source …
  cad/bracket.step      # … artifact, same basename, same directory
  cad/bracket.snap/     # snapshot packets
  robots/arm.urdf.star  → robots/arm.urdf
  .cadrion/               # build cache, tess cache, logs (gitignored)
```

Paths resolve from invoking CWD, never from install/skill directories.
Config precedence: **flags > env (`CADRION_*`) > project `cadrion.toml` > user config**.

## Authoring surface

- Entry points: `gen_step()`, `gen_dxf()`, `gen_urdf()`, `gen_srdf()`, `gen_sdf()`.
- Evaluation emits **feature IR** (persisted, hashed, diffable). Kernel executes IR.
- Model code is hermetic: no clock/env/net/fs; fueled caps; deterministic iteration.
  Floats and stdlib names: [`DIALECT.md`](DIALECT.md) (H5-7). User `load()` still open (OQ-2).
- Parameter overrides at build: `--set width=120` (recorded in build metadata).
- Selectors: `#o<obj>[.<solid>][.f<face>|.e<edge>|.v<vertex>]` with kernel-independent ordering
  (centroid/area tuple + tie-breakers). In-language queries (`faces(">Z")`, …) for authoring;
  tokens for CLI/tool addressing. `diff` reports token remaps across builds.

Illustrative flavor (not final stdlib API — pin concrete names in M1 with tests):

```python
# block.cad.star
P = params(width=100.0, depth=60.0, height=20.0, hole_d=8.0, cham=2.0)

def gen_step():
    blk = box(P.width, P.depth, P.height, at=CENTER)
    holes = [cylinder(d=P.hole_d, h=P.height + 2.0, at=(x, y, 0.0))
             for x in (-40.0, 40.0) for y in (-20.0, 20.0)]
    blk = cut(blk, union(holes))
    top_edges = faces(blk, ">Z").outer_wire().edges()
    blk = chamfer(blk, edges=top_edges, size=P.cham)
    return solid(blk, label="calibration_block")
```

## The wire / API surface

Global CLI flags: `--json`, `--quiet`, `--project <dir>`, `--kernel <backend>`, `--no-color`.

| Surface | Purpose | Shape notes |
|---------|---------|-------------|
| `cadrion build <target>` | Hermetic eval + kernel → primary artifact | JSON: artifacts[], facts, diagnostics[], hashes, validity, wall_ms. Refuses directory-wide builds |
| `cadrion inspect refs\|measure\|align\|frame\|diff` | Numeric interrogation | JSON numeric results + construction text; align is pass/fail vs tol |
| `cadrion snapshot <target>` | PNG packet (+ GIF) | Files on disk; MCP/API also return image content blocks |
| `cadrion export …` | Secondary formats | Provenance: source hash + tolerances |
| `cadrion view [paths…]` | Embed viewer, reuse instance | Prints deep links (`http://127.0.0.1:<port>/…`) |
| `cadrion parts search\|show\|fetch\|lock` | Catalog + lockfile | Checksum-verified cache; fail closed on lock mismatch |
| `cadrion robot validate` | URDF/SRDF/SDF | Cross-check flags where paired |
| `cadrion fab check\|slicers\|slice\|gcode-check` | DFM + slice path | Evidence-backed findings; real slicer CLIs |
| `cadrion printer status\|upload\|dry-run\|start\|watch` | Printer handoff | `start` gated (below) |
| `cadrion serve api` | Local Axum | Loopback + bearer token default; `/v1/*` + jobs/SSE + OpenAPI |
| `cadrion mcp` | Agent tools | stdio default; progress notifications; image blocks for snapshot |
| `cadrion skills export\|install` | L2 skill packs | Original prose; tool invocations → `cadrion` |
| `cadrion bench run\|agent` | Parity-10 + agent harness | Local only; no phone-home |
| `cadrion engine info\|install` | Kernel inventory | H4-2: info is live compile flags; install refuse-or-already-present — **no tarball yet** |
| `cadrion schema [cli\|mcp\|api\|errors]` | Live schema dump | H4-1 shipped — see [`SCHEMA.md`](SCHEMA.md) |

### MCP tools (names final; schemas via `cadrion schema mcp`)

`build`, `write_source`, `read_source`, `inspect_refs`, `measure`, `align_check`, `frame`,
`diff`, `snapshot`, `export`, `viewer_open`, `parts` (`op=search|fetch|lock`; H6-1), `robot_validate`,
`fab_check`, `fab_slice`, `gcode_check`, `printer_status`, `printer_upload`, `printer_dry_run`,
`printer_start`, `project_artifacts`, `engine_info`.

Resources: `cadrion://project/**`, `cadrion://artifact/**`, `cadrion://doc/**`.

Tool descriptions + schemas total **≤ 4,000 tokens** (D12).

### HTTP API

- Bearer token (printed at start); loopback bind default.
- Sync for fast ops; async jobs: `POST /v1/jobs`, `GET /v1/jobs/{id}`, `GET /v1/jobs/{id}/events` (SSE).
- Artifacts: `GET /v1/artifacts/{hash}`.
- Families mirror MCP 1:1 under `/v1/...`. OpenAPI at `/v1/openapi.json` from the same types as `cadrion schema`.
- Versioned `/v1`; additive-only within a major.

### Printer start gate (all surfaces)

`printer start` / `printer_start` requires **all** of:

1. G-code hash matches last validated/uploaded file
2. Printer is on the allow-list in config
3. Explicit confirmation: CLI `--confirm-start`, MCP `confirm: "START"`, API `"confirm": true` + capability token

No defaulting. Refusal → exit code **8** / structured safety diagnostic.

### Exit codes (stable)

| Code | Meaning |
|------|---------|
| 0 | ok |
| 2 | usage |
| 3 | evaluation/build error |
| 4 | validation failed |
| 5 | kernel operation failed |
| 6 | I/O |
| 7 | provider/network |
| 8 | safety gate refused |
| 9 | internal |

## Types

Load-bearing serialized shapes (evolve only with schema + fixtures):

**Diagnostic**

```json
{
  "code": "CADRION-E-FILLET-RADIUS",
  "severity": "error",
  "message": "…",
  "target": "cad/flange.cad.star",
  "span": {"file": "cad/flange.cad.star", "line": 14, "col": 5},
  "refs": ["#o1.1.e7"],
  "hint": "reduce radius below 2.4 mm or exclude edge #o1.1.e7",
  "docs_url": null
}
```

Codes are stable and enumerable (`cadrion schema errors`). Kernel failures are translated to
feature-level terms (which op, which selector, plausible causes) — never raw FFI dumps as the
only message.

**Build result (`--json`)**

```json
{
  "ok": true,
  "artifacts": [{"path": "cad/block.step", "sha256": "…", "kind": "step"}],
  "facts": {
    "bbox_mm": [100.0, 60.0, 20.0],
    "volume_mm3": 137904.2,
    "area_mm2": null,
    "centroid_mm": [0.0, 0.0, 10.0],
    "solids": 1,
    "faces": 22,
    "edges": null
  },
  "validity": {"closed": true, "positive_volume": true, "shells": 1},
  "diagnostics": [],
  "meta": {
    "source_sha256": "…",
    "params": {},
    "cadrion_version": "0.1.0",
    "kernel": "occt",
    "kernel_version": "…",
    "wall_ms": 420
  }
}
```

**Job (HTTP)** — `pending | running | completed | failed` (never silent forever). Paid/long work
that outlives a client poll stays **pending/running** with a resumable id (doctrine #9), not
premature `failed`.

Float comparisons in tests and parity assertions use documented tolerances (Parity-10: volume
±0.5% unless a tighter check is named). STEP reproducibility: stable modulo timestamp header;
`--reproducible` pins the header.

## Lifecycle / state machine

### Build

```
resolve target → load source + params + lock → cache lookup
  → (miss) eval Starlark → IR → kernel → validate → write artifact + meta
  → facts summary → JSON/human render
```

Failure at any stage: `ok: false`, diagnostics filled, non-zero exit (3/5/6…). No partial
artifact presented as success.

### HTTP job

```
POST /v1/jobs → pending → running → completed | failed
SSE /events streams progress
GET artifact by content hash after completed
```

Cancel where supported flips to `failed` with reason `cancelled`, never deletes spend evidence
for fab/printer jobs without an audit line.

### Printer

```
slice → gcode-check → dry-run upload → (human) confirm start → watch
```

Any skipped gate is a hard error (code 8), not a warning.

## Environment

| Var | Default | Purpose |
|-----|---------|---------|
| `CADRION_PROJECT` | cwd walk | Project root override (else `--project` / find `cadrion.toml`) |
| `CADRION_KERNEL` | `occt` | Backend id (`occt` \| `truck`) |
| `CADRION_ENGINE_DIR` | platform cache | OCCT/engine install location |
| `CADRION_VIEWER_PORT` | `7411` | Default viewer bind port |
| `CADRION_API_PORT` | `7410` | Default API bind port |
| `CADRION_API_TOKEN` | auto at start | Bearer token; never log full value |
| `CADRION_LOG` | `info` | `tracing` filter (stderr only) |
| `CADRION_CACHE` | `<project>/.cadrion` | Build/tess cache root |
| `CADRION_PARTS_CACHE` | user cache | Catalog STEP cache |
| `CADRION_NO_COLOR` | unset | Disable ANSI |

Flags win over env; env wins over `cadrion.toml`. Tokens and printer credentials: **0600 files /
env only**, never committed, never full-printed (lengths/heads only).

## Invariants

1. **Source + lock + Cadrion/kernel versions determine artifacts.** No hidden ambient inputs in model code.
2. **Primary artifact basename matches source** (`block.cad.star` → `block.step` beside it).
3. **No directory-wide build or scan mutation.** Explicit targets only.
4. **Stdout on MCP is JSON-RPC only.** All logs on stderr.
5. **`--json` is canonical**; human text is a rendering of the same structure.
6. **Safety gates cannot be defaulted away** on any surface.
7. **Schema drift is a CI failure** (CLI / MCP / OpenAPI one type layer).
8. **Clean-room:** no reference-project source in tree, as dependency, or as translation input.
9. **OCCT stays behind `GeomKernel`** and separate distribution — no smearing LGPL into core static link without charter amendment + legal review.
10. **Fake success is a bug.** Missing engine, missing slicer, lock mismatch, allow-list miss → structured failure.

## Honest degrades

| Condition | Behavior |
|-----------|----------|
| OCCT engine not installed | `cadrion build` fails with `CADRION-E-ENGINE-MISSING` + hint to `cadrion engine install`; never pretends truck parity |
| `truck` selected for parity path | Explicit non-parity warning in meta; parity-10 may skip or xfail loudly |
| Parts catalog unreachable | `parts search/fetch` → code 7; builds using lock miss fail closed |
| Slicer CLI not found | `fab slice` → structured miss listing discovery paths; no fake gcode |
| Printer not allow-listed / confirm missing | code 8 safety refuse |
| Snapshot backend/GPU unavailable | clear diagnostic; doctrine skip only if skill policy allows and reason recorded |
| Feature not in this milestone | `"not yet implemented"` / capability flag from `engine_info` — never empty success |

## Verification hooks (contract-level)

- Parity-10 deterministic suite (PRD §12) is the geometry acceptance gate.
- Agent harness scores loops-to-success; not a substitute for deterministic asserts.
- Library face: 30-line "bracket → STEP" example compiles in CI once `cadrion-lang` + kernel land.

## Open questions

See charter OQ-1…OQ-7. Design-level watches:

- Stdlib symbol names + IR float JSON: pinned H5-7 (`docs/DIALECT.md` + dialect goldens).
  Selector query grammar still open.
- Default viewer/API ports (7411/7410 above) — change only with schema + docs together.
- ~~Whether the facade remains public on crates.io~~ **Resolved (OQ-1 / H3-10):**
  product is **Cadrion**. First public install crate is `cadrion-cli`. See `docs/NAME_OQ1.md`.
