# Cadre-RS — charter

> **The decisions log below is BINDING.** Amend it with a dated entry; never silently.
> Where this document and the code disagree, one of them is a bug — say which.
> Where a later doc and D1–Dn disagree, **D1–Dn win**.

## What this is

Cadre is a Rust-native CAD runtime for AI agents: hermetic Starlark source → B-rep kernel →
numeric inspect → visual snapshot → export / parts / robots / fab handoff. One workspace,
three co-equal surfaces (CLI, MCP, local HTTP), plus a generated skill pack that teaches the
workflow. Clean-room functional-conceptual equivalent of `earthtojake/text-to-cad` — parity of
jobs and doctrine, never of source or API.

## What it is not

- Not a GUI CAD app, sketcher, or constraint solver — parametricity lives in source.
- Not a build123d / OpenCascade Python runtime — no CPython, no running reference sources.
- Not CAM, FEA, BIM, mesh sculpting, or organic modeling as the authoring medium.
- Not a multi-tenant hosted SaaS — HTTP is local/embedded; public hardening is future work.
- Not a slicer reimplementation — Cadre orchestrates real slicer CLIs and validates G-code.
- Not a pure-Rust-only stack in v1 — default kernel is OCCT (LGPL + exception), fetched as a
  separate engine component; pure-Rust `truck` is experimental and non-parity.
- Not telemetry-bearing and not auto-printing — hardware and third-party effects are
  dry-run-first and consent-gated at every surface.

## Decisions

Numbered, binding, dated. One decision per entry, with the reason — a decision whose
rationale is lost gets re-litigated within a month.

- **D1 — Clean-room of the reference, not a port of its source.** Implement from this
  repo's PRD (`docs/cadre-prd.md`) and public reference *behavior* only. No translating
  Python/JS from `earthtojake/text-to-cad`. Rules out source compatibility and dual-maintenance
  of a Python surface.
- **D2 — Starlark is the authoring language.** Hermetic, deterministic, LLM-fluent
  Python-shaped syntax without ambient authority. Rules out JSON-only feature trees (too weak
  for impeller/staircase benchmarks) and embedded CPython (forfeits static binary + sandbox).
- **D3 — STEP-first; source is truth; artifacts are derived.** Primary artifact is STEP next
  to source with the same basename. STL/3MF/GLB/DXF are secondary with provenance. Never
  hand-edit generated geometry — edit source and rebuild.
- **D4 — Default kernel is OCCT behind `GeomKernel`; truck is experimental.** Only realistic
  path to reference-grade fillets/booleans/STEP. Sanctioned C/FFI exception to the house
  "pure Rust" preference. OCCT ships as a separately fetched engine (`cadre engine install`),
  not statically linked into the core binary. Truck never carries parity claims.
- **D5 — Three co-equal faces + skill pack.** CLI (`cadre`), MCP (`cadre mcp`), local HTTP
  (`cadre serve api`) share one schema source (`cadre schema`). L2 requires installable skill
  packs for Claude Code and Codex. Standalone-first: no ApexOS ownership assumed.
- **D6 — Dual license MIT OR Apache-2.0 for core.** Rust convention; skill-pack prose and
  vendor profiles are original works. OCCT remains LGPL-2.1 + OCCT exception via dynamic /
  separate engine distribution (legal review before 1.0 — NFR-6).
- **D7 — Machine-first I/O and stable diagnostics.** Every command has `--json`. Failures are
  structured `{code, severity, message, target, span, refs, hint}` with enumerable codes.
  Exit codes 0/2–9 are stable for agent branching.
- **D8 — Numeric verify before visual; snapshots still mandatory in doctrine.** facts /
  measure / align / diff are the repair loop; snapshots complement and skill doctrine makes
  review non-optional after visible geometry changes (sanctioned skips only).
- **D9 — Hermetic model evaluation.** Model code: no clock, env, network, or filesystem;
  fueled/time/memory caps; builds cache by content hash. Same source + Cadre version ⇒
  identical IR and equal-within-tolerance geometry.
- **D10 — Consent for consequences.** Printer start, vendor upload, and any hardware /
  third-party effect: dry-run default, allow-list, explicit non-defaultable confirm at every
  surface. No surface may default `confirm`.
- **D11 — Explicit targets only.** Commands name files/refs; refuse directory-wide builds or
  ambient mutation outside the stated target.
- **D12 — MCP tool surface budget ≤ 4,000 tokens.** Deep guidance lives in skill-pack
  progressive references and `cadre://doc/**` resources, not tool descriptions.
- **D13 — Single schema source.** CLI JSON shapes, MCP tool schemas, and OpenAPI are
  generated from one Rust type layer; drift is a CI failure.
- **D14 — No telemetry.** None. Opt-in local bench reports may be printed; nothing phones home.
- **D15 — Cerebro agent id `CADRE`.** Session memory for this repo is isolated under
  `agent_id="CADRE"`.
- **D16 — Crate map is a requirement; internal design is free.** Workspace members follow
  PRD §6 (`cadre-kernel`, `cadre-lang`, `cadre-model`, faces, …). Bootstrap keeps a thin
  `cadre` facade crate so the workspace resolves from commit 0; slices split logic into the
  named crates rather than growing an unstructured monolith.
- **D17 — House MCP is hand-rolled (OQ-7 closed 2026-08-06).** Stdio + streamable HTTP stay
  in `cadre-mcp` without the official SDK until a dated amendment re-opens OQ-7. See
  `docs/MCP_SDK.md` for the compliance matrix and reopen criteria. D12 tool budget still binds.
- **D18 — Working name `cadre` pending trademark/crates.io sweep.** Binary and crate
  namespace stay `cadre` until OQ-1 closes; rename is a deliberate charter amendment.
- **D19 — OCCT bind path GO (S1).** Default parity backend will be `cadre-occt` implementing
  `GeomKernel`, layered on bschwind `opencascade`/`opencascade-sys` with **dynamic or
  separately installed engine** preferred over static `occt-sys` in the default binary.
  Fallback ladder: thin cxx to prebuilt OCCT → `occt-wasm` engine process. Do **not** adopt
  `cadrum`/high-level crates as the authoring surface. Full write-up: `docs/occt-binding.md`.
  Live OCCT box→STEP remains S3; this decision locks the *approach*, not the finished binding.

## Phases

Aligned with PRD §13 milestones. Each "done when" is checkable.

| Phase | Scope | Done when | v1 outcome |
|-------|-------|-----------|------------|
| **M0 — Kernel spike** | OCCT binding, `GeomKernel` v0, Starlark PoC | Part 1 e2e | **done** (S0–S3); mock CI + local OCCT |
| **M1 — Core loop** | IR+cache, build/inspect/export, selectors, parity 1–4 | Parts 1–4 + CLI JSON | **done** (S4–S6) |
| **M2 — See & serve** | snapshot/GIF, viewer alpha, MCP, skill-pack | Snapshot + MCP loop | **done** (S7–S8); parts 5–8 deferred |
| **M3 — Assemble & source** | assemblies, parts.lock, HTTP API | Assembly + API | **done** (S9); harness score amber |
| **M4 — Robots** | URDF/SRDF/SDF + validate | URDF parses | **done** (S10); joint-jog viewer deferred |
| **M5 — Fabricate** | DXF, DFM, slicer, gcode, Bambu gates | Safety gates | **done** (S11); live print deferred |
| **M6 — 1.0 hardening** | Windows, fuzz, dual skills, licensing, metrics | Metrics 1–16 green | **done** (S12) |

Bootstrap (this stamp) was **S0** under M0. **S0–S12 are merged on `main` as of 2026-08-05.**
Honesty on partial done-whens: see `docs/METRICS.md` rows 17–20 and `docs/STATUS.md`.

## Deliberately out of v1

**Permanently out (v1 non-goals)**

- 2D constraint solver / GUI sketcher — code-CAD only (NG1)
- CAM toolpathing, FEA, production raytracing, BIM (NG2)
- Hosted multi-tenant SaaS with authn/z (NG3)
- Mesh sculpting as modeling medium (NG4)
- Running build123d/ezdxf Python sources (NG5)

**Out of v1, honestly deferred**

- WASM component authoring against IR (post-1.0 escape hatch)
- Pure-Rust kernel as default (truck seeds the trait; OCCT remains default until parity)
- Additional printer adapters (Klipper/Moonraker, OctoPrint) and extra DFM vendor profiles
- Implicit SDF CAD (FR-9xx experimental)
- build123d migration assistant (OQ-3) — only if buildable from public docs, never reference source
- STEP PMI/GD&T and drawing sheets

## Open questions

From PRD §16 — still unresolved; do not silently assume answers in code:

1. **OQ-1** Final product/binary name and crate namespace (trademark + crates.io).
2. **OQ-2** Starlark dialect details: float formatting, module system for shared libraries, stdlib naming via LLM A/B.
3. **OQ-3** Migration assistant scope/timing (M6 vs defer).
4. **OQ-4** Depth of assembly joint model in STEP for 1.0 (kinematic AP242 vs labels+placements).
   **Partial (H3-4):** labels+placements+joint envelope → `cadre.assembly_kinematics` sidecar and
   `assembly emit-robot` → URDF path. **AP242 STEP joint entities still open.**
5. ~~**OQ-5** Whether MCP `write_source`/`read_source` default on for local stdio or HTTP-only.~~ **Resolved 2026-08-05 (H7):** stdio `write_source` **OFF** by default; HTTP **ON** by default; override via `CADRE_MCP_WRITE_SOURCE`. `read_source` on both. See amendments + `cadre://doc/write-source-policy`.
6. **OQ-6** Vendor-profile governance for community DFM rulepack updates.
7. ~~**OQ-7** MCP transport: official SDK vs house hand-rolled (see D17).~~ **Resolved 2026-08-06 (H2-2):** **stay hand-rolled**. No dual stack. Compliance matrix + reopen criteria in `docs/MCP_SDK.md`. `initialize.serverInfo.implementation = "hand-rolled"`.

---

## Amendments

Dated entries. A decision changes here first, then in the code.

- **2026-08-05** — charter adopted from `docs/cadre-prd.md` (Draft v0.1, 2026-07-28) at Launchpad-RS bootstrap.
- **2026-08-05** — D19: S1 kernel spike GO on OCCT via `GeomKernel` + opencascade-rs family
  (dynamic/separate engine); see `docs/occt-binding.md`. `cadre-kernel` + `MockKernel` landed.
- **2026-08-05** — S2: `cadre-lang` Starlark host + feature IR v0 (box/cylinder/boolean/label);
  hermetic `load()` refuse; diagnostic JSON shape pinned in `docs/design.md`.
- **2026-08-05** — S3: `cadre-occt` + `execute_ir`; fillet/chamfer IR ops; calibration block
  STEP e2e green locally (`CMAKE_POLICY_VERSION_MINIMUM=3.5`). CI excludes OCCT package.
- **2026-08-05** — S4: `cadre-model` selectors + build cache; `cadre-inspect` refs/measure;
  stable sort keys (centroid/area); cache keyed by source+params+versions (FR-105).
- **2026-08-05** — S5: `cadre-cli` (`cadre` bin) build/inspect/export + `--json`; mock default;
  optional `occt` feature; dir-wide builds refused; IR companion always written.
- **2026-08-05** — S6: Parity parts 1–4 fixtures + `cadre-bench` + `cadre bench run`; mock
  volume goldens; selectors/measure checks in CI.
- **2026-08-05** — S7: `cadre-render` software z-buffer snapshots (PNG multi-view + orbit GIF);
  `cadre snapshot` / `cadre view` (loopback HTML + deep links). Preview mesh notes for cuts.
- **2026-08-05** — S8: hand-rolled MCP stdio (`cadre mcp`) with 6 tools; skill-pack alpha at
  `skills/cadre` + `cadre skills export`. Snapshot tool can return image content blocks.
- **2026-08-05** — S9: `cadre-parts` (parts.lock fail-closed, LocalFsProvider, AssemblySpec +
  align_check); `cadre-api` Axum `/v1/*` + jobs/SSE + OpenAPI; `cadre serve api`; example
  plate+bolt assembly under `examples/assembly/`.
- **2026-08-05** — S10: `cadre-robot` URDF writer + structural/inertial validation + urdf-rs
  parse; SRDF/SDF emit; `cadre robot gen|validate`; `examples/robots/simple_arm`.
- **2026-08-05** — S11: `cadre-fab` DXF R12, DFM engine + bundled SendCutSend-style profile,
  slicer discovery/command preview, gcode-check, Bambu adapter dry-run + hard start gates
  (live MQTT start still refused); `cadre fab` / `cadre printer`.
- **2026-08-05** — S12: v1 hardening — `docs/METRICS.md`, `docs/LICENSING.md`, Windows CI,
  `skills export --all` (claude-code/codex/hermes), property tests on lang/gcode/selectors,
  release checklist.
- **2026-08-05** — Post-S12 docs sync: `docs/STATUS.md` as-built map; README/CLAUDE/design
  crate tables aligned to reality (no phantom crates); milestone table marked done with
  honesty notes on amber/red metrics.
- **2026-08-05** — **H7 / OQ-5 resolved:** MCP `write_source` default **OFF on stdio**, **ON on HTTP**;
  override `CADRE_MCP_WRITE_SOURCE=0|1`. `resources/list` + `resources/read` for
  `cadre://doc/**` and `cadre://artifact/**`. `read_source` remains on both transports.
- **2026-08-06** — **H2-2 / OQ-7 resolved:** stay **hand-rolled** MCP (no official SDK dual stack).
  Compliance constants + tests in `cadre-mcp`; decision log `docs/MCP_SDK.md`. D17 amended.
- **2026-08-06** — **H2-5 joint depth:** assembly `JointSpec` gains axis/origin/limits;
  `validate_assembly` fail-closed; robot revolute/prismatic missing or inverted limits are **errors**.
  CLI `cadre assembly validate`. Not AP242 STEP kinematics (OQ-4 still open for STEP depth).
  See `docs/JOINTS.md`.
- **2026-08-06** — **H2-9 SDF secondary:** `cadre-sdf` analytic box/cyl sample + raw/NRRD;
  CLI `cadre sdf sample`. **Not a modeling path** — STEP/B-rep remains primary (`docs/SDF.md`).
- **2026-08-06** — **H2-10 truck parity bid prep:** evidence pack `docs/TRUCK_PARITY_BID.md`.
  Decision **NO-GO** for default/`parity_eligible`. No code default flip. Horizon-2 board complete.
- **2026-08-06** — **Horizon-3 chartered:** `docs/HORIZON3.md` Top-N H3-1…H3-10 (honesty, agent loop,
  gated BREP spike). Default next **H3-1**. Does not flip D4/truck default.
- **2026-08-06** — **H3-1 honesty pass:** OCCT `cone` is **Unsupported** (no cylinder stand-in);
  suite/kernel fences in `docs/KERNEL_HONESTY.md`; version JSON `truck_implementation=truck-seed-analytic-csg`.
- **2026-08-06** — **H3-4 OQ-4 partial:** `assembly emit-kinematics` / `emit-robot` bridge CAD joints →
  robot IR (mm→m). Not AP242. See `docs/JOINTS.md`.
- **2026-08-14** — **H3-6 truck BREP spike:** optional `--kernel truck-brep` (`--features truck-brep`)
  wires upstream `truck-modeling`/`truck-shapeops` for box+boolean+tessellate. `parity_eligible` still
  false; default unchanged. G1 partial (no STEP). Apache-2.0 pins in `docs/LICENSING.md`.
