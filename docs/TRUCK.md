# H10 + H3-6 — truck experimental lane (non-parity)

## Two implementations

| Id | Feature | What |
|----|---------|------|
| `--kernel truck` | always | Analytic CSG **seed** (volume math + bbox mesh) |
| `--kernel truck-brep` | `cadrion-cli` feature `truck-brep` | **H3-6 spike:** upstream `truck-modeling` 0.6 + `truck-shapeops` 0.4 |

## Honesty (binding)

| Rule | |
|------|--|
| Default kernel | **never** truck / truck-brep |
| Parity-10 | **`parity_eligible() == false`** always |
| STEP | unsupported (both) |
| Fillet/chamfer | unsupported (both) |
| Seed | not upstream truck |
| Spike | real B-rep box/cyl + boolean + triangulation — **still NO-GO** for default |

## CLI

```sh
cargo test -p cadrion-truck
cargo test -p cadrion-truck --features brep
cargo run -p cadrion-cli --features truck-brep -- \
  build examples/pmi/block.cad.star --kernel truck-brep --json
cargo run -p cadrion-cli -- version --json
# truck_parity_eligible: false
# truck_brep_spike: true|false
```

Without `--features truck-brep`, `--kernel truck-brep` returns `CADRION-E-KERNEL-UNAVAILABLE`.

## Spike notes (H3-6)

- Box centered at placement (Cadrion convention) via `tsweep`.
- Cylinder: `rsweep` + `try_attach_plane` + `tsweep`.
- Cut = invert tool + `truck_shapeops::and` (tol 0.05 mm). Fail-closed if `None`.
- Tessellate = `MeshableShape::triangulation` (not bbox).
- Volume facts from signed triangle volume (amber vs OCCT).
- Inspect topology still IR fallback (`truck-brep-ir-fallback`).
- License: truck crates **Apache-2.0** (G7 pin review started — see bid).

## Promotion bar

Unchanged: **H2-10 NO-GO** until G1–G7 + CHARTER D4. H3-6 is **G1 partial**
(boolean + mesh yes; STEP write **not** yet).
