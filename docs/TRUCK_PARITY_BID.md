# Truck parity bid — evidence pack (H2-10)

**Status (2026-08-06):** bid **PREPARED**, decision **NO-GO for default / Parity-10**.  
**Not a promotion.** Horizon-3 may reopen when go criteria clear.

This document is the exit artifact for Horizon-2 **H2-10**. It does **not** flip the
default kernel, claim parity, or wire the upstream [`truck`](https://github.com/ricosjp/truck)
crate.

---

## 1. What exists today (`cadrion-truck` H10 seed)

| Item | Reality |
|------|---------|
| Trait | Implements `GeomKernel` |
| `backend_id` | `"truck"` (name only — **not** upstream truck B-rep) |
| `parity_eligible()` | **always `false`** (hard-coded) |
| Default CLI kernel | **never** — `mock` default; `occt` optional feature |
| Primitives | box, cylinder only |
| Booleans | **analytic volume/bbox approx** (`Approx` solid) — not BREP CSG |
| Fillet / chamfer | `Unsupported` |
| STEP R/W | unsupported |
| Tessellate | coarse **bbox** mesh |
| Sphere / cone / mirror / patterns | not on this kernel path |
| Upstream `truck` crates | **not** a default dependency; optional `brep` feature pins 0.6/0.4 |

See also: [`docs/TRUCK.md`](TRUCK.md).

### Binding honesty lines (must not regress)

```text
parity_eligible() == false
CLI default != truck
no Parity-10 claim under --kernel truck
```

`cargo run -p cadrion-cli -- version --json` reports `truck_parity_eligible: false`.

---

## 2. Gap matrix vs OCCT + Parity parts 1–10

Legend: **Y** = real enough for agent fab loops · **~** = approximate / honesty-limited · **N** = missing

| Capability | mock | OCCT | cadrion-truck today | Parity-10 need |
|------------|------|------|-------------------|----------------|
| box / cyl | Y | Y | Y | Y |
| sphere / cone | Y (IR) | Y | N | Y (parts 05+) |
| boolean BREP | ~ IR | Y | N seed / **Y spike** (`truck-shapeops`) | Y |
| fillet | N | Y | N | Y (parts 11+) |
| chamfer | N | Y | N | Y |
| translate / rotate / mirror | IR | Y | N on kernel | Y |
| polar/linear patterns | IR | via IR+exec | N | Y (impeller etc.) |
| STEP write (primary artifact) | N (.ir.json) | Y | N | Y for fab |
| STEP read | N | Y | N | useful |
| topology snapshot (faces/edges) | IR analytic | mesh/topo | weak Approx | Y for inspect |
| measure / PMI dims | via IR topo | via OCCT topo | weak | Y |
| tessellate for snapshot/GLB | coarse | real | bbox seed / **triangulation spike** | Y for viz |
| volume/facts accuracy | analytic | mesh-approx | analytic/approx | amber OK if disclosed |
| LGPL / link hygiene | n/a | LGPL-2.1 occt | pure MIT/Apache seed | dual-license path if truck crates added |

### Parity parts (illustrative)

| Part | Why truck fails today |
|------|------------------------|
| 01 calibration block | would “pass” volume only — no STEP |
| 02 bolt circle | patterns + holes need real cut BREP |
| 03 L-bracket | booleans must be BREP |
| 04 stepped shaft | stacked cyl + cuts |
| 05–07 enclosures / fins | multi-boolean |
| 08 impeller | polar pattern + cuts |
| 09 spiral / 10 planetary | transforms + many bodies |
| 11–12 fillet/chamfer | unsupported |

**Conclusion:** truck lane cannot enter `parts1-10` or `parts*-occt` suites without lying.

---

## 3. What a real pure-Rust B-rep bid would need

### A. Geometry stack (choose one primary path)

1. **Upstream truck binding** (`truck-modeling` / `truck-topology` / `truck-meshalgo` / STEP via truck-step or similar)  
   - Map `GeomKernel` ops → truck solid ops  
   - Fillet/chamfer: confirm truck coverage or keep Unsupported honestly  
2. **Alternate pure-Rust BREP** (if truck gaps block fillet) — research spike only  
3. **Hybrid:** truck for prims+boolean; OCCT remains fillet/STEP until truck catches up — **must not** be default dual-brain confusion; feature-gate only

### B. Product surface (minimum)

- [ ] Real boolean topology (not volume arithmetic)
- [ ] STEP write round-trip golden (or documented subset)
- [ ] Tessellation usable for snapshot/MCP mesh.json
- [ ] Topology snapshot compatible with `inspect refs` / dims
- [ ] Sphere, cone, transform, mirror (or explicit Unsupported + IR limitation)
- [ ] Fillet **or** documented permanent Unsupported with suite skip (not silent zero)

### C. Process / honesty

- [ ] `parity_eligible()` flips only after suite green **and** CHARTER D4 amendment
- [ ] Default kernel remains mock/occt until separate decision
- [ ] CI: truck tests always; optional truck parity job **never** required for main green until eligible
- [ ] License review if truck crates pulled (Apache/MIT-ish — verify at pin time)
- [ ] Name clarity: `backend_id` may need `"truck-seed"` vs `"truck"` when real crate lands

### D. Effort guess (order-of-magnitude, not a promise)

| Phase | Scope | Rough |
|-------|--------|-------|
| Spike | wire truck box/cyl + one boolean + mesh | days–1 week |
| STEP | write path + one golden | 1–2 weeks |
| Transforms + more prims | match mock IR set | 2–4 weeks |
| Fillet/chamfer | if truck supports; else stay Unsupported | unknown |
| Parity suite | parts1-10 under truck | multi-week after BREP real |

---

## 4. Go / no-go criteria

### NO-GO (current — **binding until amended**)

| # | Criterion | Met? |
|---|-----------|------|
| N1 | Real BREP booleans | **Partial (H3-6)** — spike `and`/`or`/`cut`; not suite-proven |
| N2 | STEP primary artifact path | **No** |
| N3 | Fillet path or honest permanent skip policy in suite | **No** |
| N4 | Parity suite green under truck | **No** |
| N5 | Agent fab loops prefer truck over OCCT on real parts | **No** (OCCT remains) |
| N6 | CHARTER D4 amended to allow truck default/parity | **No** |

**Decision:** **NO-GO** for default kernel and for `parity_eligible() = true`.

### GO (all required to reopen promotion bid)

| # | Criterion |
|---|-----------|
| G1 | BREP boolean + tessellate + STEP write on ≥ parts 01–04 class (**H3-6 partial:** boolean+mesh; no STEP) |
| G2 | `parity_eligible` still false until G3 |
| G3 | Documented suite (`parts1-10` truck or explicit subset) green in CI optional job |
| G4 | Fillet: implemented **or** CHARTER + suite mark fillets OCCT-only forever |
| G5 | Agent loop smoke: build → inspect dims → snapshot without fake success |
| G6 | Dated CHARTER amendment for any default or parity flip |
| G7 | License + supply-chain pin review (**H3-6 started:** Apache-2.0 truck-modeling 0.6 / shapeops 0.4 / meshalgo 0.4) |

Promotion target earliest: **Horizon-3** (not H2).

---

## 5. Explicit non-goals (this bid)

- No default flip in this PR / H2-10  
- No `parity_eligible() = true`  
- No silent volume-as-boolean “wins”  
- No claiming upstream truck integration before Cargo.toml depends on it
  (H3-6: optional `brep` feature **does** depend — seed path still does not)  
- No blocking main CI on truck parity  

---

## 6. Recommended next engineering

1. ~~Spike: `truck-modeling` box + boolean cut + mesh~~ **done H3-6** (`--kernel truck-brep`)
2. Seed remains `--kernel truck` (analytic)
3. Golden: single STEP from truck vs OCCT volume delta (still open — G1 remainder)
4. Only then draft Parity-truck suite

## 7. Sign-off

| Role | Statement |
|------|-----------|
| H2-10 exit | Evidence pack complete |
| H3-6 spike | Upstream truck wired behind feature; default/parity **unchanged NO-GO** |
| Default kernel | unchanged (`mock` / optional `occt`) |
| Parity-10 | OCCT + mock paths only |
| Reopen | when G1–G7 met + CHARTER amendment |

*Prepared 2026-08-06 · H3-6 update 2026-08-14.*
