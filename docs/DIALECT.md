# Starlark dialect pin (H5-7 / OQ-2 partial)

Pinned **this slice:** float → IR f64, and the stdlib symbol set.
**Still open:** user library modules (`load()` stays refused — D9).

## Floats

- Authors may write `10` or `10.0`. The host accepts Starlark `int` and `float`.
- Feature IR stores IEEE `f64`. Companion `.ir.json` uses `serde_json` default
  (whole numbers print as `10.0`).
- Golden: `crates/cadrion-lang/fixtures/dialect/01_int_and_float.*`
- `CENTER` is `(0.0, 0.0, 0.0)`.

This is **not** a custom float printer. Do not invent `%0.3f` in model code.

## Stdlib symbols (frozen)

Source: `cadrion_lang::STDLIB_SYMBOLS`.

`params` · `box` · `cylinder` · `sphere` · `cone` · `cut` · `union` · `intersect` ·
`union_all` · `solid` · `translate` · `rotate` · `rotate_z` · `mirror` ·
`linear_pattern` · `polar_pattern` · `fillet` · `chamfer` · `print`

Module global (not a function): `CENTER`.

**Not shipped:** `load()`, `use()`, ambient filesystem modules. Names are already
global — `use("cadrion.patterns")` is **not** a sanctioned import.

## What OQ-2 still means

A shared *user* library system (files, versioned packs) needs a later charter
amendment. Do not treat this pin as permission to add `load()`.

## Tests

```sh
cargo test -p cadrion-lang --test dialect
```
