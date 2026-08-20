# Cadrion-RS backlog — slice ledger

A row gets its ✅ when the slice is **merged, deployed, and verified live** — not when tests
pass (house doctrine #5). Notes carry the date and the evidence.

## v1 (agent CAD loop parity) — COMPLETE

Milestones M0–M6 / slices S0–S12 shipped on `main` (2026-08-05). Scorecard: `docs/METRICS.md`.
As-built map: `docs/STATUS.md`.

- [x] **S0 — bootstrap**: Launchpad stamp, CLAUDE.md, CHARTER, design contract, PRD in-tree,
      workspace + `crates/cadrion` facade, CI, dual license (2026-08-05)
- [x] **S1 — M0 kernel spike (trait + binding eval)**: `cadrion-kernel` `GeomKernel` v0 +
      `MockKernel` tests; `docs/occt-binding.md`; charter D19 GO (2026-08-05). Live OCCT → S3
- [x] **S2 — M0 Starlark host PoC**: `cadrion-lang` hermetic eval; `params`/`box`/`cylinder`/
      booleans/`solid` → feature IR v0; structured diagnostics JSON; overrides (2026-08-05)
- [x] **S3 — M0 e2e part 1**: `cadrion-occt` GeomKernel; IR execute; fillet/chamfer in lang;
      calibration block → STEP + facts (OCCT local; CI excludes package) (2026-08-05)
- [x] **S4 — M1 build cache + selectors**: `cadrion-model` (selectors + content-hash cache);
      `cadrion-inspect` (refs/measure); stable `#o…` tokens (2026-08-05)
- [x] **S5 — M1 CLI face**: `cadrion-cli` binary `cadrion` — `build` / `inspect refs|measure` /
      `export step|stl|glb` with `--json`; mock default, optional `--features occt` (2026-08-05)
- [x] **S6 — M1 parity parts 1–4**: `parity/parts/01–04` + `cadrion-bench` runner +
      `cadrion bench run --suite parts1-4`; mock CI green (2026-08-05)
- [x] **S7 — M2 snapshot + viewer alpha**: `cadrion-render` software PNG/GIF packets;
      `cadrion snapshot` / `cadrion view` deep links (2026-08-05)
- [x] **S8 — M2 MCP stdio + skill-pack alpha**: `cadrion-mcp` 6 tools; `cadrion mcp`;
      `cadrion skills export`; bundled `skills/cadrion` doctrine (2026-08-05)
- [x] **S9 — M3 assemblies + parts.lock + HTTP API**: `cadrion-parts` lock/provider/assembly;
      `cadrion-api` Axum `/v1` + jobs/SSE/OpenAPI; plate+bolt example; `cadrion serve api` (2026-08-05)
- [x] **S10 — M4 robots**: `cadrion-robot` URDF/SRDF/SDF gen+validate+inertials; urdf-rs parse;
      `cadrion robot gen|validate`; simple_arm example (2026-08-05)
- [x] **S11 — M5 fab path**: `cadrion-fab` DXF + DFM (SendCutSend-style) + slicer discover +
      gcode-check + Bambu dry-run/gated start (no live print); `cadrion fab` / `cadrion printer`
      (2026-08-05)
- [x] **S12 — M6 1.0 hardening**: metrics table, licensing review, Windows CI job, dual-agent
      skills export (`--all`), property/fuzz-style parser tests, release checklist (2026-08-05)

## Post-v1 candidates (unordered; pick deliberately)

Priority suggestions when resuming:

1. **OCCT depth** — live topology + AdHoc booleans + mesh topology ✅ (PR #14)
2. **OCCT bench lane** — `parts1-4-occt` + expect.occt.json ✅ (PR #15)
3. **Face→DXF** — project planar face selector to DXF ✅ (PR #16)
4. **Live Bambu** — FTPS/MQTT behind gates + `--live` ✅ (PR #17)
5. **Streamable-HTTP MCP** — POST /mcp + SSE ✅ (PR #18)
6. **Agent harness score** — scripted agent10 ≥6/10 ✅ (PR #19)
8. **Diff/align/frame** CLI polish beyond assembly align_check ✅ (PR #20)
9. **Parity 5–10** — full parts1-10 mock suite + translate/rotate ✅ (PR #21)
10. **OCCT translate/rotate + expect.occt 5–10** ✅ (PR #22)

## Horizon-1 board (ordered) — COMPLETE

**Archive:** [`docs/HORIZON.md`](docs/HORIZON.md). **Active:** [`docs/HORIZON2.md`](docs/HORIZON2.md).

| # | Slice | Status |
|---|-------|--------|
| H1 | Live agent harness driver (`--cmd` / MCP score) | ✅ PR #24/#25 |
| H2 | Stdlib depth (mirror, patterns, cone/sphere, …) | ✅ PR #26 |
| H3 | OCCT transform quality (drop STEP rotate round-trip) | ✅ PR #27 |
| H4 | Fillet/chamfer in OCCT parity + diagnostics | ✅ PR #28 |
| H5 | Viewer: G-code scrub + URDF jog alpha | ✅ PR #29 |
| H6 | Slicer execute (gated) + 2nd DFM profile | ✅ PR #30 |
| H7 | MCP resources + write_source policy | ✅ PR #31 |
| H8 | build123d → skeleton migrator (clean-room) | ✅ PR #32 |
| H9 | Klipper/Moonraker gated adapter | ✅ PR #33 |
| H10 | truck experimental non-parity lane | ✅ PR #34 |

## Horizon-2 board (ordered) — COMPLETE

**Archive:** [`docs/HORIZON2.md`](docs/HORIZON2.md). **Active:** [`docs/HORIZON3.md`](docs/HORIZON3.md).

| # | Slice | Status |
|---|-------|--------|
| H2-1 | WASM IR component (escape hatch) | ✅ PR #35 |
| H2-2 | MCP OQ-7 SDK decision | ✅ PR #36 |
| H2-3 | Fab depth (DFM + OctoPrint) | ✅ PR #37 |
| H2-4 | Published live harness score | ✅ PR #38 |
| H2-5 | Assembly joint depth | ✅ PR #41 |
| H2-6 | Viewer 3D depth | ✅ PR #42 |
| H2-7 | Migrator depth | ✅ PR #43 |
| H2-8 | PMI/drawing alpha | ✅ PR #44 |
| H2-9 | SDF secondary experimental | ✅ PR #45 |
| H2-10 | Truck parity bid prep (not default) | ✅ PR #46 |

## Horizon-3 board (ordered) — COMPLETE

**Source of truth:** [`docs/HORIZON3.md`](docs/HORIZON3.md).

| # | Slice | Status |
|---|-------|--------|
| H3-1 | Honesty pass (cone + fences + truck naming) | ✅ this slice |
| H3-2 | Live harness frontier score | ✅ 4.0/10 PR #60 (miss ≥6) |
| H3-3 | MCP surface depth (dims/assembly/sdf) | ✅ PR #48 |
| H3-4 | Assembly / OQ-4 bite | ✅ PR #49 |
| H3-5 | PMI → viewer overlay | ✅ PR #50 |
| H3-6 | Truck BREP spike (bid G1) | ✅ PR #51 |
| H3-7 | OCCT parity depth | ✅ PR #52 |
| H3-8 | DFM / OQ-6 governance seed | ✅ PR #54 |
| H3-9 | Migrator / WASM polish | ✅ this slice |
| H3-10 | OQ-1 name decision packet | ✅ this slice — Cadrion-RS |

Default when resuming with no pref: **Horizon-5** ([`docs/HORIZON5.md`](docs/HORIZON5.md)).
H3-2 published 2026-08-20: **4.0/10** on PR #60 (miss ≥6). Live re-score is not an H5 slice.

## Horizon-4 board (ordered) — COMPLETE

**Archive:** [`docs/HORIZON4.md`](docs/HORIZON4.md). **Active:** [`docs/HORIZON5.md`](docs/HORIZON5.md).

| # | Slice | Status |
|---|-------|--------|
| H4-1 | `cadrion schema` (D13) | ✅ PR #57 |
| H4-2 | `cadrion engine info\|install` (D4) | ✅ PR #58 |
| H4-3 | HTTP measure / dims / sdf (D5) | ✅ PR #59 |

## Horizon-5 board (ordered) — ACTIVE

**Source of truth:** [`docs/HORIZON5.md`](docs/HORIZON5.md).

| # | Slice | Status |
|---|-------|--------|
| H5-1 | Harness prompt honesty (A3) | ✅ this slice |
| H5-2 | MCP/HTTP align · frame · diff | ✅ this slice |
| H5-3 | MCP/HTTP export | ✅ this slice |
| H5-4 | MCP/HTTP fab check | ✅ this slice |
| H5-5 | MCP engine + schema | ✅ this slice |
| H5-6 | MCP prompts | |
| H5-7 | OQ-2 dialect bite | |
| H5-8 | MCP/HTTP robot gen \| validate | |
| H5-9 | Migrator bite | |
| H5-10 | Truck G1 STEP or honest refuse | |

Default next: **H5-6**.

## Post-v1 parking (still deferred)

- truck as **default** — only after bid G1–G7 + CHARTER (post H3-6+)
- Public multi-tenant SaaS — needs re-charter (NG3)
- Full PMI/drawing package beyond alpha bites
- SDF as **primary** modeling medium (forbidden)
