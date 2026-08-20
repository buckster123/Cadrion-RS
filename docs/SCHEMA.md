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
H5-2 added align / frame / diff. H5-3 added `/v1/export`. H5-4 added `/v1/fab/check`.
H5-5 added MCP `engine` / `schema` and `/v1/engine` / `/v1/schema` (MCP dumps mcp/errors;
OpenAPI face is HTTP-only; clap remains `cadrion schema`).
H5-8 added `/v1/robot/gen` and `/v1/robot/validate`.
H6-1 added MCP `parts` and `/v1/parts/fetch` + `/v1/parts/lock` (`/v1/parts/search` now wraps the same tool).
H6-2 added MCP `viewer_open` and `/v1/viewer/open` (`once=true` only).
H6-3 added MCP `gcode_check` and `/v1/fab/gcode-check`.
Drift tests
require the dump to match those live functions — they do not invent a second catalog.
