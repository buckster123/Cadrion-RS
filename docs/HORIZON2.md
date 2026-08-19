# Cadrion-RS Horizon-2 — mediums & distribution

> **Status:** **COMPLETE** 2026-08-06 (H2-1…H2-10). **Archive.**  
> **Successor (active):** [`docs/HORIZON3.md`](HORIZON3.md)  
> **When written:** 2026-08-06 (after H1–H10 / PR #24–#34).  
> **Authority:** does **not** override `docs/CHARTER.md` D1–Dn. Ordering only.  
> **Prior board:** [`docs/HORIZON.md`](HORIZON.md) (Horizon-1 archive + principles).  
> **Agent id:** `CADRION`

## 0. Where we are

| Layer | State |
|-------|--------|
| v1 S0–S12 | complete on `main` |
| Horizon-1 H1–H10 | complete (#24–#34) |
| Default CI | still **mock / OCCT-free** |
| Kernels | mock default · occt opt-in · truck experimental NON-PARITY |

**Horizon-1 pitch delivered:** agent loop depth (harness, stdlib, OCCT xforms, fillet, viewer, fab gates, MCP resources, migrate, klipper, truck seed).

**Horizon-2 pitch:** expand **mediums and distribution** without breaking STEP-first, consent gates, or “truck never default.”

---

## 1. Ordering principles (same five as H1)

| # | Principle | Prefer | Deprioritize |
|---|-----------|--------|--------------|
| P1 | Agent loop leverage | Surfaces agents call in write→build→inspect→repair | Speculative platforms |
| P2 | Honesty debt first | Close “looks done but isn’t” | Shiny over fake-complete |
| P3 | Known-working now | Extend IR/MCP/fab/OCCT we ship | WASM/SaaS before payoff clear — *except* H2-1 as deliberate escape hatch after H1 proof |
| P4 | Charter fit | STEP-first, hermetic, gated print/slice | SDF-as-primary; multi-tenant SaaS |
| P5 | Dependency order | Unblocks later H2-N | Parallel thrash |

**Natural next (recommended):**  
**(a) H2-1 WASM IR component** — distribution escape hatch now that agent value is proven on native CLI/MCP.

---

## 2. Top-N cook board (H2-1 … H2-10)

Numbered **H2-1..H2-10**. Cook order = recommended order. Inside a slice, (a)(b)(c) still applies.

### H2-1 — WASM IR component (escape hatch)
**Goal:** `cadrion-wasm` (or `cadrion-lang`/`kernel` feature) builds a **wasm32** package that can `evaluate` Starlark → IR and run **mock** facts (no OCCT). Optional thin JS glue + example page.  
**Why:** Horizon-2 distribution; agents/browsers without native binary; proves IR is the portable contract.  
**Depends on:** none (H1 done).  
**Exit:** `wasm-pack` or `cargo build --target wasm32-unknown-unknown` green in CI job or documented optional lane; `docs/WASM.md`; version/meta says mock-only + non-OCCT.  
**Not:** full OCCT-in-browser; not multi-tenant host.

### H2-2 — MCP OQ-7 revisit (SDK or harden hand-roll)
**Goal:** dated CHARTER decision: stay hand-rolled **or** migrate subset to official MCP SDK; either way document resources/tools parity and a compliance checklist.  
**Why:** OQ-7 open; H7 added resources — maintenance cost is now real.  
**Depends on:** H7 done.  
**Exit:** CHARTER amendment + `docs/MCP_SDK.md` (or “stay hand-rolled” with test matrix). No silent dual stacks.

### H2-3 — Fab depth pack (DFM profiles + OctoPrint)
**Goal:** ≥1 new DFM bundled profile + **OctoPrint** adapter behind same gates as Bambu/Klipper (`START`, allowlist, sha256, `--live`).  
**Why:** known-working fab spine; low doctrinal risk.  
**Depends on:** H6/H9 patterns.  
**Exit:** `fab profiles` lists 3+; `printer --backend octoprint` dry-run + gated start; docs.

### H2-4 — Published live-harness score (honesty)
**Goal:** one **dated** frontier or strong-local run of `harness run --suite agent10 --cmd '…'` with model id, score, median loops, failure notes in `docs/METRICS.md` / `docs/HARNESS_LIVE.md`.  
**Why:** H1 plumbing is green; PRD “is it good for AI?” still wants a public number (or honest miss).  
**Depends on:** H1.  
**Exit:** metrics row with date + cmd + model; no fake ≥6 if we score lower.

### H2-5 — Assembly joint depth (OQ-4 bite)
**Goal:** richer joint model beyond labels+placements: revolute/prismatic limits in robot+assembly path, validate + one example that fails closed on bad limits.  
**Why:** OQ-4 residual; robots already ship.  
**Depends on:** S10/H5.  
**Exit:** example + tests; CHARTER note if still short of AP242 kinematics.

### H2-6 — Viewer 3D depth
**Goal:** loopback viewer gains coarse 3D (mesh from tessellate/GLB) *or* deeper scrub (time-based gcode, multi-joint FK 3D). Stay single HTML process if possible.  
**Why:** H5 is 2D alpha; human-in-loop still thin.  
**Depends on:** H5.  
**Exit:** docs + `--once` CI; honesty: not Blender.

### H2-7 — Migrator depth
**Goal:** `cadrion migrate` covers more public-API shapes (Workplane/Locations offsets → translate, simple fillet call → note stub, extrude-ish → box).  
**Why:** H8 skeleton is thin; onboarding payoff.  
**Depends on:** H8.  
**Exit:** +2 fixtures; still refuse unsafe Python.

### H2-8 — PMI / drawing alpha (minimal)
**Goal:** smallest honest PMI surface: attach linear dimension facts to selectors → JSON “drawing packet” (not full sheets).  
**Why:** Horizon-2 mediums; huge if unbounded — keep alpha.  
**Depends on:** inspect refs.  
**Exit:** `cadrion inspect dims` or MCP tool; example; “not a drafting package” note.

### H2-9 — SDF secondary medium (experimental)
**Goal:** optional `cadrion-sdf` or feature: SDF sample grid from mock/OCCT mesh **or** simple analytic SDF for box/cyl; export NRRD/raw; never default modeling path.  
**Why:** FR-9xx parking; must stay **secondary** to STEP.  
**Depends on:** none hard.  
**Exit:** docs fence + tests; CHARTER reminder STEP-first.

### H2-10 — Truck parity bid **prep** (not default)
**Goal:** evidence pack only: what real B-rep would need, gap vs OCCT parts1-10, optional spike binding note — **no** default flip, **no** parity claims.  
**Why:** H10 seed exists; promotion is Horizon-3.  
**Depends on:** H10.  
**Exit:** `docs/TRUCK_PARITY_BID.md` with go/no-go criteria; code changes optional and feature-gated.

---

## 3. Explicitly NOT Horizon-2

| Item | Why |
|------|-----|
| truck as **default** kernel | Horizon-3 + parity-gated |
| Multi-tenant public SaaS | Charter NG3; needs re-charter |
| SDF as primary CAD medium | STEP-first doctrine |
| Weaken print/slice gates | Safety invariant |
| Full PMI/drawing package | Too big; H2-8 is alpha only |

---

## 4. Parking → board map

| Parking | H2 slot |
|---------|---------|
| WASM IR | **H2-1** |
| MCP SDK (OQ-7) | **H2-2** |
| More DFM / OctoPrint | **H2-3** |
| Live LLM score publish | **H2-4** |
| Joint/AP242 depth | **H2-5** |
| Viewer depth | **H2-6** |
| Migrator depth | **H2-7** |
| PMI/drawings | **H2-8** (alpha) |
| Implicit SDF | **H2-9** (secondary) |
| truck → default | **out** → H2-10 prep only, promotion H3 |

---

## 5. Cook cadence

Same as Horizon-1:

1. One H2-N per branch `feat/h2n-short-name`  
2. Exit criteria in PR body  
3. Agent merges when CI green (user trust)  
4. Tick checklist + BACKLOG on merge  
5. Default next = next unchecked H2-N  

**Default next if “cook on” with no pref:** **Horizon-2 complete** — see § archive / Horizon-3.

---

## 6. Checklist

- [x] **H2-1** WASM IR component  
- [x] **H2-2** MCP OQ-7 SDK decision  
- [x] **H2-3** Fab depth (DFM + OctoPrint)  
- [x] **H2-4** Published live harness score  
- [x] **H2-5** Assembly joint depth  
- [x] **H2-6** Viewer 3D depth  
- [x] **H2-7** Migrator depth  
- [x] **H2-8** PMI/drawing alpha  
- [x] **H2-9** SDF secondary experimental  
- [x] **H2-10** Truck parity bid prep  

**Horizon-2 complete (2026-08-06).** Default next: park or charter Horizon-3.

---

## 7. One-line pitch

> **Horizon-2** ships Cadrion beyond the native agent workstation — portable IR (WASM), sturdier MCP story, deeper fab/robots/viewer, and fenced experimental mediums — without surrendering STEP-first honesty or consent-gated fab.
