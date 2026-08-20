# Cadrion-RS v1 exit metrics (PRD §12 / M6)

Living scorecard. A row is **green** only with evidence (command + date), not intent.

| # | Metric | Target | Status | Evidence |
|---|--------|--------|--------|----------|
| 1 | Hermetic Starlark → IR | refuse `load()`, stable IR | **green** | `cargo test -p cadrion-lang` |
| 2 | Mock kernel CI | default workspace tests OCCT-free | **green** | `.github/workflows/ci.yml` |
| 3 | Selectors + inspect | stable `#o…` + measure | **green** | S4/S5 CLI tests |
| 4 | Parity parts 1–4 | deterministic suite | **green** | `cargo test -p cadrion-bench` |
| 4b | Parity parts 1–10 | full mock suite | **green** | `bench run --suite parts1-10` |
| 24 | OCCT translate/rotate | GeomKernel | **green** | transform_smoke + parts5-10-occt |
| 25 | Live harness | `--cmd` / `@oracle` | **green** | agent10 live 10/10 |
| 26 | Stdlib depth H2 | sphere/cone/mirror/patterns | **green** | IR v2 + pattern_hub |
| 27 | OCCT transform H3 | no STEP thrash | **green** | transform_smoke + parts5-10-occt |
| 28 | Fillet/chamfer H4 | OCCT diagnostics + suite | **green** | fillet_smoke + fillet-occt |
| 29 | Viewer H5 | gcode scrub + robot jog | **green** | cli_snapshot view_once_* |
| 30 | Slicer/DFM H6 | gated SLICE + pcb.outline | **green** | cadrion-fab slicer/dfm tests |
| 31 | MCP H7 | resources + write_source policy | **green** | cadrion-mcp resources_* tests |
| 32 | Migrate H8 | build123d skeleton | **green** | migrate fixtures + unit tests |
| 33 | Klipper H9 | Moonraker gated adapter | **green** | printer_klipper tests |
| 34 | Truck H10 | experimental non-parity kernel | **green** | cadrion-truck tests |
| 35 | WASM H2-1 | mock IR in wasm32 | **green** | cadrion-wasm + CI wasm job |
| 36 | MCP H2-2 | OQ-7 stay hand-rolled | **green** | compliance matrix tests |
| 37 | Fab H2-3 | waterjet DFM + OctoPrint | **green** | octoprint + dfm tests |
| 38 | Harness H2-4 | published live score | **green** | oracle 10.0/10 2026-08-06 · no frontier (backends down) |
| 39 | MCP NDJSON | Hermes framing | **green** | hermes mcp test cadrion |
| 40 | Joints H2-5 | assembly + robot limits | **green** | validate_assembly + assembly validate CLI |
| 41 | Viewer H2-6 | mesh 3D + gcode/robot orbit | **green** | view --once mesh.json + cli_snapshot |
| 42 | Migrator H2-7 | Locations/extrude/fillet notes | **green** | migrate fixtures 04–05 + unit tests |
| 43 | PMI H2-8 | drawing packet + inspect dims | **green** | auto 20/60/100 on pmi block |
| 44 | SDF H2-9 | cadrion-sdf analytic + NRRD | **green** | sdf sample box/cyl |
| 45 | Truck bid H2-10 | parity evidence pack NO-GO | **green** | docs/TRUCK_PARITY_BID.md |
| 46 | Honesty H3-1 | cone refuse + suite fences | **green** | OCCT cone Unsupported; docs/KERNEL_HONESTY.md |
| 47 | MCP H3-3 | dims + assembly + sdf tools | **green** | 9 tools tools/list |
| 48 | Assembly H3-4 | kinematics + emit-robot | **green** | lid_hinge → URDF path |
| 49 | PMI H3-5 | viewer dim overlay | **green** | view --once drawing.json + canvas chips |
| 50 | Truck H3-6 | upstream truck-brep spike | **green** | box+cut+mesh; parity_eligible false |
| 51 | OCCT H3-7 | fillet-occt +13 L + cone refuse | **green** | vol 8794; cone Unsupported |
| 52 | DFM H3-8 | override + schema validate | **green** | drift refuse; override pin |
| 53 | Migrate/WASM H3-9 | Circle+extrude + inspect_ir | **green** | fixture 06; wasm inspect |
| 54 | Name H3-10 | OQ-1 Cadrion-RS | **green** | docs/NAME_OQ1.md; crates/bin renamed |
| 55 | Schema H4-1 | cadrion schema dump | **green** | cli_schema + ERROR_CATALOG |
| 56 | Engine H4-2 | engine info / fail-closed install | **green** | cli_engine; no fake fetch |
| 57 | HTTP H4-3 | measure / dims / sdf routes | **green** | http_api + OpenAPI paths |
| 58 | Inspect H5-2 | MCP/HTTP align · frame · diff | **green** | align_check/frame/diff + `/v1/inspect/*` |
| 59 | Export H5-3 | MCP/HTTP export | **green** | stl/gltf preview; mock STEP Unsupported |
| 60 | Fab H5-4 | MCP/HTTP fab check | **green** | DFM report cites profile version; no START |
| 61 | Engine/schema H5-5 | MCP/HTTP engine + schema | **green** | install fail-closed; mcp/errors dump |
| 62 | Prompts H5-6 | MCP prompts/list + get | **green** | loop · write_source · hermetic load |
| A1 | OCCT cone | no silent cylinder | **green** | H3-1 fail-closed |
| A2 | truck-seed label | version JSON tag | **green** | truck_implementation field |
| A3 | H3-2 frontier | live LLM score | **amber** | 4.0/10 miss 2026-08-20 Qwen3.8-27B Q6; H5-1 made prompts fair — score unchanged until a live re-run |
| 5 | Snapshot packet | multi-view PNG + orbit GIF | **green** | `cli_snapshot` tests |
| 6 | MCP stdio | Content-Length tools | **green** | `cargo test -p cadrion-mcp` |
| 7 | HTTP API | `/v1/*` + OpenAPI + jobs | **green** | `http_api` 5 tests |
| 8 | parts.lock fail-closed | checksum verify | **green** | `cadrion-parts` tests |
| 9 | URDF validate | urdf-rs parse | **green** | `cadrion-robot` + `robot validate` |
| 10 | DFM preflight | profile findings cite rules | **green** | `fab check` plate.flat.json |
| 11 | G-code check | bbox/temp/flavor | **green** | `fab gcode-check` sample.gcode |
| 12 | Printer start gates | allowlist+hash+confirm; no silent start | **green** | `printer` unit tests + dry-run |
| 13 | Skills export | claude-code **and** codex packs | **green** | `skills export --all` (S12) |
| 14 | Licensing review | dual MIT/Apache core; OCCT LGPL isolated | **green** | `docs/LICENSING.md` (S12) |
| 15 | Windows CI | `cargo test` on windows-latest | **green** | CI `windows` job (S12) |
| 16 | Fuzz / property | parsers don't panic on junk | **green** | property tests (S12) |
| 17 | Live OCCT STEP e2e | optional local | **green** | cal-block STEP + cut via AdHocShape |
| 18 | Live Bambu MQTT start | gated + network | **green** | `--live` after allowlist+hash+START |
| 19 | Face→DXF projection | from B-rep face ref | **green** | `fab dxf-face` (planar outline) |
| 20 | Agent harness ≥6/10 | external eval | **green** | `harness run --suite agent10` (scripted) |
| 21 | OCCT live topology inspect | faces/normals from B-rep | **green** | mesh-clustered topology |
| 22 | OCCT parity lane parts1-4 | expect.occt.json suite | **green** | `parts1-4-occt` (feature occt) |
| 23 | inspect align/frame/diff | CLI polish | **green** | `inspect align|frame|diff` |

**v1 ship bar (this table):** rows 1–16 green; 17–20 may remain amber/red with honesty notes.

Last updated: 2026-08-20 (H5-6 MCP prompts).  
Harness live log: [`HARNESS_LIVE.md`](HARNESS_LIVE.md).  
As-built companion: [`STATUS.md`](STATUS.md).
