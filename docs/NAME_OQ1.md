# Name decision (H3-10 / OQ-1)

**Decision date:** 2026-08-19  
**Decision:** **KEEP** product name **Cadre** / garden name **Cadre-RS**.  
**Not:** rename the binary, repo, crates, MCP server, skill pack, or Cerebro agent.  
**Not:** publish the workspace facade crate as crates.io `cadre` — that name is taken.

Sweep date: 2026-08-19. Informal registry/product check, not legal advice.

## Why keep

André likes the name. The public repo is already `buckster123/Cadre-RS`. Rename cost is
large and rising (see below). The collisions that exist do **not** require a product
rename if publish and install paths stay honest.

| Collision | What it is | Why it does not force a rename |
|-----------|------------|--------------------------------|
| crates.io `cadre` 0.5.4 | Modal Labs remote-config service; last publish 2022-11-07; GitHub `modal-labs/cadre` **archived**; ~12k downloads, ~50 recent; **bin name `cadre`** | Different domain (S3 config, not CAD). We never publish *our* facade as `cadre`. `cargo install cadre` is **not** this project. |
| [cadre3d.com](https://cadre3d.com/) | Live browser parametric CAD + AI manufacturability review | Closest same-class product. Different form: hosted sketch/SaaS vs local Rust Starlark → B-rep → MCP/CLI. Public face stays **Cadre-RS**. |
| CADRE Analytic / CADRE Pro | Windows FEA structural analysis | Adjacent engineering; not agent CAD. |
| npm `@cadre-dev/cadre` | Agent issue→PR orchestrator; global CLI also named `cadre` | PATH collision if someone `npm i -g` that package. Unrelated product. |
| PyPI `cadre` 0.4.0 | Modal-owned stub (owner `luis-modal`, 2022) | We do not ship Python. |
| npm `cadre` 0.0.0 | Empty placeholder (2016) | Unused. |
| USPTO live marks “CADRE” | Staffing (Cadre, LLC) and investment SaaS (QUADRO PARTNERS) — Class 35/42, not CAD | Informal; no Class 9 CAD-software registration found in this sweep. **No filing this slice.** |

## Locked names

| Surface | Name | Notes |
|---------|------|-------|
| Product / docs | Cadre | Short form |
| Garden / GitHub | Cadre-RS | `https://github.com/buckster123/Cadre-RS` |
| Local / Hermes binary | `cadre` | `cadre-cli` `[[bin]]`; install via `cargo build -p cadre-cli`, not `cargo install cadre` |
| Workspace crates | `cadre`, `cadre-*` | Path deps. Facade `name = "cadre"` stays **unpublished**. |
| First crates.io install crate (when we publish) | `cadre-cli` | Free as of sweep. Bin stays `cadre`. |
| Optional later libs | `cadre-kernel`, `cadre-lang`, … | All `cadre-*` workspace names were **404** on crates.io 2026-08-19. |
| MCP / Hermes | `cadre` / `mcp_servers.cadre` | Unchanged |
| Skill pack | `skills/cadre` | Unchanged |
| Cerebro | `agent_id=CADRE` (D15) | Unchanged |

`cadre-rs` and `cadre-cad` are also free. Held as fallbacks if `cadre-cli` is squatted
before first publish. Do **not** squat-reserve them this slice (no empty crates).

## Honesty lines

- `cargo install cadre` installs Modal’s archived config server. Ours is
  `cargo install --git https://github.com/buckster123/Cadre-RS cadre-cli`
  (or a later crates.io `cadre-cli` once published).
- Facade crate `cadre` has `publish = false` so a workspace publish cannot collide.
- Public marketing says **Cadre-RS** when the short name could mean cadre3d.com.
- Trademark: **do not file** without counsel. Revisit only if we ship a paid/public
  binary under the mark or receive a conflict.

## Options considered

| Option | Verdict |
|--------|---------|
| **(a) KEEP Cadre-RS + publish fences** | **Taken.** Matches preference; rename cost avoided; crates.io honesty held. |
| (b) Rename product now | Rejected. Same-class Cadre 3D is real, but form-factor differs; cost is 1–2 dedicated slices (see below) for no agent-loop gain. |
| (c) Defer OQ-1 | Rejected. Sweep is enough to close. |
| (d) Claim crates.io `cadre` | Impossible without Modal transfer. Do not ask; do not impersonate. |

## Rename cost (if a later GO)

A product rename is a **follow-up slice**, not this one. Rough map:

| Bucket | Hits |
|--------|------|
| Workspace crate `name =` + path dirs | 18 members + facade |
| Binary + `default-run` + Hermes `~/.local/bin` + `mcp_servers.*` | `cadre` |
| Skill pack path + export names | `skills/cadre` |
| Cerebro `agent_id` | `CADRE` (D15) |
| GitHub repo + topics + README/banner | `Cadre-RS` |
| Docs, examples, MCP `cadre://` URIs, schema titles | widespread |

Estimate: **one docs+crate rename slice** (mechanical) plus **one field slice** (Hermes
re-wire, skill re-export, Cerebro agent id). Do not start unless CHARTER amends D18.

## When to reopen OQ-1

Re-open only with a CHARTER amendment if **any** hold:

1. Modal (or successor) objects to our local binary name, or `cadre-cli` is taken before publish  
2. Cadre 3D (cadre3d.com) or a Class 9 CAD mark holder sends a conflict  
3. André wants a cleaner public noun than Cadre-RS  

Otherwise the name is closed.
