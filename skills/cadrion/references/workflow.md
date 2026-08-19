# Cadrion workflow (progressive reference)

## Authoring
- One part per `.cad.star`; entry `gen_step()` returns a labeled `solid(...)`.
- Parameters via `P = params(...)`; override at build with `--set k=v` / MCP `set`.
- Prefer through-features (holes taller than plate) for robust cuts.

## Verify
1. `inspect_refs --facts` — solid/face/edge inventory + volume
2. `measure` thickness between opposite face normals
3. `snapshot` iso+front minimum; read PNG content / open viewer

## Repair
- Fillet fail → reduce radius (mock may UNSUPPORTED fillet entirely)
- Wrong volume → check hole count / params
- Bad selector after edit → re-run `inspect_refs` (tokens remapped)

## Export
- IR always: `*.ir.json`
- STEP/STL: `cadrion --kernel occt` (binary with `--features occt`)
- Snap packet: `*.snap/`

## Robots
```sh
cadrion robot gen arm.robot.json -o out/ --json
cadrion robot validate out/arm.urdf --json
```
Geometry JSON uses external tags: `{"box":{"size":[x,y,z]}}`. Units SI in URDF.

## Fab
```sh
cadrion fab check --part-json plate.flat.json --json
cadrion fab gcode-check print.gcode --json
cadrion printer dry-run print.gcode --json
# start: allowlist + sha256 + --confirm START (live start may still be refused)
```

## Surfaces
- CLI: `cadrion … --json`
- MCP: `cadrion mcp` (stdout = protocol only)
- HTTP: `cadrion serve api --token …` → `/v1/*`
- Skills: `cadrion skills export --all -o dist/skills`
