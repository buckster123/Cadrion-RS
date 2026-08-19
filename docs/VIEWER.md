# Viewer — snaps · mesh 3D · G-code · robot jog (H5 + H2-6)

Loopback `cadrion view` stays a tiny HTTP server (no new crate).

## Targets

| Path | Kind | Artifact |
|------|------|----------|
| `.snap/` / `.cad.star` | star/snap | multi-view PNG + GIF + **`mesh.json`** (H2-6) |
| `.gcode` / `.gco` / `.nc` | gcode | `{stem}.view/path.json` + XY scrub **+ 3D path orbit** |
| `.robot.json` | robot | `{stem}.view/robot.json` + joint sliders (**3D stick FK**) |
| `.urdf` | — | validates only; jog needs `.robot.json` |

## Commands

```sh
# CI-friendly prepare
cargo run -p cadrion-cli -- view examples/studio/stellar_crown.cad.star --once --json
cargo run -p cadrion-cli -- view examples/fab/sample.gcode --once --json
cargo run -p cadrion-cli -- view examples/robots/simple_arm.robot.json --once --json

# Serve (browser)
cargo run -p cadrion-cli -- view \
  examples/studio/stellar_crown.cad.star \
  examples/fab/sample.gcode \
  examples/robots/simple_arm.robot.json
# open http://127.0.0.1:7411/
```

## H2-6 — coarse 3D

### Mesh (`.cad.star`)
- Preview mesh from `mesh_from_ir` written as `mesh.json` (positions + indices + bbox)
- Canvas painter’s algorithm, drag to orbit, backface cull
- Still shows static PNG/GIF snapshot grid below

### G-code
- Keeps layer XY scrub
- Adds second canvas: 3D path with orbit; layer slider filters cumulative points

### Robot
- Full 3D stick FK (4×4 matrices, axis-angle revolute / prismatic)
- Drag canvas to orbit; joint sliders with limits

## H3-5 — PMI overlay

On `.cad.star` prepare, viewer writes `drawing.json` into the snap dir:

1. Prefer sibling `*.drawing.json` (from `inspect dims`)
2. Else **auto** opposite-face dims (same as H2-8)

The mesh canvas shows **dim value chips** (HUD) + a list under the canvas.  
**Not** leader lines / sheets / GD&T — not a drafting package.

```sh
cargo run -p cadrion-cli -- inspect dims examples/pmi/block.cad.star --json
cargo run -p cadrion-cli -- view examples/pmi/block.cad.star --once --json
# → examples/pmi/block.snap/drawing.json + mesh
cargo run -p cadrion-cli -- view examples/pmi/block.cad.star
# open http://127.0.0.1:7411/ → dim chips on orbit canvas
```
