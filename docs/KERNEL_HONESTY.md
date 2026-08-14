# Kernel & parity suite honesty (H3-1)

Binding fences so agents do not treat amber geometry as gold.

## Kernel matrix

| Kernel | CLI | Parity-eligible | STEP | Fillet | Cone | Notes |
|--------|-----|-----------------|------|--------|------|-------|
| **mock** | default | yes (mock suites) | no (`.ir.json`) | Unsupported | **true analytic** volume πr²h/3 | CI default |
| **occt** | `--kernel occt` / `--features occt` | yes (occt suites) | yes | yes | **Unsupported** (H3-1; no cylinder stand-in) | LGPL lane |
| **truck** | `--kernel truck` | **always false** | no | Unsupported | Unsupported | **truck-seed** analytic CSG |
| **truck-brep** | `--kernel truck-brep` + `--features truck-brep` | **always false** | no | Unsupported | Unsupported | H3-6 upstream truck spike |

## Suite fences

| Suite | Kernel | Use |
|-------|--------|-----|
| `parts1-10` | mock | volume/IR ops goldens |
| `parts1-4-occt` / `parts5-10-occt` | occt | tessellation goldens (local) |
| `fillet-occt` | occt | fillet diagnostics |
| truck | — | **not** a parity suite; unit tests only |

```sh
cargo run -p cadre-cli -- bench run --suite parts1-10 --json
# OCCT local only:
cargo run -p cadre-cli --features occt -- bench run --suite parts1-4-occt --json
```

## Cone (H3-1)

| Path | Behavior |
|------|----------|
| mock / IR facts | true cone volume |
| OCCT execute | **CADRE-E-UNSUPPORTED** — refuse silent cylinder |
| truck | unsupported |

Agents needing cone solids on OCCT: wait for real MakeCone binding (Horizon-3+ OCCT depth) or stay on mock for analytic facts.

## Truck naming

- CLI flag / `backend_id`: `truck` (seed) or `truck-brep` (H3-6 spike)
- Implementation tag: **`truck-seed-analytic-csg`** + `truck_brep_spike` bool
- Bid: [`TRUCK_PARITY_BID.md`](TRUCK_PARITY_BID.md) — default **NO-GO** (H3-6 does not flip)

## Related

- [`TRUCK.md`](TRUCK.md) · [`METRICS.md`](METRICS.md) · [`CHARTER.md`](CHARTER.md) D4
