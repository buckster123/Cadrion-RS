# DFM profile governance (H3-8 / OQ-6 seed)

**Not a live vendor quote API.** Bundled profiles are Cadre-authored data.
Community shops may overlay rules — **never silently** against a bumped base.

## Schemas

| Kind | `schema` | `schema_version` |
|------|----------|------------------|
| Full profile | `cadre.dfm_profile` | **1** |
| Override | `cadre.dfm_override` | **1** |

Unknown schema / version → **fail closed**.

Profile `version` is semver-like `N.N.N`. Rules must be finite ≥ 0. At least one material
with positive thicknesses.

Legacy JSON without `schema` fields still loads (defaults applied) then validates.

## Bundled ids

| Id | Version |
|----|---------|
| `sendcutsend.laser` | 1.0.0 |
| `pcb.outline` | 1.0.0 |
| `waterjet.generic` | 1.0.0 |

Bump the **string version** when changing a bundled rule. Old overrides that pin
`base_version` will then refuse (`CADRE-E-DFM-DRIFT`) until explicitly updated.

## Community override

```json
{
  "schema": "cadre.dfm_override",
  "schema_version": 1,
  "base": "waterjet.generic",
  "base_version": "1.0.0",
  "note": "shop tighter web",
  "rules": { "min_web_mm": 2.5 }
}
```

```sh
cargo run -p cadre-cli -- fab check --profile waterjet \
  --override-file examples/fab/community.waterjet.override.json \
  --part-json examples/fab/waterjet.flat.json --json
```

| Gate | |
|------|--|
| `base` must equal resolved profile id | |
| `base_version` must **equal** bundled `version` | else drift error |
| Overlay only listed rule fields | rest stay from base |

## How profiles update (no silent drift)

1. Change bundled rules → bump `1.0.0` → `1.1.0` (or major if semantics break).
2. Overrides still pinning `1.0.0` fail until their `base_version` is edited on purpose.
3. Full custom profiles via `--profile-file` must pass `validate_profile`.

## OQ-6 status

**Seed only.** Not a community registry, not signed packages, not auto-pull.
Full vendor-pack governance stays open.
