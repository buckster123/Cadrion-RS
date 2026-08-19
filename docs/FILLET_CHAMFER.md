# H4 — Fillet / chamfer parity + diagnostics

## Doctrine (agents)

| Kernel | Fillet / chamfer |
|--------|------------------|
| **mock** (default CI) | Always `CADRION-E-UNSUPPORTED` — **do not** put fillet/chamfer in mock parity stars |
| **OCCT** (`--features occt`) | Real B-rep; fails with structured codes below |

Mock stays honest. For filleted geometry use OCCT lane / `expect.occt.json` only.

## Diagnostics

| Code | When |
|------|------|
| `CADRION-E-FILLET-FAILED` | OCCT `MakeFillet` not done (radius too large / bad edges) |
| `CADRION-E-CHAMFER-FAILED` | OCCT `MakeChamfer` not done |
| `CADRION-E-UNKNOWN-EDGE` | Edge index out of range |
| `CADRION-E-INVALID-ARG` | radius/distance ≤ 0 |

Diagnostics carry `refs: ["#e0", …]` (stable explorer order) + shape id + hint to reduce radius/distance.

## Parity suite `fillet-occt`

| Part | Ops |
|------|-----|
| `11_filleted_plate` | box + hole cut + all-edge fillet |
| `12_chamfered_brick` | box + all-edge chamfer |
| `13_filleted_l` | **H3-7** two-plate L union + fillet |

```sh
CMAKE_POLICY_VERSION_MINIMUM=3.5 cargo test -p cadrion-occt --test fillet_smoke
CMAKE_POLICY_VERSION_MINIMUM=3.5 cargo test -p cadrion-bench --features occt fillet_occt
CMAKE_POLICY_VERSION_MINIMUM=3.5 cargo run -p cadrion-cli --features occt -- \
  bench run --suite fillet-occt
```

## Implementation notes

- Patched `opencascade` `Shape::fillet*` / `chamfer*` return `Result` and call `Build` + `IsDone`.
- `cadrion-occt` maps failures to the codes above.
- Default CI remains mock / OCCT-free.
