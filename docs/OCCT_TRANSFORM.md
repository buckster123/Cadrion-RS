# H3 — OCCT transform quality

## Problem

Upstream `opencascade` 0.2 keeps `Shape.inner` (`TopoDS_Shape`) crate-private.
Cadrion previously cloned/transformed via **STEP write → read → BRep → STEP write → read**,
which was slow, serial-hostile, and littered `/tmp`.

## Fix

Workspace patch:

```toml
[patch.crates-io]
opencascade = { path = "third_party/opencascade" }
```

Minimal LGPL fork of 0.2.0 adding:

| API | Role |
|-----|------|
| `Shape::deep_copy()` | identity BRep transform, `copy=true` |
| `Shape::apply_transform` / `transformed_with` | in-memory `gp_Trsf` |
| `Shape::sphere(r)` | `MakeSphere` without STEP |

`cadrion-occt` now:

- **translate / rotate** — single BRep transform
- **mirror** — scale −1 then 180° about complementary axis (sys lacks `SetMirror(gp_Ax2)`)
- **sphere** — `Shape::sphere` + optional translate
- **boolean/fillet clone** — `deep_copy` instead of STEP clone

## Evidence

```sh
CMAKE_POLICY_VERSION_MINIMUM=3.5 cargo test -p cadrion-occt --test transform_smoke
# includes assert: no new cadrion-occt-*.step in /tmp during transforms

CMAKE_POLICY_VERSION_MINIMUM=3.5 cargo test -p cadrion-bench --features occt \
  parts_5_10_occt -- --nocapture
```

## Honesty

- Cone still cylinder stand-in (no MakeCone in sys).
- Mirror is plane-correct via scale+rotate composition, not OCCT plane mirror API.
- Explicit `write_step` for export still uses STEP (correct).
- Patch must stay minimal — prefer upstream PR if/when they expose transforms.
