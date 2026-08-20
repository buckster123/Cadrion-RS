# Agent harness (`agent10`)

Two modes:

| Mode | How | What the agent sees |
|------|-----|---------------------|
| **scripted** (default) | Built-in step loops | N/A (no external process) |
| **live** | `--cmd '…'` | **Prompt only** + empty workdir |

## Scripted (CI default)

```sh
cargo test -p cadrion-harness
cargo run -p cadrion-cli -- harness run --suite agent10 --json
```

Target bar (PRD M3): **≥ 6/10**.  
Prompts must name labels/sizes the asserts check (`prompt_covers_asserts`, H5-1).

## Live driver protocol

```sh
# Plumbing check (oracle — not an LLM)
cargo run -p cadrion-cli -- harness run --suite agent10 \
  --cmd 'python3 harness/drivers/oracle_agent.py' --json

# Real agent: your process must write $CADRION_HARNESS_PART from the prompt alone.
# Do not read CADRION_HARNESS_TASK_FILE for solutions (oracle only).
export CADRION_BIN="$(pwd)/target/release/cadrion"
export CADRION_HARNESS_MODEL_ID="qwen3.8-27b-ud-q6_k"
cargo run -p cadrion-cli -- harness run --suite agent10 \
  --cmd 'python3 harness/drivers/openai_starlark.py' --timeout 600 --json
```

### Env vars (each loop)

| Variable | Meaning |
|----------|---------|
| `CADRION_HARNESS_TASK_ID` | e.g. `01_block` |
| `CADRION_HARNESS_PROMPT` | Natural language only |
| `CADRION_HARNESS_WORKDIR` | Temp workspace (cwd of `--cmd`) |
| `CADRION_HARNESS_PART` | Absolute path to write (`…/part.cad.star`) |
| `CADRION_HARNESS_LOOP` | 1-based attempt |
| `CADRION_HARNESS_MAX_LOOPS` | Cap (default 3) |
| `CADRION_HARNESS_TASK_FILE` | Task JSON (oracle/debug; **not** for fair LLM runs) |

### Contract

1. Exit **0** when you produced a candidate part.  
2. Leave valid Starlark at `CADRION_HARNESS_PART` with `def gen_step(): …`.  
3. Harness **builds + asserts** (volume/label/faces/snapshot). Fail → next loop.  
4. Scorecard `mode: "live"`; same ≥6/10 target.

### Example agent sketch

```sh
# pseudocode
# read $CADRION_HARNESS_PROMPT
# write Starlark to $CADRION_HARNESS_PART
# optional: $CADRION_BIN build "$CADRION_HARNESS_PART" --json for self-check
```

## Honesty

- Scripted 10/10 ≠ live LLM score.  
- Oracle driver cheats via task file — only for plumbing.  
- Live verify uses **mock** kernel (same as scripted CI path).  
- Snapshot packets are real software renders when not `--no-snapshot`.  
- **Published scores:** [`docs/HARNESS_LIVE.md`](../docs/HARNESS_LIVE.md) · `harness/scores/`.

# In-process oracle (CI)
cargo run -p cadrion-cli -- harness run --suite agent10 --cmd '@oracle' --json
