# Parity suite

Deterministic geometry fixtures for Cadrion-RS (PRD §12 Parity-10).

## Parts 1–4 (M1)

| Id | Part | Notes |
|----|------|--------|
| `01_calibration_block` | Plate + 2×2 holes | mock volume uses hole height as authored |
| `02_bolt_circle_flange` | Disc + bore + 6 bolts | |
| `03_l_bracket` | L + gusset + 2-dir clearances | vertical hole = slot |
| `04_stepped_shaft` | 3-step shaft + keyway | |

## Parts 5–10 (stdlib translate/rotate)

| Id | Part | Notes |
|----|------|--------|
| `05_open_enclosure` | Open-top box + floor bosses | shell via cut |
| `06_clevis_bracket` | Base + dual ears + pin/light holes | |
| `07_finned_cylinder` | Hub + 6 fins + Y-rotated boss | uses `rotate`/`rotate_z` |
| `08_impeller` | Hub + 8 blades + shroud | polar `rotate_z` |
| `09_spiral_stair` | Post + 8 treads + rails | spiral via Z rotate |
| `10_planetary_stage` | Sun + ring + 3 planets + carrier | simplified |

Each directory:

```
part.cad.star   # reference model
expect.json     # volume/bbox/ops/params (mock goldens)
```

## Run

```sh
cargo test -p cadrion-bench
cargo run -p cadrion-cli -- bench run --suite parts1-4 --json
cargo run -p cadrion-cli -- bench run --suite parts5-10 --json
cargo run -p cadrion-cli -- bench run --suite parts1-10 --json   # full Parity-10
```

Volumes are calibrated against **MockKernel** analytic booleans so default CI stays OCCT-free.

### OCCT lane
```sh
CMAKE_POLICY_VERSION_MINIMUM=3.5 cargo test -p cadrion-occt --test transform_smoke
CMAKE_POLICY_VERSION_MINIMUM=3.5 cargo run -p cadrion-cli --features occt -- \
  bench run --suite parts5-10-occt
```

OCCT `translate` uses location; `rotate` uses `BRepBuilderAPI_Transform` via STEP
round-trip (opencascade `Shape.inner` is crate-private). Run OCCT tests single-threaded.

## Stdlib growth

- `translate(shape, dx, dy, dz)`
- `rotate(shape, "x"|"y"|"z", deg)` / `rotate_z(shape, deg)`
- IR v1: `Translate` / `Rotate` nodes
