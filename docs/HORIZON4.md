# Horizon-4 — Cadrion contract surfaces

**Status:** **active** board (chartered 2026-08-20 after Horizon-3 cook list complete except blocked H3-2)  
**Predecessor:** [`docs/HORIZON3.md`](HORIZON3.md) — H3-1…H3-10 done; **H3-2** still blocked (no healthy LLM backend)

Horizon-4 is **not** a new CAD kernel and **not** SaaS.  
It closes **already-chartered** face gaps: machine-readable schema (D13), honest engine inventory (D4), and HTTP catching up to MCP (D5).

---

## 1. Theme

> Horizon-3 thickened the loop. Horizon-4 makes the three faces **describe themselves**
> and **stay aligned**, without inventing new modeling scope.

---

## 2. Ranking (same house rules)

Prefer agent-callable contract work. Defer truck-default, AP242, and paid harness scores.

---

## 3. Top-N board

### H4-1 — `cadrion schema` (D13)
**Goal:** Dump live CLI / MCP / API / error surfaces. CI pins MCP names and OpenAPI to the dump.  
**Honesty:** not a generated JSON Schema of every response body — clap + `tool_defs` + `openapi_doc` + `ERROR_CATALOG`.  
**Depends on:** none.  
**Exit:** `cadrion schema [--json] [cli|mcp|api|errors]`; catalog + drift tests green.

### H4-2 — `cadrion engine info|install` (D4)
**Goal:** Honest kernel/engine inventory. `install` does **not** fake a download — refuse with a real rebuild/fetch hint until a checksummed artifact exists.  
**Depends on:** H4-1 preferred (ENGINE-MISSING already catalogued).  
**Exit:** `engine info` JSON; `engine install` fail-closed; docs `occt-binding.md` status line.

### H4-3 — HTTP face catch-up (D5)
**Goal:** Local `/v1` grows the H3-3 MCP verbs agents already have: measure, inspect dims, sdf sample.  
**Depends on:** H3-3.  
**Exit:** routes + OpenAPI + HTTP tests; `cadrion schema api` lists the new paths.

### H3-2 (still Horizon-3, still blocked)
Live harness frontier score when ApexRouter/LocalRouter has a **healthy** backend.  
Re-probe 2026-08-20: process up, **all backends `status=down`**, aliases unhealthy. No invented score.

---

## 4. Explicitly NOT Horizon-4

| Item | Why |
|------|-----|
| truck as default | Needs bid G1–G7 + D4 amendment |
| Paid / frontier harness score | H3-2; backends down; spend gated |
| Full AP242 STEP joints | OQ-4 unbounded |
| Official MCP SDK | OQ-7 closed |
| Silent `load()` | hermetic; needs charter + sandbox |

---

## 5. Cook cadence

1. One H4-N per branch `feat/h4-n-short-name`  
2. Merge when CI green (user trust, 2026-08-20)  
3. Tick checklist + BACKLOG on merge  

**Default next:** **H4-3**. If a healthy router backend appears, **H3-2** jumps the queue.

---

## 6. Checklist

- [x] **H4-1** `cadrion schema`  
- [x] **H4-2** `cadrion engine info|install`  
- [ ] **H4-3** HTTP measure / dims / sdf  

---

## 7. One-line pitch

> **Horizon-4** makes Cadrion’s faces tell the truth about themselves — schema, engine, HTTP — so agents do not have to guess.
