# Cadrion-RS — Agent & Developer Guide

> Rust-native CAD runtime for AI agents: Starlark → B-rep → inspect → snapshot → fab.
> Single workspace; CLI + MCP + local HTTP as co-equal faces; skill pack teaches the loop.
> Standalone-first — agents and humans consume it directly; ApexOS assimilation is not assumed.

Bootstrapped 2026-08-05; **v1 surface S0–S12 complete** (2026-08-05). House conventions come
from `~/Projects/Launchpad-RS/` — load a doc from there when you need the detail behind a rule.

**Read `docs/CHARTER.md` before any non-trivial change — its decisions log (D1–Dn) is
binding.** Amend it with a dated entry when a decision changes, never silently. Where the
charter and this file disagree, the charter wins.

**Live status:** `docs/STATUS.md` · **metrics:** `docs/METRICS.md` · **gotchas:** `docs/gotchas.md`.

Reference/read-only: public docs of `earthtojake/text-to-cad` for *behavior* only — **do not
clone its source into this tree or translate it** (charter D1). Product PRD: `docs/cadrion-prd.md`.

---

## What this is

Cadrion gives coding agents a hermetic hardware-design loop without a Python CAD stack: author
parametric geometry in Starlark, build B-rep via MockKernel (default CI) or optional OCCT,
verify with numeric selectors/facts, review with snapshots/viewer, then export, source parts,
emit robot descriptions, or hand off to fabrication under consent gates. The skill pack is half
the product — tools without doctrine are not parity.

```
crates/  (as-built)
  cadrion/            # facade re-exports
  cadrion-kernel/     # GeomKernel trait + MockKernel
  cadrion-occt/       # OCCT backend (LGPL; non-default CI)
  cadrion-lang/       # hermetic Starlark → IR + execute_ir
  cadrion-model/      # selectors + BuildCache
  cadrion-inspect/    # refs / measure
  cadrion-render/     # software z-buffer PNG + orbit GIF
  cadrion-bench/      # parity parts 1–4 runner
  cadrion-mcp/        # stdio MCP (6 tools)
  cadrion-api/        # Axum /v1 + jobs/SSE/OpenAPI
  cadrion-parts/      # parts.lock + provider + assembly
  cadrion-robot/      # URDF/SRDF/SDF + urdf-rs
  cadrion-fab/        # DXF, DFM, slicer discover, gcode, printers
  cadrion-cli/        # `cadrion` binary
docs/STATUS.md      # live as-built status
docs/design.md      # THE contract
docs/cadrion-prd.md   # full PRD
BACKLOG.md          # S0–S12 done + post-v1 parking
```

Parked names from early design (not crates): `cadrion-truck`, `cadrion-export`, `cadrion-viewer`,
`cadrion-skills` — responsibilities folded into occt/cli/render/mcp as documented in STATUS.

---

## Locked decisions

The load-bearing summary; **`docs/CHARTER.md` D1–Dn is the binding long form.**
**Locked means locked — do not re-litigate these mid-session; amend deliberately, with a date.**

- **Language**: Rust — one Cargo workspace, every binary in it
- **Kernel**: OCCT via FFI is the *optional* production backend (D4/D19); default CI is MockKernel.
  Bind path: `docs/occt-binding.md`. `cadrion-kernel` is pure; OCCT stays optional.
- **Authoring**: Starlark `.cad.star`, hermetic, STEP-first when OCCT enabled (D2, D3)
- **License**: MIT OR Apache-2.0 dual for core; OCCT separate LGPL engine component (D6)
- **Faces**: CLI + MCP + local Axum HTTP; single schema source (D5, D13)
- **MCP**: stdio hand-rolled (OQ-7 SDK deferred); stdout sacred
- **HTTP**: `axum` in; `clap` for CLI; `serde` everywhere
- **CI**: fmt + clippy `-D warnings` + test + build; **ubuntu + windows** (S12)
- **Safety**: printer/vendor effects dry-run + allow-list + explicit confirm (D10)
- **No telemetry** (D14)
- **Cerebro agent**: `CADRION` (D15)
- **Name**: Cadrion / Cadrion-RS (OQ-1 closed); binary `cadrion` (D18)
- **Clean-room** vs text-to-cad (D1)

---

## The playbook (house method)

Full rationale: `~/Projects/Launchpad-RS/docs/house-doctrine.md`. Condensed:

1. **Contract first.** Pin wire/API in `docs/design.md`. Docs travel with code.
2. **Slices, not marathons.** One branch = one reviewable slice off `origin/main`.
3. **Honesty invariants.** Never fake success. Failures carry the real reason.
4. **Pure-fn test discipline.** Parsers/IR/selectors/validators unit-tested.
5. **Field truth beats green CI.** Ledger ✅ only after live verify.
6. **Secrets hygiene.** Never print keys; no credentials in CLAUDE.md.
7. **Cerebro is the thread.** `session_recall` / `session_save` with `agent_id=CADRION`.
8. **Spend is gated.** Paid ops never auto-fire.
9. **Cost the failure, not the happy path.** Recoverable pending beats orphan spend.

---

## Git discipline

- **Never commit to `main`.** Feature branch off freshly-fetched `origin/main`.
- **Ship via PR** (`gh pr create`). **Do NOT merge it yourself** unless André says so.
- **Commit format:** imperative, lowercase + `Co-Authored-By` trailer.
- **Never amend a pushed commit. Never force-push.**
- **Push after every commit.**

---

## Cerebro session protocol (mandatory)

All Cerebro MCP calls use agent `CADRION`.

**START:**
```
session_recall(query="Cadrion-RS build status", agent_id="CADRION")
```

**END / milestones:**
```
session_save(..., agent_id="CADRION", priority="HIGH")
```

Vaults: CLAUDE.md = lean core · `docs/STATUS.md` = as-built · `docs/gotchas.md` = invariants ·
Cerebro = session memory · git = code truth.

---

## Dev commands

```bash
cargo test
cargo fmt --all && cargo clippy -- -D warnings
cargo build -p cadrion-cli --release

cargo run -p cadrion-cli -- version --json
cargo run -p cadrion-cli -- schema --json
cargo run -p cadrion-cli -- engine info --json
cargo run -p cadrion-cli -- build parity/parts/01_calibration_block/part.cad.star --json
cargo run -p cadrion-cli -- snapshot parity/parts/01_calibration_block/part.cad.star --json
cargo run -p cadrion-cli -- mcp          # stdio; logs on stderr
cargo run -p cadrion-cli -- skills export --all -o dist/skills --json
cargo run -p cadrion-cli -- serve api --project examples/assembly --token dev
```

OCCT local:
```bash
CMAKE_POLICY_VERSION_MINIMUM=3.5 cargo test -p cadrion-occt
CMAKE_POLICY_VERSION_MINIMUM=3.5 cargo run -p cadrion-cli --features occt -- --kernel occt build … --json
```

---

## After v1

Post-v1 parking is in `BACKLOG.md`. Do not invent mid-session scope expansion past honesty
limits in `docs/METRICS.md` rows 17–20 without a charter amendment.
