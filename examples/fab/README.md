# Fab path examples (S11+)

```sh
# DXF plate + holes
cargo run -p cadrion-cli -- fab dxf --width 100 --height 50 \
  --hole 25,25,6 --hole 75,25,6 -o /tmp/plate.dxf --json

# Face → DXF from a part (largest +Z face outline)
cargo run -p cadrion-cli -- fab dxf-face parity/parts/01_calibration_block/part.cad.star \
  --normal 0,0,1 -o /tmp/face.dxf --json

# DFM preflight (bundled profiles)
cargo run -p cadrion-cli -- fab profiles --json
cargo run -p cadrion-cli -- fab check --part-json examples/fab/plate.flat.json --json
cargo run -p cadrion-cli -- fab check --profile pcb.outline \
  --part-json examples/fab/pcb.flat.json --json

# Slicer discovery + gated execute
cargo run -p cadrion-cli -- fab slicers --json
cargo run -p cadrion-cli -- fab slice mesh.stl --json
# LIVE slice (second consent):
cargo run -p cadrion-cli -- fab slice mesh.stl --execute --confirm SLICE \
  --allowlist prusa-slicer -o out.gcode --json

# G-code static check
cargo run -p cadrion-cli -- fab gcode-check examples/fab/sample.gcode --json

# Printer dry-run (no network) — prints sha256 for the start gate
cargo run -p cadrion-cli -- printer dry-run examples/fab/sample.gcode --json

# Gates only (still no network — missing --live)
cargo run -p cadrion-cli -- printer start examples/fab/sample.gcode \
  --sha256 <from dry-run> --confirm START --allowlist bambu:x1c-01 --json

# LIVE start (YOU opt in): FTPS upload + MQTT after all gates
# Needs: curl, mosquitto_pub, LAN access code, printer serial
export CADRION_BAMBU_ACCESS_CODE=xxxxxxxx
export CADRION_BAMBU_SERIAL=01P00A000000000
cargo run -p cadrion-cli -- printer start examples/fab/sample.gcode \
  --sha256 <from dry-run> --confirm START --allowlist bambu:x1c-01 \
  --host 192.168.1.50 --live --json

# --- Klipper / Moonraker (H9) ---
cargo run -p cadrion-cli -- printer dry-run examples/fab/sample.gcode \
  --backend klipper --id klipper:ender --host 192.168.1.60 --json
cargo run -p cadrion-cli -- printer start examples/fab/sample.gcode \
  --backend klipper --id klipper:ender --host 192.168.1.60 \
  --sha256 <from dry-run> --confirm START --allowlist klipper:ender --json
# LIVE Moonraker:
cargo run -p cadrion-cli -- printer start examples/fab/sample.gcode \
  --backend klipper --id klipper:ender --host 192.168.1.60 \
  --sha256 <from dry-run> --confirm START --allowlist klipper:ender --live --json

# --- OctoPrint (H2-3) ---
cargo run -p cadrion-cli -- fab check --profile waterjet \
  --part-json examples/fab/waterjet.flat.json --json
cargo run -p cadrion-cli -- printer dry-run examples/fab/sample.gcode \
  --id octoprint:pi --host 192.168.1.70 --json
```

## Safety gates (all required before any network)

| Gate | How you open it |
|------|-----------------|
| **allowlist** | `--allowlist bambu:x1c-01` / `klipper:ender` / `octoprint:pi` |
| **sha256** | must match file; copy from `printer dry-run` |
| **confirm** | exactly `--confirm START` (case-sensitive) |
| **gcode-check** | static validation must pass |
| **`--live`** | second consent: without it, gates may pass but **no sockets** |
| **credentials** | Bambu: access-code+serial · Klipper: optional API key · OctoPrint: API key required for live |

Live Bambu: `curl` FTPS + `mosquitto_pub` MQTT.  
Live Klipper: `curl` → Moonraker upload + print/start (`docs/KLIPPER.md`).  
Live OctoPrint: `curl` → `/api/files/local` (`docs/FAB_DEPTH.md`).
