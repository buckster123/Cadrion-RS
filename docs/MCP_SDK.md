# MCP transport decision (H2-2 / OQ-7)

**Decision date:** 2026-08-06  
**Decision:** **Stay hand-rolled** JSON-RPC MCP (`cadrion-mcp`).  
**Not:** dual stack with official SDK. Revisit only if a dated amendment re-opens OQ-7.

## Why stay hand-rolled (a)

| Factor | Hand-roll (now) | Official Rust MCP SDK |
|--------|-----------------|------------------------|
| Stdio + streamable HTTP | Shipped, tested | Would re-port transports |
| Cadrion gates (`write_source`, project root) | First-class policy | Extra glue / risk of bypass |
| Tool budget (D12 ≤4k tokens) | Short schemas we control | Framework defaults may bloat |
| Dep weight / audit surface | Tiny (serde_json + axum HTTP) | SDK + transitive churn |
| Known-working (P3) | Green CI ubuntu+windows | Migration cost without new agent value |
| Resources (H7) | `cadrion://doc/**` + artifacts | Would need reimplementation anyway |

**(b)** Partial SDK for stdio only — rejected: dual stacks violate H2-2 exit (“no silent dual stacks”).  
**(c)** Full SDK migration now — rejected: payoff unclear; breaks momentum for zero agent-loop gain.

## Protocol surface

**Advertised:** `protocolVersion` = `2024-11-05` (`cadrion_mcp::PROTOCOL_VERSION`).

### Supported methods

See `cadrion_mcp::SUPPORTED_METHODS` (source of truth):

- `initialize` / `notifications/initialized` / `initialized`
- `ping`
- `tools/list` / `tools/call`
- `resources/list` / `resources/read`
- `prompts/list` / `prompts/get` (doctrine: cadrion-loop, write-source-policy, hermetic-load)

### Explicitly unsupported (method-not-found −32601)

See `cadrion_mcp::UNSUPPORTED_BUT_DOCUMENTED`:

- `resources/subscribe` / `unsubscribe` (no live push)
- `resources/templates/list`
- `completion/complete`
- `logging/setLevel`
- `sampling/createMessage`
- `roots/list`

### Tools (20)

`build` · `write_source` · `read_source` · `inspect_refs` · `measure` · `snapshot` ·
`inspect_dims` · `assembly_validate` · `sdf_sample` · `align_check` · `frame` · `diff` ·
`export` · `fab_check` · `engine` · `schema` · `robot` · `parts` · `viewer_open` ·
`gcode_check`  
(`cadrion_mcp::TOOL_NAMES`)

### Transports

| Transport | Entry | Auth | write_source default |
|-----------|-------|------|----------------------|
| stdio | `cadrion mcp` | n/a | **OFF** (`CADRION_MCP_WRITE_SOURCE=1`) |
| streamable HTTP | `cadrion serve mcp` POST/GET `/mcp` | optional bearer | **ON** (`=0` to disable) |

## Compliance test matrix

| Check | How |
|-------|-----|
| Tool list stable | `cargo test -p cadrion-mcp tool_defs_match` |
| initialize caps | `initialize_advertises_protocol_and_caps` |
| unsupported → −32601 | `unsupported_methods_are_method_not_found` |
| core methods OK | `supported_request_methods_do_not_404` |
| resources policy doc | `resources_read_policy_doc` |
| prompts list/get | `prompts_list_and_get_doctrine` |
| write_source gate | H7 tests + policy |
| HTTP tools/list | `http` module tests |
| End-to-end loop | `write_build_inspect_snapshot_loop` |

```sh
cargo test -p cadrion-mcp
```

## When to reopen OQ-7

Re-open only with a CHARTER amendment if **all** hold:

1. Official SDK is stable, small, and supports stdio + streamable HTTP without forking  
2. Cadrion policy hooks (write_source, project root) remain enforceable  
3. Migration is one PR with dual-stack **off** (cutover, not forever parallel)  
4. Measured agent/client pain with hand-roll exceeds migration cost  

Until then: **hand-rolled is the product.**

## Related

- H7 resources: `docs/MCP_RESOURCES.md`
- Skill surface: `skills/cadrion/SKILL.md`
- Charter D17 / OQ-7 amendment: `docs/CHARTER.md`
