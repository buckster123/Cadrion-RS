---
name: cadrion
description: >
  Cadrion CAD runtime for agents — hermetic Starlark → IR → inspect → snapshot →
  robots → fab. Use for parametric CAD, DFM, URDF, and gated printer dry-runs.
---

# Cadrion skill (v1)

## Defaults
- Units: **millimeters**, XY base, **+Z up**
- Source files: `*.cad.star` with `def gen_step():`
- Always **numeric inspect before visual**; then **snapshot** after visible changes
- Never directory-wide builds; pass explicit file paths
- Do not git-diff binary STEP/PNG; use `inspect` / `snapshot`

## Loop
1. Clarify one question max if critical dims missing
2. Write `part.cad.star`
3. `build` → facts (volume, bbox)
4. `inspect_refs` / `measure`
5. `snapshot` and **review images** if geometry changed
6. Optional: robot gen, DFM check, gcode-check, printer dry-run

## Starlark flavor
```python
P = params(width=100.0, depth=60.0, height=20.0, hole_d=8.0)

def gen_step():
    blk = box(P.width, P.depth, P.height, at=CENTER)
    hole = cylinder(P.hole_d / 2.0, P.height + 2.0, at=(0.0, 0.0, -1.0))
    return solid(cut(blk, hole), label="part")
```

## CLI (high signal)
```sh
cadrion build path.cad.star --json
cadrion inspect refs path.cad.star --facts --json
cadrion snapshot path.cad.star --json
cadrion robot gen arm.robot.json -o out/ --json
cadrion parts search m6 --json
cadrion fab check --part-json plate.flat.json --json
cadrion fab gcode-check print.gcode --json
cadrion printer dry-run print.gcode --json
cadrion mcp
```

## MCP tools
`build`, `write_source`, `read_source`, `inspect_refs`, `measure`, `snapshot`,
`inspect_dims`, `assembly_validate`, `sdf_sample`, `align_check`, `frame`, `diff`,
`export`, `fab_check`, `engine`, `schema`, `robot`, `parts`, `viewer_open`

### write_source policy (H7)
| Transport | Default | Override |
|-----------|---------|----------|
| stdio (`cadrion mcp`) | **OFF** | `CADRION_MCP_WRITE_SOURCE=1` |
| HTTP (`cadrion serve mcp`) | **ON** | `CADRION_MCP_WRITE_SOURCE=0` |

Local agents already have FS tools — prefer those on stdio. HTTP agents need MCP write.

### MCP implementation (H2-2 / OQ-7)
**Hand-rolled** JSON-RPC only — no official SDK dual stack. See `docs/MCP_SDK.md`.

### MCP resources
| URI | Content |
|-----|---------|
| `cadrion://doc/status` | live status |
| `cadrion://doc/stdlib` | stdlib depth |
| `cadrion://doc/dialect` | float + stdlib name pin |
| `cadrion://doc/viewer` | viewer gcode/robot |
| `cadrion://doc/slicer-dfm` | slicer gates + DFM |
| `cadrion://doc/fillet` | fillet doctrine |
| `cadrion://doc/write-source-policy` | this policy |
| `cadrion://doc/schema` | schema dump honesty |
| `cadrion://artifact/index` | local IR/snap/gcode index |
| `cadrion://artifact/file/<rel>` | read one artifact |

`resources/list` + `resources/read` JSON-RPC methods.

### MCP prompts (H5-6)
`prompts/list` / `prompts/get`: `cadrion-loop`, `write-source-policy`, `hermetic-load`.
They teach; they are not a fourth face and not a replacement for this skill.

## Safety
- Printer **start** needs allow-list + sha256 + `confirm=START` (+ `--live`)
- Slicer **execute** needs `--confirm SLICE` (+ optional allowlist)
- Default kernel is **mock**; STEP needs OCCT feature
- DFM profiles are versioned data, not live vendor quotes

## Sanctioned snapshot skip
Only if **no visible geometry change** or **no valid artifact** — report the reason.
