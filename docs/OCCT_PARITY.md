# OCCT parity depth (H3-7)

Default CI stays **OCCT-free**. This note is for local `--features occt` runs.

## Honesty

| Prim | Mock | OCCT |
|------|------|------|
| box / cyl | analytic | B-rep |
| sphere | analytic 4/3πr³ | `Shape::sphere` (H3); volume band ~15% tessellation |
| cone | analytic πr²h/3 | **Unsupported** (H3-1) — no MakeCone in sys 0.2; **no cylinder stand-in** |
| fillet / chamfer | Unsupported | real MakeFillet / MakeChamfer + IsDone |

## Suites

| Suite | Parts | Expect |
|-------|-------|--------|
| `parts1-4-occt` | 01–04 | `expect.occt.json` |
| `parts5-10-occt` | 05–10 | `expect.occt.json` |
| `fillet-occt` | 11 plate+fillet · 12 chamfer brick · **13 filleted L** | `expect.occt.json` only |

Part **13** is the H3-7 hard golden: `union` of two plates then all-edge fillet.

```sh
CMAKE_POLICY_VERSION_MINIMUM=3.5 cargo test -p cadre-occt --test fillet_smoke
CMAKE_POLICY_VERSION_MINIMUM=3.5 cargo test -p cadre-bench --features occt fillet_occt
CMAKE_POLICY_VERSION_MINIMUM=3.5 cargo run -p cadre-cli --features occt -- \
  bench run --suite fillet-occt --json
```

Regenerate fillet goldens:

```sh
CMAKE_POLICY_VERSION_MINIMUM=3.5 cargo test -p cadre-bench --features occt \
  --test gen_expects_fillet -- --ignored --nocapture
```

## What this slice does **not** do

- Flip default kernel
- Add fillet to mock `parts1-10`
- Implement OCCT cone (still fail-closed)
