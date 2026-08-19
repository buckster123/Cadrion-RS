# SDF secondary medium (H2-9 experimental)

**STEP / B-rep remains primary.** SDF is a **secondary** sample medium for research,
viz, and ML — never the default modeling or fab path.

## Crate

`cadrion-sdf` — analytic signed-distance for:

| Prim | Params |
|------|--------|
| `box` | full extents dx, dy, dz (mm), centered |
| `cylinder` | radius r, height h along +Z, centered |

## CLI

```sh
cargo run -p cadrion-cli -- sdf sample --prim box --a 40 --b 20 --c 10 --res 32 -o /tmp/sdf_box --json
cargo run -p cadrion-cli -- sdf sample --prim cylinder --a 8 --b 24 --res 24 -o /tmp/sdf_cyl --json
```

Writes:

| File | Content |
|------|---------|
| `*.sdf.f32` | little-endian f32 voxels, X-fastest |
| `*.sdf.json` | grid + prim meta (`cadrion.sdf_volume` v1) |
| `*.nrrd` + `*.raw` | minimal NRRD0004 detached data |

## Honesty fence

- Not mesh-from-OCCT distance (future optional)
- Not a CSG modeling language
- Not used by `build` / STEP export
- Voxel cap 16M; default res=32
- CHARTER: SDF-as-primary is forbidden

## Tests

```sh
cargo test -p cadrion-sdf
```
