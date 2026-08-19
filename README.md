<div align="center">

<img src="assets/banner.png" alt="Cadrion" width="100%">

<h1>Cadrion</h1>

<p><strong>CAD for agents that have to prove the part.</strong><br>
Write the geometry. Check the facts. Look at it.<br>
Then export or print — only when you say so.</p>

<p>
<img alt="license" src="https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue">
<img alt="rust" src="https://img.shields.io/badge/rust-2021-orange?logo=rust&logoColor=white">
<img alt="ci" src="https://img.shields.io/github/actions/workflow/status/buckster123/Cadrion-RS/ci.yml?label=ci">
<img alt="status" src="https://img.shields.io/badge/status-v0.1%20·%20usable-brightgreen">
</p>

<sub>CLI · MCP · local HTTP · skill pack</sub>

</div>

---

> [!NOTE]
> **The model cannot touch the world.** A `.cad.star` file has no clock, network, or
> filesystem. Printers stay in dry-run until allow-list, file hash, and `confirm=START`.
> Default CI uses a mock kernel. Parts you would actually make need the optional
> Open CASCADE build.

Cadrion is a Rust CAD runtime for coding agents — and for humans who would rather
type than click. One binary builds the solid, measures it, draws it, and (behind
those gates) talks to slicers and printers.

```
.cad.star  →  B-rep  →  facts + snapshot  →  STEP / robot / fab
```

## Why

Hardware design for agents usually means a Python CAD stack, a GUI, and a person
who has to decide that a fillet “looks about right.” The loop dies there.

Cadrion is the other loop. The part is code. The kernel returns geometry. Selectors
and measurements are numbers, not vibes. A snapshot is mandatory review, not
decoration. Failures say why.

## Try it

```sh
git clone https://github.com/buckster123/Cadrion-RS
cd Cadrion-RS
cargo build -p cadrion-cli --release

./target/release/cadrion build    parity/parts/01_calibration_block/part.cad.star --json
./target/release/cadrion inspect refs parity/parts/01_calibration_block/part.cad.star --facts --json
./target/release/cadrion snapshot parity/parts/01_calibration_block/part.cad.star --json
```

Real STEP / STL (local; first OCCT build is long):

```sh
CMAKE_POLICY_VERSION_MINIMUM=3.5 cargo build -p cadrion-cli --release --features occt
./target/release/cadrion --kernel occt build parity/parts/01_calibration_block/part.cad.star --json
```

For an agent: `cadrion mcp` (stdio). For a human: `cadrion view`. Same schema
on the local HTTP face: `cadrion serve api`.

## Faces

| | Who | How |
|--|-----|-----|
| **CLI** | you, scripts | `build` · `inspect` · `snapshot` · `view` · `export` · `fab` · `printer` |
| **MCP** | Hermes, Claude Code, Codex | `cadrion mcp` |
| **HTTP** | anything on loopback | `cadrion serve api` — `/v1` + jobs + OpenAPI |
| **Skills** | the agent that has to *know* the loop | `cadrion skills export --all` |

The skill pack is half the product. Tools without doctrine are not a CAD runtime.

## What ships

- **Author** — hermetic Starlark, parametric, no ambient authority
- **Build** — B-rep via mock (CI) or optional OCCT
- **Verify** — stable selectors, measure, align, frame, diff
- **See** — multi-view PNG, orbit GIF, local viewer
- **Assemble** — parts.lock, assemblies, joint envelope
- **Robots** — URDF / SRDF / SDF generate + validate
- **Make** — DXF, DFM profiles, slicer handoff, gated Bambu / Klipper / OctoPrint

## Install

The binary is `cadrion`, from crate `cadrion-cli`.

```sh
cargo build -p cadrion-cli --release
cp -f target/release/cadrion ~/.local/bin/cadrion
```

`cargo install cadre` is a different, archived project. Ours is this repo, or later
`cargo install cadrion-cli`. Hermes: [`docs/HERMES_MCP.md`](docs/HERMES_MCP.md).

## Docs

| | |
|--|--|
| [`docs/STATUS.md`](docs/STATUS.md) | What is actually built |
| [`docs/design.md`](docs/design.md) | Wire contract |
| [`docs/CHARTER.md`](docs/CHARTER.md) | Binding decisions |
| [`docs/NAME_OQ1.md`](docs/NAME_OQ1.md) | Why the name is Cadrion |
| [`docs/LICENSING.md`](docs/LICENSING.md) | MIT/Apache core · OCCT is LGPL, separate |
| [`docs/gotchas.md`](docs/gotchas.md) | Operator pitfalls |

## License

MIT OR Apache-2.0 — [LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE).
The optional OCCT engine is LGPL-2.1 with the OCCT exception and ships separately.

<sub>Banner · <a href="https://github.com/buckster123/Imaginarium-RS">Imaginarium-RS</a> · job <code>01M0DV4BEAPJ5RCC7153J40A0Z</code></sub>
