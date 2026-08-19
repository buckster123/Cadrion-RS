# Simple 2-DOF arm (S10)

JSON robot spec → URDF + SRDF + SDF with inertials and urdf-rs parse check.

```sh
cargo run -p cadrion-cli -- robot gen examples/robots/simple_arm.robot.json -o /tmp/arm --json
cargo run -p cadrion-cli -- robot validate /tmp/arm/simple_arm.urdf --json
```

Inertials are analytic box tensors (kg, m). Units in URDF are SI.
