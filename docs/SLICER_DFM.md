# H6 — Gated slicer execute + second DFM profile

## Slicer gates

| Gate | Rule |
|------|------|
| mesh_exists | input file on disk |
| allowlist | empty = any slicer; else basename/path must match |
| confirm | required **only** with `--execute`; value must be exactly `SLICE` |

Default is **preview** (prints command, no spawn). Execute never runs if gates fail.

```sh
cargo run -p cadrion-cli -- fab slice mesh.stl --json
cargo run -p cadrion-cli -- fab slice mesh.stl --execute --confirm SLICE \
  --allowlist prusa-slicer -o out.gcode --json

# tests / stubs
cargo run -p cadrion-cli -- fab slice mesh.stl --slicer-bin ./fake-slicer.sh \
  --execute --confirm SLICE --allowlist fake-slicer.sh -o out.gcode --json
```

## DFM profiles

| Id | Use |
|----|-----|
| `sendcutsend.laser` (default) | sheet metal laser |
| `pcb.outline` | FR4 / Aluminum PCB outline |

```sh
cargo run -p cadrion-cli -- fab profiles --json
cargo run -p cadrion-cli -- fab check --profile pcb.outline \
  --part-json examples/fab/pcb.flat.json --json
```

## Honesty

- Cadrion does not reimplement a slicer — it shells to host CLIs.
- Bundled DFM profiles are **versioned data**, not live vendor APIs.
- Printer live path remains separate (`--live` + `START`).
