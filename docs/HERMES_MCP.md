# Hermes MCP wiring (Cadrion)

## Install binary

```sh
cargo build -p cadrion-cli --release
cp -f target/release/cadrion ~/.local/bin/cadrion
```

## Config (`~/.hermes/config.yaml`)

```yaml
mcp_servers:
  cadrion:
    command: /home/andre/.local/bin/cadrion
    args:
      - mcp
    env:
      CADRION_PROJECT: /home/andre/Projects/Cadre-RS   # garden folder until moved
      CADRION_MCP_WRITE_SOURCE: "1"   # allow write_source on stdio for agent loops
      RUST_LOG: warn
    timeout: 120
    connect_timeout: 30
    enabled: true
```

## Framing note

Hermes uses the official Python `mcp` SDK, which speaks **NDJSON** (one JSON-RPC object per line).
Cadrion auto-detects NDJSON vs Content-Length. Override with `CADRION_MCP_FRAMING=ndjson|content-length`.

## Verify

```sh
hermes mcp test cadrion
# → Connected, 16 tools: … export fab_check engine schema
```

## Live session

Config changes need **`/reload-mcp`** (or a new Hermes session). Disk-green `hermes mcp test` is not the same as the already-spawned child in a long-lived CLI session.

## Tools (prefix)

After reload, tools appear as `mcp_cadrion_*` / deferred catalog names depending on Hermes version:

- `build` · `write_source` · `read_source` · `inspect_refs` · `measure` · `snapshot`
- **H3-3:** `inspect_dims` · `assembly_validate` · `sdf_sample` (secondary)
- **H5-2:** `align_check` · `frame` · `diff`
- **H5-3:** `export` (stl/gltf preview mesh; mock STEP = Unsupported)
- **H5-4:** `fab_check` (DFM preflight; no printer start)
- **H5-5:** `engine` (info / fail-closed install) · `schema` (mcp/errors)
- resources: `resources/list` · `resources/read` (`cadrion://doc/**`)

## Drive example

```text
Use cadrion MCP build on parity/parts/01_*/…cad.star and report volume_mm3.
```
