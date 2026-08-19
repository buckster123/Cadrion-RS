# OCCT depth notes (post-v1)

## Shipped
- `OcctKernel::topology_snapshot(shape)` — mesh-clustered faces (normals + area + COM) and
  mesh edges; safe on boolean-cut solids
- `cadrion inspect … --kernel occt` uses live topology when binary built `--features occt`
- Boolean ops via **AdHocShape** (avoids `Shape::subtract` → `SectionEdges` crash)
- Tests: box topology + thickness; union; cut; calibration STEP e2e; parity-01 volume

## Boolean cut fix
`Shape::subtract` / `union` in opencascade-rs 0.2 call `SectionEdges()` after the op, which
throws C++ `StdFail_NotDone` on some OCCT builds. **Fix:** route all booleans through
`AdHocShape::{subtract,union,intersect}`, which only take `.Shape()`.

## Topology safety
`Face::center_of_mass` / `normal_at_center` can also throw uncatchable C++ exceptions on cut
faces (cxx does not turn them into Rust panics). **Fix:** build topology from `shape.mesh()`
triangle normal clustering — no B-rep face property calls.

## Local verify
```sh
CMAKE_POLICY_VERSION_MINIMUM=3.5 cargo test -p cadrion-occt
CMAKE_POLICY_VERSION_MINIMUM=3.5 cargo run -p cadrion-cli --features occt -- \
  --kernel occt inspect refs parity/parts/01_calibration_block/part.cad.star --facts --json
```

## Next
1. OCCT expect.json lane in cadrion-bench (`parts1-4-occt`) — **done**
2. Tighter tessellation tolerance for volume goldens
3. Face→DXF from live face refs

## Bench lane
```sh
# mock (default CI)
cargo test -p cadrion-bench
cargo run -p cadrion-cli -- bench run --suite parts1-4 --json

# OCCT (local / feature)
CMAKE_POLICY_VERSION_MINIMUM=3.5 cargo test -p cadrion-bench --features occt
CMAKE_POLICY_VERSION_MINIMUM=3.5 cargo run -p cadrion-cli --features occt -- \
  bench run --suite parts1-4-occt --json
```
Goldens live beside each part as `expect.occt.json` (looser volume tol for tessellation).
