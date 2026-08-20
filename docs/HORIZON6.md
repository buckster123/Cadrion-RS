# Horizon-6 — remaining named faces

**Status:** **ACTIVE** (board chartered 2026-08-21)  
**Predecessor:** [`docs/HORIZON5.md`](HORIZON5.md) — H5-1…H5-10 **complete**  
**H3 note:** H3-2 live score **4.0/10** published 2026-08-20 ([PR #60](https://github.com/buckster123/Cadrion-RS/pull/60)) — miss ≥6, not invented. Not re-opened here.

Horizon-6 is **not** a new kernel, **not** a live GPU re-score, and **not** SaaS.  
It ships the **already-named** `docs/design.md` MCP/CLI verbs that crates already implement
and agents still cannot call on all three faces.

---

## 1. Theme

> Horizon-5 closed leftover inspect / export / fab-check / engine / schema / robot.  
> Horizon-6 finishes the **contract list** — parts, viewer deep-link, slice / gcode,
> printer gates, migrate, assembly emit — without inventing upload, AP242, or `load()`.

---

## 2. Ranking (same house rules)

| # | Prefer | Defer |
|---|--------|--------|
| P1 | Names already in `design.md` MCP table or CLI table | Pretty-only viewer / wgpu |
| P2 | Crate + CLI (or HTTP) already exists | New vendor APIs / registries |
| P3 | Fail-closed gates stay fail-closed (D10) | Convenience defaults on `START` / `SLICE` |
| P4 | OQ bites that do **not** need a new decision | OQs that reopen D9 / D4 / NG3 |

D12 still binds: prefer one tool + `op=` per family (`parts`, `printer`) rather than
a tool per subcommand. Do **not** rename shipped tools (`fab_check`, `assembly_validate`).

---

## 3. Top-N board (all unblocked)

Every slice below is cookable on a laptop with default mock CI. None need Vast, ApexRouter,
Hermes field, or a paid model.

### H6-1 — CLI + MCP/HTTP `parts` (search \| fetch \| lock)
**Goal:** `design.md` already names `cadrion parts search|show|fetch|lock` and MCP
`parts_search` / `parts_fetch`. HTTP has `/v1/parts/search`. CLI has **no** `parts`
verb. `LocalFsProvider` already searches / fetches / pins checksums.  
**Honesty:** local filesystem catalog only — not a vendor storefront. Lock stays
fail-closed on sha256 miss. `show` is fetch (same payload).  
**Depends on:** S9 `cadrion-parts` (shipped).  
**Exit:** `cadrion parts search|fetch|lock`; MCP `parts` (`op=search|fetch|lock`);
`POST /v1/parts/fetch` (search already exists); `examples/assembly/parts` +
`parts.lock` green.

### H6-2 — MCP/HTTP `viewer_open`
**Goal:** CLI `cadrion view` already prints loopback deep links. MCP name is
`viewer_open`.  
**Honesty:** software / loopback HTML — not a desktop CAD GUI, not wgpu. Tests use
`--once` (or equivalent) so CI does not hang on the accept loop.  
**Depends on:** S7 viewer (shipped).  
**Exit:** tool + `POST /v1/viewer/open`; JSON includes `url` / links; no claim of
interactive 3D CAD.

### H6-3 — MCP/HTTP `gcode_check`
**Goal:** CLI `fab gcode-check` is static bbox / temp / flavor. Agents cannot call it.  
**Honesty:** not a slicer and not a print. Payload must not imply `printer_start`.  
**Depends on:** S11 / H5-4.  
**Exit:** `gcode_check` + `POST /v1/fab/gcode-check`; existing sample G-code fixture
green.

### H6-4 — MCP/HTTP `fab_slice`
**Goal:** CLI `fab slice` already previews argv and gates execute behind
`--confirm SLICE`.  
**Honesty:** default is **preview only**. Host slicer runs only with explicit
`confirm=SLICE` + execute. Payload `slice_executed: false` unless that gate passed.  
**Depends on:** H1-6 slicer path (shipped).  
**Exit:** `fab_slice` + `POST /v1/fab/slice`; missing confirm on execute → refuse;
no silent `std::process::Command`.

### H6-5 — MCP/HTTP printer status + dry-run
**Goal:** CLI `printer status|dry-run` is metadata + gcode-check + sha256. No
network in the default path.  
**Honesty:** `live: false`. Status does not poll MQTT.  
**Depends on:** S11 adapters (shipped).  
**Exit:** MCP `printer` (`op=status|dry_run`) + `POST /v1/printer/status` and
`/dry-run`; D12: one tool, not two.

### H6-6 — MCP/HTTP `printer_start`
**Goal:** D10 on the agent faces. CLI already requires allow-list + sha256 +
`confirm=START` and `--live` for network.  
**Honesty:** `design.md` also names `printer_upload` — **CLI has no upload verb**.
Do **not** invent FTPS/MQTT upload this slice. Start is the effect; refuse or
point at start when an agent asks for upload.  
**Depends on:** H6-5 (same `printer` tool).  
**Exit:** `op=start` + `POST /v1/printer/start`; missing confirm / allow-list /
hash → refuse; tests never open a socket to a printer.

### H6-7 — MCP/HTTP `migrate`
**Goal:** CLI `migrate` exists (H8…H5-9). Agents still drop to shell.  
**Honesty:** skeleton only. Sweep / Workplane / faces / loft / scale stay
structured refuse. Not a new mapping family (that would be a later OQ-3 bite).  
**Depends on:** H5-9.  
**Exit:** `migrate` + `POST /v1/migrate`; fixture `07_sweep_workplane.py` still
notes-not-faked; still clean-room.

### H6-8 — MCP/HTTP assembly emit
**Goal:** CLI `assembly emit-kinematics` / `emit-robot` (H3-4). MCP has validate
only.  
**Honesty:** labels + placements → sidecar / robot JSON. **`ap242: false`.**
No STEP joint entities.  
**Depends on:** H3-4.  
**Exit:** emit on MCP + `POST /v1/assembly/emit-kinematics` and `/emit-robot`
(or one route + `kind`); existing hinge / arm fixtures green; do not rename
`assembly_validate`.

### H6-9 — MCP/HTTP fab dxf \| dxf-face \| profiles
**Goal:** Remaining fab CLI: plate DXF, planar face→DXF, bundled profile list.  
**Honesty:** Cadrion-authored DXF / profile ids — not a vendor quote API.
`slicers` discover stays CLI unless it fits the same tool without blowing D12.  
**Depends on:** S11 / H3-8.  
**Exit:** tool(s) + HTTP; mock writes a DXF file; `fab profiles` lists bundled
ids + versions.

### H6-10 — `project_artifacts` + OpenAPI honesty
**Goal:** `design.md` names `project_artifacts`. `cadrion://artifact/index`
already lists local IR/snap/gcode. OpenAPI is still a hand list (`SCHEMA.md`).  
**Honesty:** the tool wraps the existing index — it does not invent a second
artifact store. OpenAPI paths added in H6-1…H6-9 must appear in `openapi_doc`
and `cadrion schema api`.  
**Depends on:** H6-1…H6-9 preferred (catch-up PR is fine if a path was missed).  
**Exit:** `project_artifacts` tool; SCHEMA.md lists every new `/v1` path; dump
tests still match live functions.

---

## 4. Explicitly NOT Horizon-6

| Item | Why |
|------|-----|
| Live harness re-score / MTP / Q8 | Spend + ApexRouter `resolve_offer`; A3 stays amber |
| Hermes `mcp_servers.cadrion` field pass | Parked — not a product slice |
| truck as **default** / `parity_eligible` | G1–G7 + D4 amendment; H5-10 STEP is refuse |
| User `load()` / filesystem modules | D9 hermetic |
| Full AP242 STEP joints | OQ-4 unbounded |
| Official MCP SDK | OQ-7 closed |
| Checksummed `engine install` fetch | Still no tarball |
| New `printer_upload` FTPS/MQTT | Not a CLI verb; do not invent |
| Community DFM registry | OQ-6 seed only |
| wgpu viewer / full PMI package | Parked |
| Multi-tenant SaaS | NG3 |
| SDF as primary | STEP-first |
| ApexOS assimilation | D5 standalone-first |

---

## 5. Parking → board map

| Source | H6 slot |
|--------|---------|
| `design.md` parts CLI + MCP | **H6-1** |
| `viewer_open` | **H6-2** |
| `gcode_check` | **H6-3** |
| `fab_slice` | **H6-4** |
| `printer_status` / `printer_dry_run` | **H6-5** |
| `printer_start` (not upload) | **H6-6** |
| CLI migrate | **H6-7** |
| CLI assembly emit | **H6-8** |
| CLI fab dxf / profiles | **H6-9** |
| `project_artifacts` + OpenAPI list | **H6-10** |
| Live ≥6 | **out** — next run, not this board |
| Hermes field | **out** |
| truck default / `load()` / AP242 | **out** |

**Cook rule:** each face slice updates `skills/cadrion` MCP tool list in the same
PR. New `CADRION-E-*` codes go in `ERROR_CATALOG` in the same PR as the first emit.

---

## 6. Cook cadence

1. One H6-N per branch `feat/h6-n-short-name` off freshly-fetched `origin/main`
2. Exit criteria in the PR body
3. Merge when CI green (same trust as H5)
4. Tick this checklist + `BACKLOG.md` on merge
5. Default next = next unchecked H6-N

**Default next if “cook on” with no pref:** **H6-2**.

---

## 7. Checklist

- [x] **H6-1** CLI + MCP/HTTP parts search \| fetch \| lock
- [ ] **H6-2** MCP/HTTP viewer_open
- [ ] **H6-3** MCP/HTTP gcode_check
- [ ] **H6-4** MCP/HTTP fab_slice
- [ ] **H6-5** MCP/HTTP printer status + dry-run
- [ ] **H6-6** MCP/HTTP printer_start (D10)
- [ ] **H6-7** MCP/HTTP migrate
- [ ] **H6-8** MCP/HTTP assembly emit
- [ ] **H6-9** MCP/HTTP fab dxf \| profiles
- [ ] **H6-10** project_artifacts + OpenAPI honesty

---

## 8. One-line pitch

> **Horizon-6** is the rest of the named contract — catalog, look, slice, gated
> print, migrate, emit — already in crates, still missing from the agent faces.
