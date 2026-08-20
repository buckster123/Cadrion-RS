# Horizon-3 — Cadrion depth, honesty, and optional pure-Rust BREP

**Status:** **active** board (chartered 2026-08-06 after Horizon-2 complete)  
**Predecessor:** [`docs/HORIZON2.md`](HORIZON2.md) — **archive complete** (H2-1…H2-10)  
**H1 archive:** [`docs/HORIZON.md`](HORIZON.md)

Horizon-3 is **not** “make truck default” and **not** multi-tenant SaaS.  
It is: **close honesty gaps**, **deepen the agent CAD loop**, and **optional BREP spikes**
behind explicit gates — STEP-first, consent-gated fab, hand-rolled MCP stay.

---

## 1. Theme

> After two full horizons, Cadrion is a real agent workstation. Horizon-3 makes the
> **facts truer**, the **loop thicker**, and the **pure-Rust path real enough to re-bid** —
> without surrendering mock-default CI or print safety.

Three pillars:

| Pillar | Intent |
|--------|--------|
| **A — Honesty** | Fix known lies / amber metrics (cone, truck naming, suite fences) |
| **B — Agent loop** | Live harness frontier, MCP/PMI/assembly depth agents actually hit |
| **C — BREP options** | Truck **spike** toward G1–G3 of the bid — still non-default |

---

## 2. Ranking principles (same house rules)

| # | Principle | Prefer | Defer |
|---|-----------|--------|-------|
| P1 | Agent loop payoff | inspect/MCP/harness/fab that agents call | pretty-only UI |
| P2 | Honesty > feature count | fix fake success, amber metrics | silent “pass” |
| P3 | Known-working now | gated spikes | default flips |
| P4 | Charter fit | STEP-first, hermetic, gated print | SDF-primary, SaaS |
| P5 | Bid discipline | truck work maps to `TRUCK_PARITY_BID` G-criteria | parity_eligible flip without suite |

---

## 3. Top-N board (ordered cook list)

### H3-1 — Honesty pass (cone + suite fences + truck naming)
**Goal:** Close residual product honesty: OCCT/mock cone≈cylinder disclosure or fix; suite docs state truck/mock limits; consider `backend_id` clarity (`truck-seed` note in version JSON already — tighten docs + any misleading strings).  
**Why:** Fake-adjacent geometry burns agent trust harder than missing features.  
**Depends on:** none.  
**Exit:** CHARTER/METRICS note; cone behavior documented or corrected; no new silent passes.

### H3-2 — Live harness frontier score (when backends exist)
**Goal:** When LocalRouter/ApexRouter (or equivalent) has a live backend, run harness `--cmd` against a real model, publish score JSON with provenance (model_id, cmd, notes) beside oracle control. **Never invent scores.**  
**Why:** H2-4 shipped oracle-only honesty; frontier number is the missing half.  
**Depends on:** user endpoint up.  
**Exit:** `harness/scores/h3-2-*.json` + `docs/HARNESS_LIVE.md` update; skip with documented blocker if backends down.

### H3-3 — MCP surface depth (dims + assembly validate + sdf sample)
**Goal:** Expose high-value CLI already on main via MCP tools (or resources): `inspect dims` drawing packet, `assembly validate`, optional `sdf sample` (secondary). Keep hand-rolled; write_source policy unchanged.  
**Why:** Hermes can drive build/snapshot today; dims/joints/sdf still CLI-only.  
**Depends on:** H2-5/H2-8/H2-9.  
**Exit:** tools/list grows; stdio NDJSON still green; docs HERMES_MCP / MCP_RESOURCES.

### H3-4 — Assembly / OQ-4 bite (STEP placement export or joint IR)
**Goal:** One honest step past H2-5: either (a) emit joint metadata sidecar with STEP, or (b) richer assembly IR consumed by robot gen — **not** full AP242.  
**Why:** OQ-4 still open; robots + assemblies are half-connected.  
**Depends on:** H2-5.  
**Exit:** example + validate path; CHARTER OQ-4 status note (still open or partial).

### H3-5 — PMI packet → viewer overlay alpha
**Goal:** Load `*.drawing.json` in `cadrion view` as dimension labels (canvas text), not a drafting package.  
**Why:** H2-8 facts are invisible unless JSON-read; agents + humans both benefit.  
**Depends on:** H2-6 + H2-8.  
**Exit:** view deep-link shows dim values; docs PMI honesty line.

### H3-6 — Truck BREP spike (bid G1 only)
**Goal:** Optional feature or gated path: wire **upstream truck** (or document why not) for box + boolean cut + mesh — **or** deepen seed with honest BREP-less limits. Must not set `parity_eligible`. Map explicitly to G1 in `TRUCK_PARITY_BID.md`.  
**Why:** H2-10 bid said spike is next; agents need a real experiment.  
**Depends on:** H2-10 bid.  
**Exit:** spike doc + tests; `parity_eligible` still false; default unchanged.

### H3-7 — OCCT parity depth (cone/sphere honesty + one hard part)
**Goal:** Improve OCCT expect path where mock/OCCT diverge; add or tighten one hard golden (e.g. filleted multi-body).  
**Why:** Agent fab path is OCCT; amber geometry is residual debt.  
**Depends on:** H3-1 preferred first.  
**Exit:** suite note + green OCCT local (CI still OCCT-free default).

### H3-8 — DFM / OQ-6 governance seed
**Goal:** Versioned DFM profile schema + one community-style override file format; document how profiles update without silent rule drift.  
**Why:** OQ-6 open; waterjet/laser/pcb exist.  
**Depends on:** H2-3.  
**Exit:** `docs/DFM_GOVERNANCE.md` + schema validation test.

### H3-9 — Migrator / WASM polish
**Goal:** Migrator: one more public-API pattern family **or** refuse-more-safely; WASM: expose one more IR fact surface for browser demos (still mock-only).  
**Why:** Onboarding + portable IR remain differentiators.  
**Depends on:** H2-1 / H2-7.  
**Exit:** fixture or wasm example + docs.

### H3-10 — Release / OQ-1 name decision packet
**Goal:** Decision packet for product/binary/crates.io name (OQ-1): options, trademark notes, rename cost estimate — **decide or explicitly defer with date**. No forced rename in-slice unless GO.  
**Why:** Public garden maturity; rename gets harder every crate.  
**Depends on:** none hard.  
**Exit:** `docs/NAME_OQ1.md` with GO/DEFER; if GO, follow-up slice owns rename.  
**Done 2026-08-19:** **Cadrion / Cadrion-RS.** In-tree rename. First public crate = `cadrion-cli`. Legacy `CADRE_*` / `cadre://` accepted.

---

## 4. Explicitly NOT Horizon-3

| Item | Why |
|------|-----|
| truck as **default** kernel | Needs full bid G1–G7 + CHARTER D4 amendment |
| `parity_eligible() = true` without suite | Honesty invariant |
| Multi-tenant public SaaS | Charter NG3 |
| SDF as primary modeling | STEP-first |
| Weaken print/slice/START gates | Safety |
| Full drawing package / GD&T / AP242 complete | Unbounded; stay alpha bites |
| Official MCP SDK dual stack | OQ-7 closed stay hand-rolled unless reopen criteria |

---

## 5. Parking → board map

| Source | H3 slot |
|--------|---------|
| Cone / amber geometry honesty | **H3-1** |
| Frontier harness score | **H3-2** |
| MCP dims/assembly/sdf | **H3-3** |
| OQ-4 joints/STEP | **H3-4** |
| PMI visible in viewer | **H3-5** |
| Truck bid G1 spike | **H3-6** |
| OCCT hard goldens | **H3-7** |
| OQ-6 DFM governance | **H3-8** |
| Migrator / WASM | **H3-9** |
| OQ-1 name | **H3-10** |
| truck default | **out** — post-bid only |
| SaaS | **out** |

---

## 6. Cook cadence

1. One H3-N per branch `feat/h3n-short-name`  
2. Exit criteria in PR body  
3. Agent merges when CI green (user trust)  
4. Tick checklist + BACKLOG on merge  
5. Default next = next unchecked H3-N  
6. Truck work must cite bid G-criteria in PR  

**Default next if “cook on” with no pref:** **H3-2** when a *healthy* router backend exists.
Otherwise cook [`docs/HORIZON4.md`](HORIZON4.md). Horizon-3 cook list is complete except H3-2
(H3-10 closed 2026-08-19; backends still down 2026-08-20).

---

## 7. Checklist

- [x] **H3-1** Honesty pass (cone + fences + truck naming)  
- [ ] **H3-2** Live harness frontier score *(blocked: backends still down 2026-08-20)*  
- [x] **H3-3** MCP surface depth  
- [x] **H3-4** Assembly / OQ-4 bite  
- [x] **H3-5** PMI → viewer overlay  
- [x] **H3-6** Truck BREP spike (G1)  
- [x] **H3-7** OCCT parity depth  
- [x] **H3-8** DFM / OQ-6 governance seed  
- [x] **H3-9** Migrator / WASM polish  
- [x] **H3-10** OQ-1 name decision packet — Cadrion-RS (`docs/NAME_OQ1.md`)  

---

## 8. One-line pitch

> **Horizon-3** makes Cadrion’s answers truer and its agent surfaces thicker, while running a
> disciplined pure-Rust BREP spike that can only earn parity the hard way.

---

## 9. Suggested first cook

**(a) H3-1 Honesty pass** — recommended first (trust foundation).  
**(b) H3-3 MCP depth** — if you want Hermes power immediately.  
**(c) H3-6 Truck spike** — if BREP curiosity is the energy.
