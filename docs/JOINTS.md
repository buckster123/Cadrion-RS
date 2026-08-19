# Assembly joints & kinematics (H2-5 + H3-4)

## JointSpec (assembly JSON)

| Field | Meaning |
|-------|---------|
| `kind` | `fixed` \| `revolute` \| `prismatic` |
| `axis` | parent-frame axis (default +Z) |
| `origin_mm` | joint origin in parent frame |
| `lower` / `upper` | **required** for revolute (rad) and prismatic (mm) |
| `effort` / `velocity` | optional; must be ≥ 0 if set |

## Validate (fail-closed)

```sh
cargo run -p cadrion-cli -- assembly validate examples/assembly/lid_hinge.assy.json --json
cargo run -p cadrion-cli -- assembly validate examples/assembly/bad_limits.assy.json --json
```

## H3-4 — kinematics emit (OQ-4 partial)

```sh
# Sidecar: cadrion.assembly_kinematics v1 (m / rad)
cargo run -p cadrion-cli -- assembly emit-kinematics examples/assembly/lid_hinge.assy.json --json
# → examples/assembly/lid_hinge.kinematics.json

# Minimal robot JSON (placeholder 50mm cubes) → URDF path
cargo run -p cadrion-cli -- assembly emit-robot examples/assembly/lid_hinge.assy.json -o /tmp/lid.robot.json --json
cargo run -p cadrion-cli -- robot gen /tmp/lid.robot.json -o /tmp/lid_urdf --json
```

| Artifact | Contents |
|----------|----------|
| `*.kinematics.json` | links, joints, placements_mm, unit notes |
| `*.robot.json` | RobotSpec-shaped JSON + `_cadrion` honesty tag |
| URDF/SRDF/SDF | via existing `robot gen` |

**Not AP242.** No kinematic STEP entities. Placeholder visuals/inertials — not CAD meshes.

## Examples

| File | Expect |
|------|--------|
| `plate_bolt.assy.json` | fixed joint |
| `lid_hinge.assy.json` | revolute with limits → kinematics z=0.02 m |
| `bad_limits.assy.json` | inverted limits — **must fail** validate |

## OQ-4 status

| Bite | Status |
|------|--------|
| Limit envelope (H2-5) | done |
| Assembly → kinematics sidecar + robot IR (H3-4) | done |
| AP242 / true STEP joint entities | **still open** |

## CHARTER
OQ-4 remains open for full STEP kinematics depth; H3-4 closes the **CAD assembly ↔ robot IR bridge**.
