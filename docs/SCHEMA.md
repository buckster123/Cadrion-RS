# Schema dump (H4-1 / D13)

`cadrion schema` prints the **live** surfaces. It is not a generated JSON Schema of every
CLI/MCP/HTTP response body.

```sh
cadrion schema --json
cadrion schema --json mcp
cadrion schema --json errors
```

| Face | Source |
|------|--------|
| `cli` | clap `Command` tree |
| `mcp` | `cadrion_mcp::tool_defs` + `TOOL_NAMES` |
| `api` | `cadrion_api::openapi_doc` |
| `errors` | `cadrion_kernel::ERROR_CATALOG` |

Adding a diagnostic code: put it in `ERROR_CATALOG` **and** emit that string at the call site.
Adding an MCP tool: `tool_defs` + `TOOL_NAMES` + tests. The schema dump will follow.

OpenAPI remains hand-maintained alpha (`openapi.rs`); H4-3 added measure / dims / sdf paths;
H5-2 added align / frame / diff.
Drift tests
require the dump to match those live functions — they do not invent a second catalog.
