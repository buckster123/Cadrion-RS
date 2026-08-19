# Name decision (H3-10 / OQ-1)

**Decision date:** 2026-08-19  
**Decision:** **RENAME** to **Cadrion** / garden name **Cadrion-RS**.  
**Not:** keep Cadre (too close to [cadre3d.com](https://cadre3d.com/)).  
**Not:** Cadius (Apple II ProDOS CLI + Cadence echo).

Sweep date: 2026-08-19. Informal registry/product check, not legal advice.

## Why Cadrion

André picked it after the Cadre sweep showed a same-class collision. Cadrion is free on
crates.io (`cadrion`, `cadrion-cli`, `cadrion-*`), pronounceable, and not a live CAD
product. The in-tree rename landed in this slice.

## Why not Cadre

| Collision | What it is |
|-----------|------------|
| [cadre3d.com](https://cadre3d.com/) | Live browser parametric CAD + AI manufacturability review — **same job**. |
| crates.io `cadre` 0.5.4 | Modal Labs remote-config (archived). `cargo install cadre` is **not** us. |
| npm `@cadre-dev/cadre` | Unrelated agent orchestrator; also installs a `cadre` CLI. |

An earlier same-day KEEP draft is superseded by this rename.

## Locked names

| Surface | Name |
|---------|------|
| Product / docs | Cadrion |
| Garden / GitHub | Cadrion-RS |
| Binary | `cadrion` (`cadrion-cli` `[[bin]]`) |
| Workspace crates | `cadrion`, `cadrion-*` |
| First crates.io install crate (when published) | `cadrion-cli` |
| MCP / Hermes | `cadrion` / `mcp_servers.cadrion` |
| Skill pack | `skills/cadrion` |
| Cerebro | `agent_id=CADRION` (D15) |

## Aliases (do not break old env / JSON)

| New | Still accepted |
|-----|----------------|
| `CADRION_*` | `CADRE_*` at library reads (MCP policy, printer, framing) |
| `cadrion://doc/**` | `cadre://doc/**` |
| `cadrion.dfm_profile` / `cadrion.dfm_override` | `cadre.dfm_*` |
| `.cadrion/` cache dir | `.cadre/` still gitignored |

## Honesty lines

- `cargo install cadre` is still Modal’s archived config server.
- Install us with `cargo build -p cadrion-cli --release` or later `cargo install cadrion-cli`.
- Local garden folder may still be `~/Projects/Cadre-RS` until the directory is moved.
- No trademark filing this slice.

## When to reopen OQ-1

Re-open only with a CHARTER amendment if **any** hold:

1. `cadrion` / `cadrion-cli` is taken on crates.io before first publish  
2. A same-class Cadrion product or mark appears  
3. André wants another noun  

Otherwise the name is closed.
