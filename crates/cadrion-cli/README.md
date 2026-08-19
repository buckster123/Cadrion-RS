# cadrion-cli

Binary name: **`cadrion`**.

```sh
# default (mock kernel — CI-safe)
cargo run -p cadrion-cli -- build path/to/part.cad.star --json
cargo run -p cadrion-cli -- inspect refs path/to/part.cad.star --facts --json
cargo run -p cadrion-cli -- inspect measure part.cad.star '#o1.1.f1' '#o1.1.f2' --kind thickness --json

# OCCT (local; CMAKE_POLICY_VERSION_MINIMUM=3.5 on CMake ≥ 4)
cargo run -p cadrion-cli --features occt -- --kernel occt build part.cad.star --json
cargo run -p cadrion-cli --features occt -- --kernel occt export part.cad.star --format stl -o part.stl --json
```

## Commands

| Cmd | Role |
|-----|------|
| `build <file.cad.star>` | eval → IR → kernel → IR file (+ STEP if kernel can) + cache |
| `inspect refs <target>` | stable `#o…` inventory (`--facts`) |
| `inspect measure <target> <a> [b] --kind …` | distance / angle / diameter / thickness |
| `export <target> --format step\|stl\|glb` | secondary formats (tessellation needs OCCT) |
| `version` | versions + feature flags |

Global: `--json`, `-q`, `--project`, `--kernel mock|occt`, `-v`.

## Honesty notes

- Directory targets refused (`CADRION-E-EXPLICIT-TARGET`).
- Mock cannot write STEP/STL — diagnostics say so; IR still written.
- `export --format glb` currently emits JSON **glTF** (embedded buffers), not binary GLB.
