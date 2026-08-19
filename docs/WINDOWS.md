# Windows notes (S12)

Cadrion default workspace is intended to build on Windows without OCCT.

## CI

GitHub Actions job `windows` runs on `windows-latest`:
- `cargo fmt` is Linux-only in CI (line endings); Windows job runs **clippy + test + build**.
- OCCT feature is **not** exercised on Windows CI.

## Local

```powershell
rustup default stable
cargo test
cargo run -p cadrion-cli -- version --json
cargo run -p cadrion-cli -- fab check --part-json examples/fab/plate.flat.json --json
```

## Paths

- Prefer forward slashes in Starlark/JSON examples; Windows APIs accept them in Rust `Path`.
- Slicer discovery looks at `PATH` for `prusa-slicer.exe` / `orca-slicer.exe` names when present.
- Loopback API bind `127.0.0.1:7410` works the same.

## Known gaps

- OCCT / opencascade-rs Windows build is **unsupported in-tree** until someone lands a
  documented MSVC recipe (out of S12 scope).
- Long path issues: enable Windows long paths if monorepo checkouts nest deeply.

## Line endings

`.gitattributes` recommends LF for Rust/Starlark sources so `cargo fmt --check` stays stable.
