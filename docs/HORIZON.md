# Cadrion-RS Horizon-1 — post foundation board (COMPLETE)

> **Status:** **COMPLETE** (H1–H10 merged ~PR #24–#34).  
> **When written:** 2026-08-05 · **closed:** 2026-08-06.  
> **Next board:** [`docs/HORIZON2.md`](HORIZON2.md)  
> **Authority:** does **not** override `docs/CHARTER.md` D1–Dn.  
> **Agent id:** `CADRION`

## 0. Where we are (ground truth)

**Done and honest:**

| Layer | Evidence |
|-------|----------|
| v1 agent CAD loop | S0–S12 on `main` (CLI · MCP stdio+HTTP · API · robots · fab · skills · Windows CI) |
| OCCT depth | AdHoc booleans, mesh topology, cut e2e (#14) |
| Parity | mock **parts1-10** (#21); OCCT **parts1-4-occt** + **parts5-10-occt** (#15, #22) |
| Agent proof | scripted harness **10/10** (#19) — not frontier-LLM |
| Fab safety | allowlist + sha256 + `START` + `--live` (#17) |
| Inspect polish | align / frame / diff (#20); face→DXF (#16) |

**Foundation property:** default CI is still **mock / OCCT-free**. Real B-rep is opt-in (`--features occt`). That split is load-bearing — do not break it for convenience.

**Parking lot (from BACKLOG, still valid):**

1. Implicit SDF CAD (FR-9xx)  
2. WASM component authoring against IR  
3. truck kernel promotion toward default (parity-gated)  
4. build123d → skeleton `.cad.star` migration assistant  
5. Klipper/Moonraker/OctoPrint + more DFM profiles  
6. STEP PMI/GD&T + drawing sheets  
7. Public multi-tenant HTTP hardening  

Plus **residual honesty debt** not in the old parking bullets (these matter more short-term):

- R1. Official MCP SDK still deferred (OQ-7); hand-rolled works  
- R2. Live LLM harness (`harness --cmd` / MCP agent) not wired  
- R3. OCCT rotate is STEP round-trip (slow; not multi-thread safe)  
- R4. Mock booleans/volumes are analytic approx — agent doctrine must keep saying so  
- R5. Viewer is loopback HTML, not full URDF-jog / G-code scrub PRD surface  
- R6. Fillet/chamfer real on OCCT but unused in most parity stars (mock unsupported)  
- R7. Assembly joints are data model + align_check, not full AP242 kinematics  
- R8. Slicer path is discover + preview; execute still soft  

---

## 1. Ordering principles (how to pick)

Rank candidates by **all five**, not one:

| # | Principle | Prefer | Deprioritize |
|---|-----------|--------|--------------|
| P1 | **Agent loop leverage** | Things that make “write → build → inspect → repair” tighter | Things agents never call in the hot path |
| P2 | **Honesty debt first** | Close “looks done but isn’t” gaps | New shiny surfaces over fake-complete ones |
| P3 | **Known-working now** | Extends OCCT/mock/MCP paths we already ship | Bleeding platforms (WASM/SaaS) before payoff is clear |
| P4 | **Charter fit** | STEP-first, hermetic, consent-gated | Implicit SDF as primary medium; multi-tenant SaaS |
| P5 | **Dependency order** | Unblocks later Top-N items | Parallel islands that thrash context |

**Natural next step (recommended):**  
**(a) Horizon-1 agent depth** — live LLM harness + richer stdlib (fillet/patterns used for real) + OCCT transform quality — *not* truck, WASM, or SaaS.

Why: foundation is agent CAD. The biggest remaining gap is **“a real model can drive Cadrion end-to-end and we can score it”** plus **geometry vocabulary depth** so parts 7–10 stop being mock-shaped approximations. Parking-lot truck/WASM/SaaS do not improve that loop yet.

---

## 2. Top-N todo list (Horizon-1 cook board)

Numbered **H1..H10**. Alphabetical option lists still apply inside a slice; this list is **recommended cook order**.

### H1 — Live agent harness driver (highest leverage)
**Goal:** `cadrion harness run --suite agent10 --cmd '…'` (or MCP client) drives **natural-language prompts only**, scores first-try / loops-to-success / wall / tokens.  
**Why now:** scripted 10/10 proves surfaces; does not prove LLM fluency. PRD §12 agent number is still the open “is it good for AI?” metric.  
**Depends on:** nothing new.  
**Exit:** one documented run with a frontier or strong local model ≥6/10 ≤3 loops median (or honest score + failure corpus).  
**Not:** training data collection; not multi-tenant eval farm.

### H2 — Stdlib depth pack (patterns + mirror + cone/sphere + polar)
**Goal:** IR/stdlib ops agents need without cheating: `mirror`, `linear_pattern` / `polar_pattern`, `cone`/`sphere` (or wedge), optional `revolve` stub.  
**Why:** impeller/stair/planetary get *more real* without leaving STEP-first.  
**Depends on:** H3 nice-to-have for OCCT rotate quality.  
**Exit:** at least 2 parity parts rewritten to use new ops; mock + OCCT goldens updated; docs/examples.

### H3 — OCCT transform quality (drop STEP round-trip rotate)
**Goal:** rotate/translate without per-op STEP clone (direct `BRepBuilderAPI_Transform` on shape handle, or thin `opencascade` patch / helper crate).  
**Why:** H2 polar patterns will thrash STEP I/O; serial-only OCCT is a footgun.  
**Depends on:** none.  
**Exit:** transform_smoke + parts5-10-occt faster; documented thread policy or mutexed kernel pool.  
**Honesty:** if still serial, say so in gotchas.

### H4 — Fillet/chamfer in parity + diagnostics loop
**Goal:** cal-block / flange / bracket stars use real OCCT fillet where mock xfail’s cleanly; diagnostics name selectors.  
**Why:** FR-style repair loop; mock must stay `Unsupported` (already).  
**Depends on:** H3 optional.  
**Exit:** OCCT-only expect lane for filleted variants; skill doctrine “if mock, skip fillet”.

### H5 — Viewer depth (G-code scrub + URDF jog alpha)
**Goal:** extend `cadrion view` beyond STEP/STL/GLB: G-code layer scrub (fab) + URDF joint sliders (robot).  
**Why:** human-in-loop for fab/robot already shipped as data; P2 user still blind.  
**Depends on:** none.  
**Exit:** deep links + `--json` meta; no new crate if possible (keep loopback HTML honest).

### H6 — Slicer execute (gated) + second DFM profile
**Goal:** `fab slice --run` behind confirm; one more vendor profile (e.g. generic laser or PCB outline).  
**Why:** closes “preview only” honesty on fab; low risk if gates match printer doctrine.  
**Depends on:** none.  
**Exit:** dry-run default; `--confirm SLICE` or reuse START pattern; profile version in report.

### H7 — MCP surface completion (resources + write_source policy)
**Goal:** `cadrion://doc/**` + `cadrion://artifact/**` resources; decide default for `write_source` (local stdio off / HTTP on).  
**Why:** OQ-5/OQ-7 residue; agents on HTTP need project I/O.  
**Depends on:** none.  
**Exit:** dated CHARTER note on write_source default; resource list in skills.

### H8 — build123d → skeleton migrator (clean-room)
**Goal:** `cadrion migrate path.py` → best-effort `.cad.star` skeleton (structure + params, not full semantics).  
**Why:** onboarding; charter OQ-3. **Only** from public build123d docs / user-supplied files — never reference repo source.  
**Depends on:** H2 (richer target stdlib = better skeletons).  
**Exit:** 3 golden .py fixtures (hand-written, public-API shaped); refuse if parse unsafe.

### H9 — Klipper/Moonraker adapter (gated like Bambu)
**Goal:** second printer family behind same consent gates.  
**Why:** parking lot fab breadth; only after H6 so slice→send is one story.  
**Depends on:** H6.  
**Exit:** dry-run + allowlist + hash + confirm; no silent start.

### H10 — truck experimental lane (explicit non-parity)
**Goal:** `cadrion-truck` crate implements subset of `GeomKernel` (box/cyl/boolean/facts); never default; never parity-10 claims.  
**Why:** D4 honesty valve; pure-Rust future seed. **Not** “promotion toward default” until H1–H4 prove OCCT still wins.  
**Depends on:** none, but **schedule after H1–H4** so it doesn’t steal agent-loop oxygen.  
**Exit:** feature-gated tests; meta warns non-parity.

---

## 3. Parking lot map → Top-N

| Parking item | Maps to | Horizon |
|--------------|---------|---------|
| Implicit SDF CAD | **Defer hard** | Horizon-2+ (charter: STEP-first; FR-9xx experimental only) |
| WASM IR components | **Defer** | Horizon-2 (escape hatch after H1 proves agent value) |
| truck → default | **H10 seed only** | Promotion is Horizon-3 and parity-gated |
| build123d migrator | **H8** | After stdlib depth |
| Klipper/Moonraker/DFM | **H6 then H9** | Fab spine first |
| STEP PMI / drawings | **Defer** | Horizon-2 (huge; not agent hot path) |
| Multi-tenant HTTP | **Defer / NG3** | Not Cadrion’s product shape unless you re-charter |

---

## 4. Horizon-2

**Moved to separate board:** [`docs/HORIZON2.md`](HORIZON2.md) (H2-1 … H2-10).

Archive note (historical one-liner list): WASM · PMI · SDF secondary · truck parity bid · multi-tenant · MCP SDK — all expanded with exit criteria on the H2 board.

---

## 5. Suggested cook cadence

Same rhythm that worked for #14–#22:

1. Pick **one** H-item (default: next unchecked H#).  
2. Branch `feat/hN-short-name` off `main`.  
3. Exit criteria from this doc in PR body.  
4. You merge + prune; we immediately take next H#.  
5. Update this file’s checklist + `BACKLOG.md` one-liner when H# merges.

**Default next slice if you say “cook on” with no pref:** see **[`docs/HORIZON2.md`](HORIZON2.md)** → **H2-1**.

---

## 6. Checklist (tick when merged)

- [x] **H1** Live agent harness driver  
- [x] **H2** Stdlib depth pack  
- [x] **H3** OCCT transform quality  
- [x] **H4** Fillet/chamfer parity + diagnostics  
- [x] **H5** Viewer G-code + URDF jog  
- [x] **H6** Slicer execute + 2nd DFM profile  
- [x] **H7** MCP resources + write_source policy  
- [x] **H8** build123d skeleton migrator  
- [x] **H9** Klipper/Moonraker gated adapter  
- [x] **H10** truck experimental lane (non-parity)  

---

## 7. Anti-goals for Horizon-1

- Do not make truck the default.  
- Do not build multi-tenant SaaS.  
- Do not make SDF the primary modeling medium.  
- Do not weaken printer/slice gates for demo speed.  
- Do not claim frontier-LLM harness scores without publishing the cmd + model + date.  
- Do not parallelize OCCT STEP-heavy tests until H3 lands.

---

## 8. One-line pitch

> **Horizon-1 makes Cadrion unmistakably good for agents** (live harness + deeper stdlib + faster OCCT transforms).  
> **Horizon-2 expands mediums and distribution** (WASM, PMI, SDF side-path, truck parity bid).  
> **Parking stays parking** until Horizon-1 checklist is mostly green.
