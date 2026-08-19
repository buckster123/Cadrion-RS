# Gotchas — the invariant ledger

> **RULE: before modifying ANY subsystem, grep this file for it and read the matching
> entries.** These are load-bearing invariants — most were written after something broke
> on a live node, and many end with an explicit "don't do X" that a future change would
> otherwise walk straight into.
>
> **A newly discovered gotcha goes HERE**, not in CLAUDE.md. Docs travel with code —
> update this file in the same PR as the change that discovered or altered an invariant.
>
> Format: one bullet, **bold lead naming the invariant**, then the story, ending with the
> explicit don't. Cross-project version drift lives in
> `~/Projects/Launchpad-RS/docs/sharp-edges.md` instead.

- **Clean-room only.** Cadrion is a functional-conceptual peer of `earthtojake/text-to-cad`, not
  a source port. Don't vendor, translate, or "check how they did it" in their Python/JS tree.
  Behavior comes from our PRD + their *public* docs/README/skill text. Don't add a submodule
  or copy of the reference repo.

- **OCCT behind `GeomKernel`, engine separate.** Default kernel is LGPL-adjacent. Don't
  statically link OCCT into MIT/Apache core binaries or bypass the trait "just for a spike"
  in a way that smears license boundary into published crates. Use `cadrion engine install`
  distribution story; legal review before 1.0.

- **Truck is not parity.** Experimental backend seeds the trait. Don't mark Parity-10 green
  on truck or advertise truck as default without a charter amendment.

- **No directory-wide builds.** Agents will try `cadrion build .`. Refuse. Explicit targets only.

- **Don't git-diff STEP/STL/3MF.** Binary noise. Use `inspect diff` and content hashes. Skill
  doctrine must keep saying this.

- **MCP stdout is sacred.** JSON-RPC only on stdout; all `tracing` to stderr. Don't `println!`
  in library code that MCP links.

- **Printer start is triple-gated.** Allow-list + gcode hash match + explicit confirm string/flag.
  Don't add a "dev convenience" default that skips confirm on any surface.

- **Schema is one type layer.** CLI JSON, MCP tools, OpenAPI — generate together. Don't
  hand-edit one surface's schema without the others; CI must fail on drift once S8+ lands.

- **Fake success is a bug.** Missing engine, slicer, lock entry, or GPU snapshot path →
  structured error. Don't return empty artifacts with `ok: true`.

- **`MockKernel` is not OCCT.** Volumes after boolean are analytic approximations; fillet /
  chamfer / STEP / tessellate return `CADRION-E-UNSUPPORTED`. Don't mark Parity-10 or field
  evidence against mock. Don't silently “implement” fillet in mock as a no-op success.

- **OCCT stays out of default `cargo test`.** `cadrion-kernel` must stay pure Rust. Don't add
  `opencascade`/`occt-sys` to the default workspace build graph — gate behind `cadrion-occt`
  features / optional member so fresh clones stay green (see `docs/occt-binding.md`).

- **`load()` is hermetic-forbidden.** `cadrion-lang` refuses AST loads before eval
  (`CADRION-E-HERMETIC-LOAD`). Don't add a silent file loader "for convenience" without a
  charter amendment and sandbox story.

- **Starlark `box` is `r#box` in Rust.** The stdlib exports the name `box` to Starlark; don't
  rename it to `make_box` in the language surface (PRD / agent fluency).

- **OCCT build needs CMake policy on CMake ≥ 4.** `export CMAKE_POLICY_VERSION_MINIMUM=3.5`
  before `cargo test -p cadrion-occt`. Don't "fix" by forking occt-sys in-tree without a
  charter note.

- **`OcctKernel` is Send via unsafe.** Unique ownership only — never share one kernel across
  concurrent jobs. Don't remove the safety comment when touching Send.

- **Selector indices are 1-based in tokens.** `#o1.1.f1` is the first face after stable sort,
  never zero. Don't emit 0-based tokens to agents.

- **Cache get must re-hash artifacts.** A hit that skips `artifact_sha256` check will serve
  corrupted STEP as success — never.

- **`.cad.star` stem is not `with_extension("")`.** That yields `foo.cad` from `foo.cad.star`.
  Always strip the `.cad.star` / `.star` suffix explicitly (see `strip_model_suffix` in CLI).

- **Mock build is IR-first.** STEP/STL need `--kernel occt` (+ binary built `--features occt`).
  Don't treat mock STEP warnings as success in agent loops that need fab files.

- **Snapshot preview ≠ B-rep.** Cut/intersect preview meshes keep A only; fillet/chamfer are
  not shown. Agents must still trust numeric inspect, not pixels alone.

- **`cadrion view` blocks.** Use `--once` in CI (prepare only). Default serves until Ctrl-C.

- **MCP stdout is protocol-only.** Never print human banners on stdout in `cadrion mcp`.
- **`part.cad.star` is not a repo file.** Fixtures live under `parity/parts/*/part.cad.star`.
- **parts.lock is fail-closed.** Missing lock entry or checksum mismatch must error — never
  silently fetch/substitute.
- **API default is loopback.** Don't bind `0.0.0.0` without a token.
- **Printer start is gated by you.** Needs allow-list + gcode sha256 match + `confirm=START`.
  Even then, **no network** unless you also pass `--live` (and access code + serial).
  Live path: FTPS via `curl` + MQTT via `mosquitto_pub` (self-signed TLS).
- **DFM profiles are versioned data.** Reports are profile-truth, not live vendor quotes.
