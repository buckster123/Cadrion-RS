# H2-3 — Fab depth (DFM + OctoPrint)

## DFM profiles (bundled ≥3)

| Id | Alias | Use |
|----|-------|-----|
| `sendcutsend.laser` | `scs` | sheet laser (default) |
| `pcb.outline` | `pcb` | FR4 / Al PCB |
| `waterjet.generic` | `waterjet`, `wj` | waterjet / abrasive cut |

```sh
cargo run -p cadre-cli -- fab profiles --json
cargo run -p cadre-cli -- fab check --profile waterjet \
  --part-json examples/fab/waterjet.flat.json --json
```

## OctoPrint backend

Same gates as Bambu/Klipper: allowlist · sha256 · `confirm=START` · gcode-check · `--live`.

```sh
cargo run -p cadre-cli -- printer dry-run examples/fab/sample.gcode \
  --id octoprint:pi --host 192.168.1.70 --json

cargo run -p cadre-cli -- printer start examples/fab/sample.gcode \
  --backend octoprint --id octoprint:pi --host 192.168.1.70 \
  --sha256 <from-dry-run> --confirm START --allowlist octoprint:pi --json

# LIVE
export CADRE_OCTOPRINT_API_KEY=xxxxxxxx
cargo run -p cadre-cli -- printer start examples/fab/sample.gcode \
  --backend octoprint --id octoprint:pi --host 192.168.1.70 \
  --sha256 <from-dry-run> --confirm START --allowlist octoprint:pi --live --json
```

Id prefixes `octoprint:` / `octo:` auto-select backend.

### Live path

`POST {url}/api/files/local` multipart (`file`, `select=true`, `print=true`) via curl + `X-Api-Key`.

| Env | Role |
|-----|------|
| `CADRE_OCTOPRINT_API_KEY` | API key (required for live) |
| `CADRE_OCTOPRINT_URL` | base URL override |
| `CADRE_CURL` | curl binary |

`--moonraker-url` is reused as a generic base-URL override for klipper/octoprint when set.

## Honesty

- Profiles are versioned data, not live vendor quotes
- Overrides: `docs/DFM_GOVERNANCE.md` (H3-8) — pin `base_version` or fail
- OctoPrint API can drift; not a full plugin host
- Gates never skipped for demos
