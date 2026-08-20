# OCCT binding strategy (S1 spike)

> Status: **GO** (D19). S3 landed `crates/cadrion-occt` + IR execute + calibration STEP e2e.
> Default CI still excludes OCCT (CMake/OCCT compile cost). Local: `CMAKE_POLICY_VERSION_MINIMUM=3.5 cargo test -p cadrion-occt`.


## Goal

Ship reference-grade B-rep (fillets, booleans, STEP AP242) behind [`GeomKernel`](../crates/cadrion-kernel)
without:

1. smearing LGPL into MIT/Apache core static links (D4, D6)
2. forcing every `cargo build` to compile OCCT from source
3. coupling the authoring surface (Starlark) to any third-party high-level CAD API

## Options surveyed (2026-08-05)

| Option | Crate(s) | License (tooling) | Link model | Fit |
|--------|----------|-------------------|------------|-----|
| **A. bschwind opencascade-rs** | `opencascade` 0.2, `opencascade-sys` 0.2, `occt-sys` 0.6 | LGPL-2.1 | Static via `occt-sys` **or** dynamic system OCCT (`default-features = false`, `DEP_OCCT_ROOT`) | **Primary candidate** — cxx bridge exists, STEP/fillet/boolean surface, active enough for a spike |
| **B. occt-wasm** | `occt-wasm` 3.3 | MIT/Apache tooling; **WASM binary LGPL** | In-process wasmtime or out-of-process engine | Strong **portable engine** story for `cadrion engine install` without host C++ toolchain; extra latency/host layer |
| **C. cadrum** | `cadrum` 0.8 | Claims MIT; static OCCT inside | Static | **Do not adopt as modeling API** — would fork authoring away from Starlark/IR. Techniques for static/headless builds may inform packaging later |
| **D. Hand-rolled cxx → system OCCT** | none | our MIT/Apache + system LGPL | Dynamic `.so`/`.dylib` | Maximum control; more glue code; still need prebuilt engine matrix |
| **E. Pure Rust** | `truck-modeling` 0.6, `cadcore` 0.1 | Apache/MIT | None | **Experimental second backend only** — not parity-eligible (D4) |

Sources: crates.io metadata + upstream READMEs (`bschwind/opencascade-rs`, `andymai/occt-wasm`).

## Decision — GO path

### Layering

```
cadrion-lang / cadrion-model
        │  feature IR
        ▼
 cadrion-kernel::GeomKernel     ← pure Rust, this repo, MIT OR Apache-2.0
        │
        ├── MockKernel        ← tests / offline dry-run (ships now)
        ├── cadrion-occt        ← default parity backend (S3+)
        └── cadrion-truck       ← optional, non-parity
```

### `cadrion-occt` implementation plan

1. **Implement `GeomKernel` only** — no Starlark, no CLI inside the crate.
2. **Prefer dynamic / separate engine** over baking `occt-sys` static into the default
   `cadrion` binary:
   - Dev/CI option: `opencascade` with `default-features = false` against distro or
     `DEP_OCCT_ROOT` prebuilts.
   - Product option: `cadrion engine install` drops versioned shared libraries + a small
     loader (dlopen or a dedicated `cadrion-engine` helper process). Matches D4.
3. **Spike order for S3:** box → cylinder → boolean cut → fillet → `write_step` → facts
   (volume/bbox) on calibration-block topology. If bschwind high-level API blocks a needed
   op, drop to `opencascade-sys` / cxx for that op only.
4. **Error translation:** OCCT failures → `KernelError::Diagnostic` with `CADRION-E-*` codes
   and feature-level hints (never raw C++ as the only message).
5. **Feature flag:** workspace builds default **without** OCCT so `cargo test` on a fresh
   clone stays green. `--features occt` or separate package enables the backend.

### Explicit NO / not-now

| Choice | Why |
|--------|-----|
| Static-link OCCT into published MIT core by default | LGPL distribution complexity; huge binaries; slow CI |
| Depend on `cadrum` / `opencascade` high-level as the *language* | Authoring is Starlark→IR (D2); kernel is an executor |
| Parity claims on truck/cadcore/mock | D4 |
| Compile OCCT from source on every user machine | NFR install-to-first-STEP ≤ 5 min — needs prebuilts |

### Fallback if A fails hard

If opencascade-rs proves unmaintained or API-blocked during S3:

1. Try **D** (thin cxx to same prebuilt OCCT).
2. Evaluate **B** (`occt-wasm`) as the *engine process* implementation of
   `cadrion engine install` (host talks JSON/IPC; WASM stays LGPL-isolated).
3. Only then reconsider deeper vendoring.

Record any switch as a dated charter amendment — do not silent-pivot.

## S1 exit evidence

- [x] `cadrion-kernel` crate with `GeomKernel` v0 + `MockKernel` tests
- [x] This document
- [x] Charter D19 + amendment dated 2026-08-05

## S3 exit evidence

- [x] `cadrion-occt` implements `GeomKernel` (box/cylinder/boolean/fillet/chamfer/STEP)
- [x] `cadrion_lang::execute_ir` lowers IR onto any kernel
- [x] Calibration block `.cad.star` → fillet → STEP (1524 ents) + volume/bbox facts
- [x] Default CI excludes OCCT; local recipe documented (CMake 4 policy env)
- [x] `cadrion engine info` reports compile-time kernels (H4-2)
- [ ] Prebuilt `cadrion engine install` artifacts (still packaging). H4-2 `install` is **fail-closed** (`CADRION-E-ENGINE-MISSING`) unless the backend is already compiled in — it does not fetch.

### Local OCCT gotchas (2026-08-05)

- **CMake 4.x:** vendored OCCT CMakeLists use `cmake_minimum_required` &lt; 3.5. Set
  `CMAKE_POLICY_VERSION_MINIMUM=3.5` when building `cadrion-occt` on CMake ≥ 4.
- **Volume facts:** S3 uses tessellation-based volume (public API has no GProp helpers
  without `opencascade-sys` internals). Tolerances in e2e are ~5–8% relative.
- **Shape clone:** STEP round-trip clone (no public `Clone` on `Shape`).


## Open follow-ups (not blocking S1)

- Pin exact OCCT version for prebuilds (7.7 vs 7.8 vs vendor fork).
- Windows/macOS engine artifact layout.
- Whether engine is in-process `cdylib` vs subprocess (security + crash isolation).
- Legal review memo before 1.0 (NFR-6) — distribution story only, not blocking M0 spike.
