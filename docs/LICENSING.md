# Licensing review (S12 / M6)

## Core (default workspace)

| Component | License | Notes |
|-----------|---------|-------|
| All default crates (`cadre-*` except occt) | **MIT OR Apache-2.0** | Dual, Rust convention |
| Skill pack prose under `skills/` | MIT OR Apache-2.0 with the repo | Agent-exportable |
| Docs | same as repo | |

SPDX on packages: `license.workspace = true` → `MIT OR Apache-2.0`.

## Optional OCCT backend

| Component | License | Notes |
|-----------|---------|-------|
| `cadre-occt` | **LGPL-2.1** | Declared in its `Cargo.toml` |
| Open CASCADE Technology | LGPL-2.1 with OCCT exception | System / linked dependency |

**Isolation rules (binding):**
1. `cadre-occt` is a workspace member but **not** a default-member and **not** in default CI.
2. Default CLI/API/MCP builds do **not** link OCCT.
3. Enabling OCCT is opt-in: `cargo build -p cadre-cli --features occt`.
4. Distributing a binary **with** OCCT requires LGPL compliance for the combined work
   (offer object files / relink path as applicable). Distros should document this.

## Third-party Rust crates

Pulled via crates.io under their own licenses (MIT/Apache-heavy).

**H3-6 truck spike (optional `cadre-truck` feature `brep`):**

| Crate | Pin | License |
|-------|-----|---------|
| `truck-modeling` | 0.6 | Apache-2.0 |
| `truck-shapeops` | 0.4 | Apache-2.0 |
| `truck-meshalgo` | 0.4 | Apache-2.0 |

Not in default CLI/MCP binaries. No copyleft contamination of default path.


```sh
cargo install cargo-license
cargo license --avoid-dev-deps
```

## Skill packs / generated installs

Exported packs under `dist/skills/**` inherit repo dual-license. Do not bundle proprietary
vendor PDFs or SendCutSend trademarks beyond the honest “-style profile” naming.

## Printer / vendor profiles

Bundled DFM profile is **Cadre-authored** data inspired by public process rules, not a
vendor SDK and not a live quote API. No vendor license is implied.

## Review conclusion (2026-08-05)

- Core dual-license: **OK for 1.0**.
- OCCT LGPL fence: **OK** (optional, non-default, documented).
- No known copyleft contamination of default binaries.
- Residual: run `cargo license` before any binary GitHub Release asset and attach output.
