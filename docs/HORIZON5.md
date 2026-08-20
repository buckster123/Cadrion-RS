# Horizon-5 — in-tree agent leftovers

**Status:** **COMPLETE** (H5-1…H5-10 merged 2026-08-20/21)  
**Predecessor:** [`docs/HORIZON4.md`](HORIZON4.md) — H4-1…H4-3 done  
**H3 note:** H3-2 live score **4.0/10** published 2026-08-20 ([PR #60](https://github.com/buckster123/Cadrion-RS/pull/60)) — miss ≥6, not invented. Not re-opened here.

Horizon-5 is **not** a new kernel, **not** a live GPU re-score, and **not** SaaS.  
It closes **already-shipped** agent gaps that have **no external blocker**: fairer harness
prompts, face catch-up (CLI verbs agents still cannot call), dialect pin, migrator bite,
and an honest truck G1 remainder.

---

## 1. Theme

> Horizon-4 taught the faces to describe themselves. Horizon-5 lets an agent **finish
> the loop** on those faces — without renting a box, opening `load()`, or flipping D4.

---

## 2. Ranking (same house rules)

| # | Prefer | Defer |
|---|--------|--------|
| P1 | Verbs agents already have on CLI but not MCP/HTTP | Pretty-only viewer |
| P2 | Honesty the live run already proved (underspec prompts, stale skill list) | Buying a ≥6 |
| P3 | Fail-closed if a crate cannot do the job | Fake STEP / fake fillet |
| P4 | Charter OQ bites that do **not** need a new decision | OQs that reopen D9 / D4 / NG3 |

---

## 3. Top-N board (all unblocked)

Every slice below is cookable on a laptop with default mock CI. None need Vast, ApexRouter,
Hermes field, or a paid model.

### H5-1 — Harness prompt honesty (A3)
**Goal:** Each `agent10` task prompt names the **labels and sizes** its asserts already
check. The 2026-08-20 miss was mostly `cylinder`≠`pin`, `o1`≠`cube`, and volumes the
prompt never stated.  
**Honesty:** oracle still cheats via the task file. This does **not** claim ≥6 and does
**not** require a live re-run.  
**Depends on:** none.  
**Exit:** prompt text contains every asserted `label` / size / volume band; a unit test
locks that; `docs/HARNESS_LIVE.md` notes the suite is now fair for the next live attempt.

### H5-2 — MCP/HTTP `inspect align|frame|diff` (D5 leftover)
**Goal:** CLI already has the repair verbs (`inspect align|frame|diff`). MCP and `/v1`
do not.  
**Depends on:** H2-8 polish (shipped).  
**Exit:** tools + OpenAPI paths + HTTP/MCP tests; `cadrion schema` lists them; D12 tool
budget still holds (short descriptions).

### H5-3 — MCP/HTTP export (D3)
**Goal:** Agents can request STL/GLB (and STEP when the kernel writes it) without dropping
to CLI. Mock STEP stays **Unsupported** — never a fake file.  
**Depends on:** existing `cadrion export`.  
**Exit:** `export` tool + `/v1/export`; refuse with `CADRION-E-UNSUPPORTED` when the
kernel cannot write the format.

### H5-4 — MCP/HTTP `fab check` (DFM preflight)
**Goal:** DFM profile check is the fab verb agents need before any printer. Printer
start / slice execute stay CLI-gated.  
**Depends on:** H3-8 profiles.  
**Exit:** `fab_check` tool + `/v1/fab/check`; findings cite profile version; no `START`.

### H5-5 — MCP `engine` + `schema` (D4 / D13)
**Goal:** An agent can ask “what kernels are in this binary?” and “what is the live
surface?” without shelling out. HTTP copies if cheap; MCP is the must.  
**Depends on:** H4-1 / H4-2.  
**Exit:** tools wrap `engine info` and `schema` dumps; install remains fail-closed (no
fetch). D12 budget: point at `cadrion://doc/**` for long catalogs.

### H5-6 — MCP prompts (empty `prompts/list`)
**Goal:** `prompts/list` returns `[]` today. Ship a handful of doctrine prompts: the
write→build→inspect→snapshot loop, write_source policy, hermetic `load()` refuse.  
**Honesty:** prompts teach; they are not a fourth face and not a skill-pack replacement.  
**Depends on:** H7 resources (shipped).  
**Exit:** `prompts/list` non-empty; `prompts/get` returns the text; compliance tests.

### H5-7 — OQ-2 dialect bite (no `load()`)
**Goal:** Pin what design.md still calls open: **float formatting** and **stdlib symbol
names** via golden-IR tests. Optional compiled-in `use("cadrion.patterns")` **only** if
it maps to already-global stdlib (no filesystem).  
**Forbidden:** user `load()` / ambient modules (D9).  
**Depends on:** none.  
**Exit:** `docs/STDLIB_DEPTH.md` (or a short dialect note) + goldens; OQ-2 marked
**partial** in CHARTER — module system for *user* libraries stays open.

### H5-8 — MCP/HTTP `robot gen|validate`
**Goal:** Robot IR → URDF path is CLI-only. Agents already have assembly joints (H3-4).  
**Depends on:** H3-4.  
**Exit:** tools + `/v1/robot/gen` and `/validate` (or one tool with `op`); existing
`simple_arm` fixture green; no silent inertial invention.

### H5-9 — Migrator bite (OQ-3)
**Goal:** One more **public** build123d-style family: either a real mapping (`mirror` /
`scale` → stdlib) **or** a structured refuse (`Workplane`, face-select, sweep/loft)
with a fixture. Fillet/chamfer stay stubs — no fake apply on mock.  
**Depends on:** H3-9.  
**Exit:** `fixtures/migrate/07_*` + `docs/MIGRATE.md` row; still clean-room.

### H5-10 — Truck G1 remainder (honest STEP or refuse)
**Goal:** Close the H3-6 hole: truck-brep **STEP write** if the pinned crates expose it;
otherwise `CADRION-E-UNSUPPORTED` and bid **N2 stays No**.  
**Honesty:** `parity_eligible` stays false; default kernel unchanged; cite G1 in the PR.  
**Depends on:** H3-6.  
**Exit:** one golden **or** a documented refuse + `docs/TRUCK_PARITY_BID.md` N2 line;
no suite lie.

---

## 4. Explicitly NOT Horizon-5

| Item | Why |
|------|-----|
| Live harness re-score / MTP / Q8 | Spend + ApexRouter `resolve_offer`; André will reopen when the sibling is fixed |
| Hermes `mcp_servers.cadrion` field pass | Parked this session — not a product slice |
| truck as **default** / `parity_eligible` | G1–G7 + D4 amendment |
| User `load()` / filesystem modules | D9 hermetic |
| Full AP242 STEP joints | OQ-4 unbounded |
| Official MCP SDK | OQ-7 closed |
| Checksummed `engine install` fetch | H4-2 packaging; still no tarball |
| wgpu viewer / full PMI package | Parked |
| Multi-tenant SaaS | NG3 |
| SDF as primary | STEP-first |
| ApexOS assimilation | D5 standalone-first |

---

## 5. Parking → board map

| Source | H5 slot |
|--------|---------|
| A3 underspec prompts | **H5-1** |
| CLI align/frame/diff | **H5-2** |
| CLI export | **H5-3** |
| CLI fab check | **H5-4** |
| CLI engine/schema | **H5-5** |
| Empty MCP prompts | **H5-6** |
| OQ-2 dialect | **H5-7** |
| CLI robot | **H5-8** |
| OQ-3 migrator | **H5-9** |
| Truck bid G1 STEP | **H5-10** |
| Live ≥6 | **out** — next run, not this board |
| Hermes field | **out** |
| truck default | **out** |

**Cook rule:** each face slice (H5-2…H5-5, H5-8) updates `skills/cadrion` MCP tool list in
the same PR. `STDLIB_DEPTH.md` still claims OCCT cone≈cylinder — fix that lie in **H5-7**
(or the first PR that touches the file).

---

## 6. Cook cadence

1. One H5-N per branch `feat/h5-n-short-name` off freshly-fetched `origin/main`  
2. Exit criteria in the PR body  
3. Merge when CI green (same trust as H4)  
4. Tick this checklist + `BACKLOG.md` on merge  
5. Default next = next unchecked H5-N  
6. Truck work must cite bid G-criteria  

**Default next if “cook on” with no pref:** Horizon-5 **complete** — do not invent a board.

---

## 7. Checklist

- [x] **H5-1** Harness prompt honesty  
- [x] **H5-2** MCP/HTTP align · frame · diff  
- [x] **H5-3** MCP/HTTP export  
- [x] **H5-4** MCP/HTTP fab check  
- [x] **H5-5** MCP engine + schema  
- [x] **H5-6** MCP prompts  
- [x] **H5-7** OQ-2 dialect bite  
- [x] **H5-8** MCP/HTTP robot gen \| validate  
- [x] **H5-9** Migrator bite  
- [x] **H5-10** Truck G1 STEP or honest refuse  

---

## 8. One-line pitch

> **Horizon-5** is the leftover agent loop — fair tasks, the CLI verbs still missing from
> MCP/HTTP, a pinned dialect, and an honest truck STEP — no box required.
