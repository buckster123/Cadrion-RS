# H7 — MCP resources + write_source policy

## OQ-5 decision (2026-08-05)

| Transport | `write_source` | Override |
|-----------|----------------|----------|
| stdio | **OFF** | `CADRION_MCP_WRITE_SOURCE=1` |
| HTTP | **ON** | `CADRION_MCP_WRITE_SOURCE=0` |

`read_source` stays available on both.

## Resources

```
resources/list
resources/read  { "uri": "cadrion://doc/status" }
```

| URI prefix | Role |
|------------|------|
| `cadrion://doc/*` | doctrine markdown (STATUS, STDLIB, VIEWER, SLICER_DFM, FILLET, policy, schema) |
| `cadrion://artifact/index` | scan under `CADRION_PROJECT` / cwd |
| `cadrion://artifact/file/<rel>` | read artifact (path-escape refused) |

## Tests

```sh
cargo test -p cadrion-mcp
```

## Honesty

- Not full MCP SDK resource subscriptions
- Artifact scan is depth-capped, not a full VCS index
- Policy is process-global (`OnceLock`) — set at stdio/HTTP entry
