# Studio demos (MCP field tests)

Hand-authored / agent-MCP examples — not parity goldens.

| Part | Notes |
|------|--------|
| `stellar_crown.cad.star` | Polar spikes + ring + gem sphere |
| `lunar_bug.cad.star` | Chassis, polar legs, dome, antennae |

```sh
cargo run -p cadrion-cli -- build examples/studio/stellar_crown.cad.star --json
cargo run -p cadrion-cli -- snapshot examples/studio/stellar_crown.cad.star
# or via Hermes MCP: write_source / build / snapshot
```

Snapshot dirs (`*.snap/`) and companion `*.ir.json` are gitignored.
