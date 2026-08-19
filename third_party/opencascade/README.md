# Patched `opencascade` 0.2.0 (Cadrion-RS H3)

Local path patch of [opencascade-rs](https://github.com/bschwind/opencascade-rs) 0.2.0.

## Why

Upstream `Shape.inner` is `pub(crate)`, so consumers cannot call `BRepBuilderAPI_Transform`
without STEP write/read thrash. Cadrion-RS needs in-memory translate/rotate/mirror/sphere.

## Cadrion additions (`src/primitives/shape.rs`)

| Method | Purpose |
|--------|---------|
| `deep_copy()` | identity transform, `copy=true` |
| `apply_transform(&gp_Trsf)` | direct BRep transform |
| `transformed_with(setup)` | build `gp_Trsf` + apply |
| `Shape::sphere(radius)` | `BRepPrimAPI_MakeSphere` → Shape |
| `fillet` / `chamfer` (+ edges variants) | `Result` + `Build`/`IsDone` (H4) |

Also `Error::TransformFailed` / `PrimitiveFailed` / `FilletFailed` / `ChamferFailed`.

## Wire-up

Workspace root:

```toml
[patch.crates-io]
opencascade = { path = "third_party/opencascade" }
```

LGPL-2.1 same as upstream. Do not treat this as a general fork — keep the delta minimal.
