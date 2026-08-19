# PRD — **Cadrion**: a Rust-native CAD toolkit for AI agents

*Clean-room functional-conceptual equivalent of `earthtojake/text-to-cad` ("CAD Skills")*

| | |
|---|---|
| **Status** | Draft v0.1 |
| **Date** | 2026-07-28 |
| **Working name** | `cadrion` (CAD Runtime & Engine) — placeholder pending trademark / crates.io check |
| **Target license** | MIT OR Apache-2.0 (dual, Rust convention) |
| **Reference project** | https://github.com/earthtojake/text-to-cad (MIT, ~9.1k stars) |

---

## 0. Clean-room basis

This document specifies behavior, not implementation, and was written from the reference project's **publicly documented behavior only**: its README, per-skill `SKILL.md` documents, benchmark prompt descriptions, and public docs site. No Python or JavaScript source from the reference project may be translated, transliterated, or copied by implementers of this PRD. The reference project is MIT-licensed, so this posture is belt-and-braces rather than legally required — but it keeps Cadrion a genuinely independent implementation, free to be dual-licensed MIT/Apache-2.0 and to diverge where Rust idioms or agent ergonomics call for it.

Implementers should treat this PRD, not the reference repo, as their input. Where this PRD says "parity," it means *workflow and capability parity* (same jobs get done the same conceptual way), never API or source compatibility.

---

## 1. Background: what the reference project is

The reference project, despite its repo name, is not a single text→CAD model or service. It is a **library of agent skills** — Markdown workflow instructions plus local Python/JS tooling — that get installed into coding agents (Claude Code, Codex) via a skills CLI or plugin marketplaces. Once installed, the agent itself is the "text-to-CAD model": it reads the skill doctrine, writes parametric CAD source code, runs local scripts to generate/inspect/validate geometry, and iterates.

Its core design decisions, which Cadrion must preserve conceptually:

1. **STEP-first, source-of-truth-is-code.** The agent authors Python (build123d, an OpenCascade B-rep wrapper) files exposing a `gen_step()` entry point. STEP is the primary validated artifact; STL/3MF/GLB are secondary exports derived from it. Generated artifacts are never hand-edited — you edit source and regenerate.
2. **Deterministic self-verification.** CLI tools return geometry *facts* (bounding boxes, mass properties, topology), *measurements*, *alignment checks*, and *diffs*, addressable via stable selector tokens (e.g. `#o1.2.f1` for "object 1, solid 2, face 1"). The agent verifies its work numerically before ever looking at a picture.
3. **Mandatory visual review.** A snapshot tool renders PNG/GIF review packets; skill doctrine makes reviewing them non-optional after visible geometry changes, and a local browser viewer (STEP/STL/3MF/GLB/G-code/DXF/URDF/SRDF/SDF/implicit-SDF) provides live links for humans.
4. **An ecosystem around the core:** off-the-shelf part sourcing from a hosted STEP catalog (checksummed downloads); 2D DXF generation; robot description files (URDF, MoveIt2 SRDF, SDFormat) with validation; vendor DFM preflight (SendCutSend profile); slicing to G-code by orchestrating real slicer CLIs; cautious LAN handoff to Bambu Lab printers (FTPS/MQTT, dry-run first); experimental GLSL signed-distance-field "implicit CAD."
5. **A 10-part benchmark suite** (calibration block → planetary gear stage) with prompts and orbit-GIF outputs, used to demonstrate and regression-test agent capability.
6. **Prompt-ware is half the product.** The `SKILL.md` files encode modeling defaults (millimeters, XY base plane, +Z up, standard clearance-hole diameters, wall-thickness heuristics), workflow sequencing (brief → plan → code → generate → validate → snapshot → hand off), and prohibitions (don't diff binary artifacts with git; don't run directory-wide generation; ask at most one clarifying question). A port that ships tools without this doctrine is not functionally equivalent.

### 1.1 Why a Rust equivalent

| Pain in the reference architecture | Cadrion answer |
|---|---|
| Python environment per skill (interpreter versions, `requirements.txt`, OCP wheels ~hundreds of MB) | One static binary + optional downloadable kernel backend; `cadrion` works minutes after `curl` |
| Agent shells into scripts; ~seconds of interpreter/import startup per call | Persistent server modes (MCP/HTTP) with warm kernel and build cache; CLI startup < 50 ms |
| Arbitrary Python `exec` as the modeling surface — full ambient authority (network, filesystem) every time the agent "draws a box" | Hermetic, deterministic, sandboxed modeling language with zero ambient authority |
| Bash-only integration; MCP/HTTP not first-class | CLI, MCP, and HTTP API as co-equal, schema-published surfaces |
| Hard to embed in other software | `cadrion-*` crates usable as a library |

---

## 2. Product summary

Cadrion is a single binary (plus optional kernel backend) that gives AI agents a complete local hardware-design loop: **author parametric CAD as sandboxed code → build B-rep geometry → interrogate it numerically → review it visually → export, source parts, describe robots, and hand off to fabrication** — through three first-class surfaces (CLI, MCP server, HTTP API) and a generated **skill pack** that drops into Claude Code / Codex exactly the way the reference project does.

```
                       ┌────────────────────────────────────────────┐
  agent / human ──CLI──▶                                            │
  agent ────────MCP────▶   cadrion core                               │──▶ artifacts:
  app ──────────HTTP───▶   ┌──────────┐  ┌─────────┐  ┌─────────┐   │    .step .stl .3mf .glb
                       │   │ lang     │─▶│ model IR│─▶│ kernel  │   │    .dxf .urdf .srdf .sdf
  skill pack (SKILL.md)│   │(Starlark)│  │+selectors│ │(OCCT ▸  │   │    .gcode  .png/.gif
  teaches the workflow │   └──────────┘  └─────────┘  │ truck)  │   │    facts/measure JSON
                       │   inspect · snapshot · export· parts ·     │
                       │   robot · fab · viewer · bench             │
                       └────────────────────────────────────────────┘
```

Two conformance levels define "port or functional-conceptual equivalent":

* **L1 — Conceptual parity (required for 1.0):** every workflow in the parity matrix (Appendix A) is achievable with equivalent verbs, defaults, and validation rigor.
* **L2 — Drop-in agent compatibility (required for 1.0):** `cadrion skills export` produces a skill/plugin pack installable into Claude Code and Codex whose doctrine mirrors the reference structure (progressive references, mandatory snapshot review, viewer handoff), with tool invocations rewritten to `cadrion` commands.
* **Explicit non-goal:** source compatibility. build123d Python files do not run on Cadrion (see §16 open question on a migration assistant).

---

## 3. Goals and non-goals

**Goals**

* G1. Single-command install on macOS (arm64/x86_64), Linux (x86_64/aarch64), Windows (x86_64); no Python, Node, or system CAD packages required for the core loop.
* G2. Full reference-benchmark parity: all 10 canonical parts buildable and passing deterministic assertions (§12) with an agent driving Cadrion.
* G3. AI-first ergonomics: every command has `--json`; every error is structured with a code, span, and fix hint; MCP tool surface fits a small context budget (§11).
* G4. Deterministic, hermetic model evaluation: same source + same Cadrion version ⇒ byte-identical IR and equal-within-tolerance geometry, with no network or filesystem access from model code.
* G5. Sub-second warm iteration loop for simple parts (build + facts), enabling tight agent repair cycles.
* G6. Safe-by-default effectful operations: printer starts and vendor uploads are dry-run first and consent-gated at every surface.
* G7. Local-first and private: no telemetry, no required cloud; the only default network egress is the parts catalog when explicitly invoked.
* G8. Embeddable: core capabilities available as documented Rust crates.

**Non-goals (v1)**

* NG1. No 2D constraint solver or GUI sketcher — like the reference, this is code-CAD; parametricity lives in source parameters.
* NG2. No CAM toolpathing, FEA, rendering-quality raytracing, or BIM.
* NG3. No hosted multi-tenant SaaS (the HTTP API is a local/embedded server; hardening for public exposure is future work).
* NG4. No mesh sculpting/organic modeling; meshes are export targets and slicing inputs, not the modeling medium.
* NG5. No attempt to run build123d/ezdxf Python sources.

---

## 4. Users and primary scenarios

* **P1 — The coding agent (primary user).** Claude Code, Codex, or any MCP client. Needs unambiguous tools, machine-readable results, stable references, image-returning review, and doctrine it can follow.
* **P2 — The engineer/maker driving the agent.** Needs viewer links, sane defaults in mm, STEP files that open in Fusion/FreeCAD/SolidWorks, and confidence that nothing prints without consent.
* **P3 — The toolsmith.** Embeds `cadrion-kernel`/`cadrion-lang` in a Rust app, or fronts the HTTP API from a web product.

Representative scenarios (each maps to acceptance tests):

* S1. "Create a centered 100×60×20 mm block with four 8 mm through-holes and a 2 mm top chamfer" → agent writes `block.cad.star`, `cadrion build`, checks facts (volume, hole count), snapshots, returns STEP + viewer link.
* S2. Iterative repair: a fillet fails on a narrow edge → structured diagnostic names the offending selector and suggests reducing radius; agent patches one parameter and rebuilds.
* S3. Assembly with a purchased part: agent searches the parts catalog for a "MG996R servo," fetches the checksummed STEP into the project lockfile, mates it via named datums, validates alignment deltas.
* S4. Robot handoff: from the assembly, generate URDF with inertials computed from actual geometry + density, validate joint frames, add an SRDF planning group, preview articulated in the viewer.
* S5. Sheet-metal path: project a DXF from a face, run the vendor DFM rulepack (SendCutSend profile), get a preflight report with pass/warn/fail findings.
* S6. Print path: export 3MF → discover local slicer → slice with a printer/material profile → static-validate G-code → dry-run upload to a Bambu printer on LAN → human confirms → start.

---

## 5. Design principles (the AI-friendly contract)

* **P1. Machine-first I/O.** Human-pretty output is a rendering of the JSON, never the other way around. `cadrion schema` dumps all command/tool/API schemas.
* **P2. Verify numerically before visually.** Every capability that changes geometry has a corresponding interrogation (facts/measure/align/diff); snapshots complement, never replace, deterministic checks — and doctrine still makes snapshot review mandatory for visible changes.
* **P3. Text is the source of truth.** Artifacts are derived; source + lockfiles fully determine outputs; generator and artifact share a basename in the same directory.
* **P4. Hermetic determinism.** Model code cannot read the clock, environment, network, or filesystem; iteration order and float formatting are pinned; builds are cacheable by content hash.
* **P5. Small context surface.** Tool names, descriptions, and schemas are budgeted (§11) and progressive: deep guidance lives in skill-pack reference docs the agent loads on demand, mirroring the reference project's "progressive references" pattern.
* **P6. Explicit targets only.** Commands operate on named files/refs; nothing scans directories or mutates outside the stated target.
* **P7. Consent for consequences.** Anything that touches hardware or third-party services is dry-run by default and requires an explicit, non-defaultable confirmation at every surface (flag / MCP param / API field), plus printer allow-listing.
* **P8. One clarifying question max.** Shipped doctrine: proceed with documented assumptions unless the model is impossible, fit-critical, or safety-critical.

---

## 6. Architecture

Cargo workspace (crate boundaries are requirements; internal design is implementer's choice):

| Crate | Responsibility |
|---|---|
| `cadrion-kernel` | `GeomKernel` trait: topology model, boolean/feature ops, queries, tessellation, STEP I/O contract |
| `cadrion-occt` | Default kernel backend binding Open CASCADE (OCCT) via FFI; loadable/dynamically linked (§11 licensing) |
| `cadrion-truck` | Experimental pure-Rust backend (truck) behind the same trait; feature-gated, non-default |
| `cadrion-lang` | Starlark host, CAD stdlib, params, diagnostics, IR emission |
| `cadrion-model` | Feature IR, content hashing, selector scheme, build cache, artifact registry |
| `cadrion-inspect` | facts / planes / measure / align / frame / diff engines |
| `cadrion-render` | Offscreen wgpu tessellated rendering: multi-view PNG packets, orbit GIF/MP4 |
| `cadrion-export` | STEP AP242 write (+AP214 compat read), STL, 3MF, GLB, DXF writer |
| `cadrion-viewer` | Embedded local web viewer (static assets in binary): serves tessellated GLB + overlays for G-code/DXF/URDF/SRDF/SDF/implicit; file-watch reload; joint jog controls |
| `cadrion-parts` | `PartProvider` trait; hosted STEP-catalog client; cache + `parts.lock` with checksums |
| `cadrion-robot` | URDF/SRDF/SDF generation stdlib + validators; inertia-from-geometry |
| `cadrion-fab` | DFM rulepacks + vendor profiles; slicer discovery/orchestration; G-code static analysis; printer adapters (Bambu LAN FTPS/MQTT first) |
| `cadrion-mcp` | MCP server (official Rust SDK), stdio + streamable HTTP |
| `cadrion-api` | Axum HTTP API, jobs, SSE, OpenAPI generation |
| `cadrion-cli` | Clap front end; the `cadrion` binary |
| `cadrion-skills` | Skill-pack generator/exporter for Claude Code & Codex; bundled doctrine docs |

### 6.1 The modeling language (the central porting decision)

The reference's expressive power comes from the agent writing *real code* — loops, math, helper functions — in a language LLMs are fluent in. A declarative JSON/feature-tree cannot express benchmark parts like the impeller (12 swept blades from a formula) or spiral staircase without a programming layer, and embedding CPython would forfeit the static binary, sandboxing, and determinism that justify the port.

**Decision: Starlark** (Python-syntax configuration language; mature Rust implementation exists under Apache-2.0) **as the authoring language**, with a Cadrion CAD standard library. Rationale: LLMs already write it as "Python"; it is hermetic and deterministic by design (no I/O, no ambient authority, bounded); it evaluates in milliseconds; conceptual porting from build123d-style code is natural.

* Files: `<name>.cad.star` (parts/assemblies), `<name>.dxf.star`, `<name>.urdf.star`, `<name>.srdf.star`, `<name>.sdf.star`. Entry-point convention preserved from the reference: `gen_step()`, `gen_dxf()`, `gen_urdf()`, `gen_srdf()`, `gen_sdf()`.
* Evaluation emits a **feature IR** (the persisted, hashed, diffable representation); the kernel executes IR. Source is for authors; IR is for caching, `diff`, and future alternate front ends.
* Escape hatch (P2, post-1.0): WASM component models for users who want to author in other compiled languages against the IR.

Illustrative source (normative for flavor, not final API):

```python
# block.cad.star  —  built with: cadrion build block.cad.star
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

Stdlib scope for 1.0 (mirrors the reference capability envelope): primitives (box, cylinder, cone, sphere, torus, wedge); 2D sketch profiles (line/arc/circle/polyline/spline, offset, fillet2d) with extrude / revolve / sweep-along-path / loft / helix; booleans; fillet/chamfer with selector-based edge sets; shell/offset; hole features (simple, counterbore, countersink) with standard clearance tables; linear/polar/grid patterns; mirrors and transforms; text (engrave/emboss, P1); assemblies with named parts, part-local frames, named mating datums, joint definitions (rigid, revolute, prismatic, cylindrical, ball) and explicit placements; labels and metadata that survive into STEP product structure.

### 6.2 Kernel abstraction

`GeomKernel` must support: exact B-rep solids; the feature ops above; mass properties (volume, area, centroid, inertia tensor at density); bounding boxes (axis-aligned and oriented); topology iteration with stable ordering; closest-distance and ray/section queries; sew/heal and validity checks (closed, positive volume, no self-intersection); tessellation with chord/angle tolerances; STEP AP242 write and AP242/AP214 read preserving assembly structure and labels. **Default backend: OCCT** — it is the only realistic route to reference-grade fillets, booleans, and STEP fidelity; it is also what powers the reference stack, making behavior comparisons meaningful. `cadrion-truck` exists to keep the trait honest and to seed an eventual all-Rust future, and is excluded from parity claims.

### 6.3 Stable selectors

A deterministic addressing scheme functionally equivalent to the reference's `#o1.2.f1` tokens: `#o<obj>[.<solid>][.f<face>|.e<edge>|.v<vertex>]`, with documented, kernel-independent ordering rules (sort by centroid/area tuple with tie-breakers) so the same source yields the same tokens across runs and platforms. `inspect refs` lists tokens with kind, area/length, centroid, and normal; `diff` reports token remaps between builds ("`#o1.1.f5` → `#o1.1.f7`") so agents can heal references after topology changes. Selector queries (`faces(">Z")`, `edges(parallel=Z)`, `largest(faces(...))`) are provided in-language; tokens are for CLI/tool addressing.

---

## 7. Functional requirements

Priorities: **P0** = 1.0 blocker, **P1** = 1.0 target, **P2** = post-1.0. Each FR implies `--json` output and structured errors (§10).

### 7.1 Modeling & build (FR-1xx)

* **FR-101 (P0).** `cadrion build <target.star> [-o out]` evaluates the source hermetically, executes the IR on the kernel, and writes the primary artifact next to the source with the same basename (`block.cad.star` → `block.step`). Directory-wide builds are refused.
* **FR-102 (P0).** Build results include: artifact path(s), content hashes, validity report (closed/positive-volume/shell count), facts summary, wall time, and diagnostics.
* **FR-103 (P0).** Parameter overrides at build time (`--set width=120`) without editing source; overrides recorded in build metadata.
* **FR-104 (P0).** Deterministic evaluation: no clock/env/net/fs access from model code; bounded execution (configurable fuel/time/memory caps) with a clear diagnostic on exhaustion.
* **FR-105 (P0).** Build cache keyed by (source hash, params, cadrion+kernel versions); warm no-op rebuilds return in < 100 ms.
* **FR-106 (P0).** Assemblies: named components, per-part frames, named datums, joints, explicit placements; STEP export preserves product structure and labels.
* **FR-107 (P1).** `cadrion watch <target>` rebuilds on change and pushes to the viewer.
* **FR-108 (P1).** Direct STEP/STP import as a first-class target (`--kind part|assembly`) for inspection/export when no generator exists, at parity with the reference's dual entry paths.
* **FR-109 (P2).** Animation/parameter sidecars for the viewer (named parameter sweeps).

### 7.2 Inspection & validation (FR-2xx)

* **FR-201 (P0).** `inspect refs <target> [--facts --planes --positioning]`: selector inventory + geometry facts (bbox, volume, area, centroid, mass@density, solid/face/edge counts, principal planes, part frames/datums for assemblies).
* **FR-202 (P0).** `inspect measure <target> <refA> [refB] [--kind distance|angle|diameter|thickness]`: numeric result with the measurement construction described.
* **FR-203 (P0).** `inspect align <assembly> --a <ref> --b <ref> [--expect coplanar|coaxial|distance=X --tol T]`: mating-delta report (translation and angular error) with pass/fail against tolerance.
* **FR-204 (P0).** `inspect diff <old> <new>`: changed volume/area/bbox, added/removed solids, selector remap table; explicitly the sanctioned alternative to git-diffing binary artifacts.
* **FR-205 (P1).** `inspect frame <target> <ref>`: local frame (origin + axes) for a face/datum, for downstream URDF/joint work.
* **FR-206 (P1).** Section queries: area/profile of a planar cut at a given plane (supports wall-thickness style checks).
* **FR-207 (P1).** Watertightness/manifold and minimum-feature scans usable standalone (shared with fab checks).

### 7.3 Snapshots & rendering (FR-3xx)

* **FR-301 (P0).** `cadrion snapshot <target> [--views iso,front,top,right] [--focus <ref>] [--out dir]`: headless PNG packet with consistent framing, edges overlaid, and a scale/annotation footer; deterministic camera derivation from bbox.
* **FR-302 (P0).** Orbit turntable GIF (and MP4, P1) generation — the benchmark demo format.
* **FR-303 (P0).** MCP/API return snapshots as image content, not just paths.
* **FR-304 (P1).** Focus/annotation modes: highlight a selector set, exploded assemblies, section view.
* **FR-305 (P0, doctrine).** Skill pack encodes the mandatory-snapshot-review policy and its only sanctioned skip cases (no visible change / no valid artifact), with the skip reason reported.

### 7.4 Import/export (FR-4xx)

* **FR-401 (P0).** STEP AP242 export (assembly structure, labels, units); AP214/AP242 import.
* **FR-402 (P0).** STL (binary) and GLB export from tessellation with stated tolerances; 3MF export (P0 for single body, P1 for multi-part with names).
* **FR-403 (P0).** Exports are secondary: derived from the STEP-first pipeline, recorded with provenance (source hash, tolerances).
* **FR-404 (P1).** DXF writer (R2018 text) for `gen_dxf()` sources and for projections of planar faces/silhouettes from 3D targets (`cadrion export --dxf --face <ref>`); layer/units conventions documented.
* **FR-405 (P2).** OBJ/PLY export; glTF instancing for patterned parts.

### 7.5 Viewer (FR-5xx)

* **FR-501 (P0).** `cadrion view [path…] [--port]` serves an embedded local web viewer; returns per-file deep links; reuses a running instance rather than spawning duplicates.
* **FR-502 (P0).** Renders STEP (server-tessellated to GLB), STL/3MF/GLB, and snapshot packets; live-reloads on rebuild.
* **FR-503 (P1).** G-code preview (layer scrub, travel/extrude coloring), DXF 2D view, URDF/SDF scene view with joint jog sliders driven by the validated model.
* **FR-504 (P1).** Selector picking: clicking a face shows its `#…` token, closing the loop between visual and numeric addressing.
* **FR-505 (P2).** Implicit-SDF raymarch view (§7.9); SRDF group visualization.

### 7.6 Parts sourcing (FR-6xx)

* **FR-601 (P0).** `PartProvider` trait: search(query) → candidates with names, standards/aliases, key dimensions; fetch(id) → STEP + checksum + license/metadata.
* **FR-602 (P0).** Built-in provider for a hosted STEP-parts catalog (the reference ecosystem's catalog first, via its public API; additional providers pluggable). All fetches checksum-verified and cached.
* **FR-603 (P0).** Project `parts.lock`: pinned part IDs, versions, checksums; builds referencing catalog parts fail closed if the lock is missing/mismatched.
* **FR-604 (P1).** Doctrine + tooling for the reference's rule: before modeling a named purchasable component, search the catalog; on miss, record the miss and model a documented envelope.
* **FR-605 (P2).** Local/vendored part libraries as a provider; offline mirror support.

### 7.7 Robot descriptions (FR-7xx)

* **FR-701 (P0).** `gen_urdf()` stdlib: links, joints (types, axes, limits, dynamics), visual/collision geometry referencing exported meshes with correct scale/units, materials; emits well-formed URDF.
* **FR-702 (P0).** Inertial computation from actual CAD geometry + material density (mass, COM, inertia tensor in the link frame), with override support — a validated-by-construction improvement on hand-typed inertials.
* **FR-703 (P0).** URDF validator: XML schema, kinematic-tree integrity (single root, no cycles), joint-axis normalization, frame/unit consistency, mesh existence + scale sanity, inertial plausibility (positive-definite, triangle inequality).
* **FR-704 (P1).** `gen_srdf()` + validator: planning groups, end effectors, group states, virtual/passive joints, disabled-collision pairs; cross-checked against the paired URDF.
* **FR-705 (P1).** `gen_sdf()` + validator: SDFormat models/worlds (frames, poses, links, joints, inertials, sensors, lights, physics, includes); mesh URI resolution checks.
* **FR-706 (P1).** Consistency suite: URDF↔source drift detection (regenerate-and-diff), URDF↔SRDF joint/link name agreement.
* **FR-707 (P2).** Viewer FK preview with joint sliders (pairs with FR-503); IK/MoveIt2 integration is out of scope for v1.

### 7.8 Fabrication (FR-8xx)

* **FR-801 (P0).** DFM engine: rulepacks evaluated against DXF/STEP targets, producing an evidence-backed preflight report (pass/warn/fail per rule, with measurements and selector/entity references). Conservative by design: findings must cite the check that ran.
* **FR-802 (P0).** First vendor profile: SendCutSend-style laser/CNC/bending checks (material+thickness availability from a profile file, min hole size vs thickness, min bridge/web width, bend-relief and flange-length rules, tapping/countersink/hardware constraints, flat-pattern closure). Profiles are data (TOML/JSON), community-updatable, and versioned — Cadrion never claims live vendor truth, only profile-version truth.
* **FR-803 (P0).** Slicer orchestration: discover local PrusaSlicer/OrcaSlicer/Bambu Studio CLIs; `cadrion fab slice <mesh> --slicer … --printer-profile … --filament …` runs the real slicer, captures logs, and returns the `.gcode` + summary (est. time, filament, bbox). Cadrion does not reimplement slicing.
* **FR-804 (P0).** G-code static validation: parseability, flavor detection, printable-volume bounds vs printer profile, temperature/first-move sanity, no vendor-proprietary opcodes for the plain-gcode path.
* **FR-805 (P0).** Printer adapters behind a `Printer` trait; first adapter Bambu Lab LAN mode (FTPS upload, MQTT status/control). `printer status`, `printer upload`, `printer dry-run` are read-only/safe; **`printer start` requires**: validated G-code hash matching the uploaded file, a printer allow-list entry in config, and an explicit confirmation input at the invoking surface (CLI `--confirm-start`, MCP `confirm: "START"` param, API `"confirm": true` + capability token). No surface may default this.
* **FR-806 (P1).** Job monitoring: progress, pause/cancel, camera-frame fetch where the protocol allows.
* **FR-807 (P2).** Additional printer adapters (Klipper/Moonraker, OctoPrint); CNC vendor profiles beyond the first.

### 7.9 Implicit CAD (FR-9xx, experimental)

* **FR-901 (P2).** Author signed-distance-field models in the same Starlark surface (SDF primitive/CSG/smooth-op combinators, TPMS lattices); render via wgpu raymarch headless and in the viewer.
* **FR-902 (P2).** Mesh extraction from SDFs (marching-cubes class) for the slicing path; clearly flagged as approximate, never a STEP substitute — mirroring the reference's "prefer STEP-first unless explicitly asked" stance.

---

## 8. Interfaces

### 8.1 CLI

Single binary, verb-grouped. Global flags: `--json`, `--quiet`, `--project <dir>`, `--kernel <backend>`, `--no-color`.

| Command | Purpose |
|---|---|
| `cadrion init` | Scaffold project layout + config |
| `cadrion build <target.star \| .step> [--set k=v] [-o]` | Build primary artifact (FR-101…) |
| `cadrion watch <target>` | Rebuild-on-change + viewer push |
| `cadrion inspect refs\|measure\|align\|frame\|diff …` | Numeric interrogation (FR-2xx) |
| `cadrion snapshot <target> [--views…] [--gif]` | Visual review packets (FR-3xx) |
| `cadrion export <target> --format step\|stl\|3mf\|glb\|dxf [--face <ref>]` | Secondary exports (FR-4xx) |
| `cadrion view [paths…]` | Start/reuse local viewer, print links |
| `cadrion parts search\|show\|fetch\|lock …` | Catalog sourcing (FR-6xx) |
| `cadrion robot validate <file.urdf\|.srdf\|.sdf> [--against <pair>]` | Robotics validators (FR-7xx) |
| `cadrion fab check <dxf\|step> --profile <vendor@ver>` | DFM preflight (FR-801/802) |
| `cadrion fab slicers` / `cadrion fab slice …` / `cadrion fab gcode-check <file>` | Slicing path (FR-803/804) |
| `cadrion printer status\|upload\|dry-run\|start\|watch …` | Printer handoff (FR-805/806, gated) |
| `cadrion serve api [--port] [--token …]` | HTTP API server |
| `cadrion mcp [--http --port]` | MCP server (stdio default) |
| `cadrion skills export --agent claude-code\|codex [--out]` / `cadrion skills install` | L2 skill packs |
| `cadrion bench run [--suite parity10]` / `cadrion bench agent --cmd …` | Verification suites (§12) |
| `cadrion engine install\|info` | Fetch/inspect kernel backend (§11 licensing/distribution) |
| `cadrion schema [command\|mcp\|api]` | Machine-readable self-description |

Exit codes (stable, documented): `0` ok · `2` usage · `3` evaluation/build error · `4` validation failed · `5` kernel operation failed · `6` I/O · `7` provider/network · `8` safety gate refused · `9` internal. Agents may branch on them.

### 8.2 MCP server

* Transports: stdio (default for local agents) and streamable HTTP (P1) via the official Rust MCP SDK. Filesystem access confined to declared roots/project dir.
* Long operations report MCP progress notifications; snapshot/build tools return image content blocks inline alongside JSON.
* Tool descriptions + schemas total ≤ **4,000 tokens** (NFR-7); deep workflow guidance lives in the skill pack, not tool descriptions.
* Destructive tools follow FR-805 gating; `printer_start` is also excluded from any auto-approval hints.

Tool catalog (names final, schemas in `cadrion schema mcp`; Appendix B sketches three):

| Tool | Purpose |
|---|---|
| `build` | Build a `.star`/STEP target; returns artifacts, facts, diagnostics |
| `write_source` / `read_source` | Project-scoped source file I/O (for remote/HTTP-backed agents; local agents may use their own fs tools) |
| `inspect_refs`, `measure`, `align_check`, `frame`, `diff` | FR-2xx interrogation |
| `snapshot` | PNG/GIF packet, returned as images |
| `export` | Secondary format export |
| `viewer_open` | Ensure viewer running; return deep links |
| `parts_search`, `parts_fetch` | Catalog sourcing with lockfile updates |
| `robot_validate` | URDF/SRDF/SDF validation (+cross-checks) |
| `fab_check`, `fab_slice`, `gcode_check` | Fabrication path |
| `printer_status`, `printer_upload`, `printer_dry_run`, `printer_start` | Printer handoff (last one gated) |
| `project_artifacts` | List artifacts + hashes + provenance |
| `engine_info` | Versions, kernel backend, capability flags |

Resources: `cadrion://project/**` sources, `cadrion://artifact/**` outputs, `cadrion://doc/**` doctrine pages (so MCP-only agents can progressively load the same references the skill pack ships).

### 8.3 HTTP API

* Local/embedded Axum server; bearer-token auth (auto-generated, printed at start); binds loopback by default.
* Sync endpoints for fast ops; async **jobs** (`POST /v1/jobs` with kind=build|snapshot|slice|…, `GET /v1/jobs/{id}`, `GET /v1/jobs/{id}/events` as SSE) for the rest; content-addressed artifact store (`GET /v1/artifacts/{hash}`).
* Endpoint families mirror MCP tools 1:1 (`/v1/build`, `/v1/inspect/*`, `/v1/snapshot`, `/v1/export`, `/v1/parts/*`, `/v1/robot/validate`, `/v1/fab/*`, `/v1/printers/*`); OpenAPI 3.1 served at `/v1/openapi.json` and generated from the same schema source as `cadrion schema` (single source of truth, NFR-8).
* Versioned under `/v1`; additive-only within a major.

### 8.4 Skill packs & agent doctrine (L2)

`cadrion skills export` emits an installable pack per agent ecosystem containing: a core CAD skill (workflow doctrine: classify → brief → plan → author source → build explicit targets → validate numerically → mandatory snapshot review → viewer handoff → report only checks that ran), progressive reference docs (modeling patterns, positioning/datums, validation sequences, repair loop, export workflows), per-domain skills (viewer, parts, dxf, urdf, srdf, sdf, vendor-preflight, gcode, printer, implicit), and the modeling defaults table (mm, XY base plane, +Z up, clearance-hole sizes, wall/fillet heuristics). Content is original prose implementing the same doctrine, not copied text. Packs must be installable via the agents' native plugin/marketplace mechanisms and via a plain directory drop.

### 8.5 Library surface

`cadrion-kernel`, `cadrion-lang`, `cadrion-model`, `cadrion-inspect`, `cadrion-export` published with semver and docs.rs coverage ≥ 90% of public items; a 30-line "build a bracket and export STEP" example compiles in CI.

---

## 9. Project & data conventions

```
myproject/
  cadrion.toml            # config: kernel, viewer port, printers allow-list, providers
  parts.lock            # pinned catalog parts (checksums)
  cad/bracket.cad.star  # source …
  cad/bracket.step      # … and its artifact, same basename, same dir
  cad/bracket.snap/     # snapshot packets
  robots/arm.urdf.star  → robots/arm.urdf
  .cadrion/               # build cache, tessellation cache, logs (gitignored)
```

Paths in commands resolve from the invoking CWD, never from install/skill directories. Config precedence: flags > env (`CADRION_*`) > project `cadrion.toml` > user config. No telemetry keys exist.

---

## 10. Error model

Every failure is a diagnostic object: `{ code: "CADRION-E-FILLET-RADIUS", severity, message, target, span: {file,line,col}, refs: ["#o1.1.e12"], hint, docs_url }`. Requirements: codes are stable and enumerable (`cadrion schema errors`); kernel failures are translated into feature-level terms (which op, which selector, plausible causes) rather than raw kernel traces; hints are actionable ("reduce radius below 2.4 mm, the minimum adjacent edge length"). Diagnostic quality is a benchmarked feature: the agent-repair benchmark (§12) measures loop count to fix seeded errors.

---

## 11. Non-functional requirements

* **NFR-1 Performance.** Warm build+facts of benchmark part 1 ≤ 1 s p95 on a 2023 laptop; cold ≤ 4 s; CLI startup ≤ 50 ms; server tool overhead ≤ 100 ms; snapshot packet ≤ 2 s; viewer first render ≤ 2 s for a 50 MB STEP.
* **NFR-2 Footprint.** Core binary ≤ 40 MB; OCCT backend as a separately fetched component (`cadrion engine install`, checksummed) ≤ 150 MB installed.
* **NFR-3 Platforms.** macOS arm64/x86_64, Linux x86_64/aarch64 (glibc + musl core), Windows x86_64. CI builds and runs the parity suite on all.
* **NFR-4 Determinism.** Same inputs ⇒ identical IR bytes and facts within documented float tolerances across platforms; STEP output stable modulo timestamp header (which is pinned under `--reproducible`).
* **NFR-5 Security.** Model evaluation sandboxed (no I/O, fueled); STEP/DXF/G-code importers fuzzed in CI; network egress only to configured providers/printers; printer allow-list + consent gates (FR-805); API loopback+token by default.
* **NFR-6 Licensing.** Core dual MIT/Apache-2.0. OCCT is LGPL-2.1 with the OCCT exception: ship it dynamically linked / as the separately-distributed engine component, keep the `GeomKernel` boundary clean, and obtain legal review of the distribution story before 1.0. Skill-pack prose and vendor profiles are original works.
* **NFR-7 Context budget.** MCP tool surface ≤ 4,000 tokens; each CLI `--help` ≤ 120 lines; skill-pack core doc ≤ ~1,500 words with progressive references beyond that.
* **NFR-8 Single schema source.** CLI JSON, MCP schemas, and OpenAPI are generated from one Rust type layer; drift is a CI failure.
* **NFR-9 No telemetry.** None. A `cadrion bench` opt-in flag may print a shareable local report; nothing phones home.

---

## 12. Verification & success metrics

**Tiers**

1. Kernel/feature unit tests with golden mass-property values.
2. Golden-IR tests: source → IR snapshots reviewed in PRs.
3. **Parity-10 suite**: ten canonical parts mirroring the reference benchmark categories (calibration block with holes+chamfer; bolt-circle flange; gusseted L-bracket with two hole directions; stepped shaft with keyway; open-top enclosure with bosses; clevis bracket with lightening cutouts; finned cylinder with angled boss; backward-curved impeller; spiral staircase with helical rail; simplified planetary stage). Each has a reference `.cad.star`, deterministic assertions (volume ±0.5%, solid/hole counts via topology, bbox, key measurements, alignment checks), and an orbit-GIF artifact. Runs in CI on every platform.
4. **Agent-in-the-loop harness** (`cadrion bench agent`): drives a configurable agent (any MCP client or CLI-scripted model) against the Parity-10 *prompts* (natural language only), scoring first-try success, loops-to-success, wall time, and token cost. This is the number that answers "is it actually good for AI?"
5. Repair benchmark: seeded failure corpus (impossible fillet, non-manifold boolean, bad joint axis, out-of-volume slice) measuring diagnostic-driven loops-to-fix.
6. Interop: exported STEP opens with correct structure in FreeCAD and at least one commercial CAD (manual gate per release); URDF loads in a standard ROS 2 parser; sliced G-code accepted by target printer dry-run.

**1.0 exit metrics**

| Metric | Target |
|---|---|
| Parity-10 deterministic suite | 10/10 on all Tier-1 platforms |
| Agent harness (frontier model, MCP) | ≥ 8/10 tasks ≤ 3 loops |
| Repair benchmark | median ≤ 2 loops |
| Warm iteration (build+facts, part 1) | ≤ 1 s p95 |
| Install-to-first-STEP (fresh machine) | ≤ 5 min including engine fetch |
| MCP tool surface | ≤ 4,000 tokens |

---

## 13. Milestones

| Phase | Scope | Exit criteria |
|---|---|---|
| **M0 — Kernel spike** (4–6 wk) | OCCT binding eval (build/vendor strategy per platform), `GeomKernel` v0, box+cylinder+boolean+fillet+STEP write, Starlark host PoC | Benchmark part 1 built end-to-end from `.star`; go/no-go on binding approach |
| **M1 — Core loop** | Full stdlib for parts, IR+cache, `build`/`inspect refs\|measure`/`export step\|stl\|glb`, selectors, diagnostics v1 | Parts 1–4 pass deterministic suite; CLI `--json` everywhere |
| **M2 — See & serve** | `snapshot` (+GIF), viewer alpha, `diff`/`align`/`frame`, 3MF, MCP stdio server, skill-pack alpha | Parts 5–8 pass; agent completes part 1 via MCP with snapshot review |
| **M3 — Assemble & source** | Assemblies/joints/datums, parts providers + lockfile, HTTP API + jobs/SSE + OpenAPI, `watch` | Parts 9–10 + one assembly scenario (S3); agent harness ≥ 6/10 |
| **M4 — Robots** | URDF gen+validate+inertials, SRDF, SDF, consistency checks, viewer joint jog | S4 end-to-end; URDF loads in ROS 2 parser |
| **M5 — Fabricate** | DXF writer+projection, DFM engine + first vendor profile, slicer orchestration, gcode-check, Bambu adapter with gates | S5 & S6 end-to-end with human confirmation; safety-gate tests pass |
| **M6 — 1.0 hardening** | Windows parity, fuzzing, docs, skills export for both ecosystems, streamable-HTTP MCP, licensing review, metrics table green | §12 exit metrics met |

---

## 14. Risks & mitigations

| Risk | Impact | Mitigation |
|---|---|---|
| OCCT FFI maintenance burden / build complexity per platform | Schedule, stability | Isolate at `GeomKernel`; prebuilt engine artifacts via `cadrion engine install`; M0 go/no-go includes a vendored-binding fallback plan |
| Fillet/boolean robustness on hard cases (impeller, staircase) | Parity misses | Curate failure corpus early; expose kernel tolerances; document diagnostic-guided workarounds; treat as Parity-10 gates from M2 |
| LLMs less fluent authoring the Cadrion stdlib than build123d | Agent scores lag | Python-shaped Starlark; rich few-shot examples in skill pack; error hints name the correct API; agent harness tracked from M2, stdlib adjusted empirically |
| LGPL/OCCT distribution missteps | Legal/licensing | NFR-6: dynamic/separate engine component, clean boundary, pre-1.0 legal review |
| Bambu LAN protocol drift (reverse-engineered) | Feature breakage | Adapter behind trait; version-pinned protocol module; dry-run path always safe; clear "community-maintained" labeling |
| Hosted parts-catalog API terms/availability for a third-party client | FR-602 blocked | Provider trait keeps core independent; engage catalog maintainer early; local-library provider as fallback |
| Scope creep toward a GUI CAD app | Focus | NG list enforced; viewer stays review-only |

---

## 15. Out of scope / future directions

Constraint-solver sketching; CAM; FEA hooks; cloud/multi-tenant service with authn/z; WASM authoring components; pure-Rust kernel promotion to default; additional vendor DFM profiles and printer adapters; STEP PMI/GD&T; drawing sheets (title blocks, dimensioned views).

## 16. Open questions

1. Final product/binary name and crate namespace (trademark + crates.io sweep).
2. Starlark dialect details: float formatting guarantees, module system for shared part libraries, stdlib naming review with LLM-authoring A/B tests.
3. Migration assistant: a best-effort `cadrion migrate <build123d.py>` that emits a *skeleton* `.cad.star` (structure + parameters, human/agent completes it) — valuable, but must be built without reading reference source (train on build123d's own public docs only). Include in M6 or defer?
4. Depth of the assembly joint model in STEP output (kinematic AP242 vs labels+placements only) for 1.0.
5. Whether `write_source`/`read_source` MCP tools are enabled by default for local stdio agents or reserved for HTTP mode.
6. Vendor-profile governance: who signs off community updates to DFM rulepacks, and how are profile versions surfaced in reports?

---

## Appendix A — Parity matrix

| Reference skill | Essence | Cadrion equivalent | Phase |
|---|---|---|---|
| CAD | build123d `gen_step()` sources; STEP-first; `step`/`inspect`/`snapshot` tools; selectors; assemblies+joints; defaults doctrine | `cadrion-lang` `.cad.star` + `build`/`inspect`/`snapshot`; selector scheme; assemblies (FR-106); doctrine in skill pack | M1–M3 |
| CAD Viewer | Local browser review for CAD/G-code/robot files; live links; reuse running instance | `cadrion view` embedded viewer (FR-5xx) | M2–M4 |
| step.parts | Hosted catalog search/fetch of purchasable STEP parts, checksums, envelope-on-miss doctrine | `cadrion parts` + `PartProvider` + `parts.lock` (FR-6xx) | M3 |
| DXF | ezdxf `gen_dxf()` 2D drawings, flat patterns, cut layouts; projections from CAD | `gen_dxf()` Starlark + DXF writer + face projection (FR-404) | M5 |
| URDF | `gen_urdf()` + generation-time validation; frames/inertials as first-class risks | `cadrion-robot` URDF gen/validate + inertia-from-geometry (FR-701–703) | M4 |
| SRDF | MoveIt2 planning semantics atop a valid URDF | `gen_srdf()` + cross-validation (FR-704) | M4 |
| SDF | SDFormat models/worlds + simulator handoff | `gen_sdf()` + validator (FR-705) | M4 |
| SendCutSend | Conservative, evidence-backed vendor preflight for DXF/STEP | DFM engine + versioned vendor profiles (FR-801/802) | M5 |
| G-code | Slice meshes via real slicer CLIs; printer-agnostic; static validation | Slicer orchestration + `gcode-check` (FR-803/804) | M5 |
| Bambu Labs | LAN FTPS/MQTT dry-run/upload/cautious start | `Printer` trait + Bambu adapter + consent gates (FR-805/806) | M5 |
| Implicit CAD | Browser GLSL SDF models, raymarched; experimental, STEP-first preferred | SDF combinators + wgpu raymarch + mesh extraction (FR-9xx) | post-1.0 |
| Skills/plugin distribution | `SKILL.md` packs installed into Claude Code/Codex marketplaces | `cadrion skills export/install` (L2, §8.4) | M2–M6 |
| Benchmarks | 10 canonical prompts + orbit GIFs | Parity-10 + agent harness (§12) | M1→ |

## Appendix B — MCP tool sketches (illustrative)

```jsonc
// build
{ "name": "build",
  "input": { "target": "cad/bracket.cad.star", "set": {"width": 120}, "snapshot": false },
  "result": { "ok": true,
    "artifacts": [{"path": "cad/bracket.step", "sha256": "…", "kind": "step"}],
    "facts": {"bbox_mm": [120,60,20], "volume_mm3": 137904.2, "solids": 1, "faces": 22},
    "diagnostics": [] } }

// measure
{ "name": "measure",
  "input": { "target": "cad/bracket.step", "a": "#o1.1.f3", "b": "#o1.1.f9", "kind": "distance" },
  "result": { "value_mm": 60.0, "construction": "min distance between parallel planar faces" } }

// printer_start (gated)
{ "name": "printer_start",
  "input": { "printer": "bambu:X1C-01", "gcode_sha256": "…", "confirm": "START" },
  "result": { "ok": true, "job_id": "…", "note": "refused unless printer allow-listed, hash matches last dry-run upload, and confirm == \"START\"" } }
```

## Appendix C — Example agent turn (CLI surface)

```
$ cadrion build cad/flange.cad.star --json
{"ok":false,"diagnostics":[{"code":"CADRION-E-FILLET-RADIUS","span":{"file":"cad/flange.cad.star","line":14},
  "refs":["#o1.1.e7"],"message":"fillet r=3.0 exceeds adjacent edge length 2.4 on #o1.1.e7",
  "hint":"reduce radius below 2.4 mm or exclude edge #o1.1.e7 from the set"}]}
$ # agent edits line 14: fillet_r = 2.0
$ cadrion build cad/flange.cad.star --json          # ok:true, facts attached
$ cadrion inspect measure cad/flange.step '#o1.2' --kind diameter --json   # 30.0
$ cadrion snapshot cad/flange.step --views iso,front --gif
$ cadrion view cad/flange.step                       # → http://127.0.0.1:7411/f/flange.step
```

## References (public sources this spec was written from)

* Reference repo & README: https://github.com/earthtojake/text-to-cad
* Reference skill docs (behavioral basis): `skills/{cad,cad-viewer,step-parts,dxf,urdf,srdf,sdf,sendcutsend,gcode,bambu-labs,implicit-cad}/SKILL.md` in the above repo; docs site https://www.cadskills.xyz
* Model Context Protocol spec & official Rust SDK: https://modelcontextprotocol.io · https://github.com/modelcontextprotocol/rust-sdk
* Open CASCADE Technology (kernel candidate): https://dev.opencascade.org · truck (pure-Rust candidate): https://github.com/ricosjp/truck
* Starlark language & Rust implementation: https://github.com/bazelbuild/starlark · https://github.com/facebook/starlark-rust
