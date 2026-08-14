# H2-1 — WASM IR component

Portable **mock-only** Cadre surface for browsers / wasm hosts.

## Honesty

| | |
|--|--|
| Kernel | **mock** only |
| OCCT / STEP | **unavailable** |
| Parity-10 | **no** (`parity_eligible: false`) |
| Default product path | still native `cadre` CLI / MCP |

## Build

```sh
# native unit tests (default CI)
cargo test -p cadre-wasm

# browser/wasm artifact
rustup target add wasm32-unknown-unknown
cargo build -p cadre-wasm --target wasm32-unknown-unknown --features browser
```

`.cargo/config.toml` sets `getrandom` wasm_js backend for `wasm32-unknown-unknown`.

Optional (local):

```sh
# if wasm-pack installed
wasm-pack build crates/cadre-wasm --target web --features browser -- --no-default-features
```

## API (JSON in / JSON out)

| Fn | Input | Output |
|----|--------|--------|
| `info` | — | capabilities |
| `build` | `{source, set?, name?}` | IR + mock facts |
| `facts_ir` | `{ir}` | mock facts from IR JSON |
| `inspect_ir` | `{ir}` | IR-analytic `inspect_refs` (H3-9; not OCCT) |

Native Rust: `cadre_wasm::build_json` / `facts_ir_json` / `inspect_ir_json` / `info_json`.

With `--features browser`: wasm-bindgen exports `info`, `build`, `facts_ir`, `inspect_ir` as strings.

## Example (conceptual JS)

```js
import init, { info, build } from "./cadre_wasm.js";
await init();
console.log(info());
const out = JSON.parse(build(JSON.stringify({
  source: `
P = params(w=40.0, d=20.0, h=10.0)
def gen_step():
    return solid(box(P.w, P.d, P.h, at=CENTER), label="block")
`,
})));
console.log(out.facts.volume_mm3);
```

See `examples/wasm/index.html` for a static sketch (load your own pack output).

## CI

Ubuntu job `wasm` builds `cadre-wasm` for `wasm32-unknown-unknown` with `--features browser`.
Windows default job stays native-only (no wasm target required).
