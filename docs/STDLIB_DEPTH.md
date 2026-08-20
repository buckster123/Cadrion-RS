# Stdlib depth (H2)

IR schema **v2**. Additive nodes + stdlib expansions for agent-friendly patterns.

## Primitives

| Starlark | IR | Mock | OCCT |
|----------|----|------|------|
| `sphere(r, at=CENTER)` | `sphere` | true 4/3πr³ | `MakeSphere` + STEP place |
| `cone(r, h, at=CENTER)` | `cone` | true πr²h/3 | **Unsupported** (H3-1; no cylinder stand-in) |
| `box` / `cylinder` | unchanged | — | — |

## Transforms

| Starlark | Notes |
|----------|--------|
| `translate(s, dx, dy, dz)` | mm offset |
| `rotate(s, "x"\|"y"\|"z", deg)` | about world origin |
| `rotate_z(s, deg)` | convenience |
| `mirror(s, "xy"\|"yz"\|"zx")` | through world origin plane (`xz` → `zx`) |

## Patterns (IR expansion, not single nodes)

| Starlark | Expansion |
|----------|-----------|
| `linear_pattern(s, count, dx, dy, dz)` | `count` copies stepped by offset; unions (includes original) |
| `polar_pattern(s, count)` | `count` copies about +Z at equal angles; unions |

Count capped at **64**.

## Example

```sh
cargo run -p cadrion-cli -- build examples/stdlib/pattern_hub.cad.star
# hub + polar fins + sphere boss + mirrored pads
```

## Honesty

- Mock boolean pattern unions **overcount** volume when copies overlap (same as all mock unions).
- OCCT `cone` is **Unsupported** (`CADRION-E-UNSUPPORTED`) — no silent cylinder. See
  [`KERNEL_HONESTY.md`](KERNEL_HONESTY.md).
- Dialect pin (floats + symbol names): [`DIALECT.md`](DIALECT.md) (H5-7 / OQ-2 partial).
- OCCT sphere/mirror/rotate still use STEP round-trips where `Shape.inner` is private (H3 target).
