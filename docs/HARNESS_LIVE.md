# H2-4 — Published live harness score

Living log of **dated** `agent10` runs. A number is only green with evidence — never invented.

## Control run (oracle plumbing)

| Field | Value |
|-------|--------|
| **Date** | **2026-08-06** |
| **Suite** | `agent10` |
| **Mode** | `live` |
| **Cmd** | `@oracle` |
| **Model id** | `oracle:in-process` |
| **Score** | **10.0 / 10** (target ≥ 6.0) |
| **Median loops** | **1.0** |
| **Passed** | 10 / 10 |
| **Kernel** | mock |
| **Artifact** | [`harness/scores/h2-4-oracle-2026-08-06.json`](../harness/scores/h2-4-oracle-2026-08-06.json) |

```sh
cargo run -p cadrion-cli -- harness run --suite agent10 --cmd '@oracle' --json
```

### Honesty

- Oracle **cheats** via task file (writes known-good Starlark).  
- This is a **control / plumbing** score, **not** a frontier LLM claim.  
- Scripted CI path also scores 10/10 — different mode (`scripted-builtin`).  
- At publish time, LocalRouter `http://127.0.0.1:8888/v1/models` was up but **all backends `status=down`** — no fair local/frontier LLM run was possible. **No fake ≥6.**

## Frontier / strong-local (not yet)

| Field | Value |
|-------|--------|
| Date | — |
| Model | — |
| Score | **not run** |
| Notes | Re-run when a healthy OpenAI-compat backend is available |

### How to publish a real agent score

```sh
# Fair: agent must NOT read CADRION_HARNESS_TASK_FILE for answers
export CADRION_BIN="$(pwd)/target/release/cadrion"
cargo run -p cadrion-cli -- harness run --suite agent10 \
  --cmd 'my-agent-runner' --timeout 600 --json \
  | tee harness/scores/YYYY-MM-DD-<model>.json
```

Then add a row here + a METRICS.md line with date, cmd, model_id, score, median_loops, failures.

## Scorecard provenance fields (H2-4)

JSON scorecard now includes when applicable:

| Field | Meaning |
|-------|---------|
| `cmd` | live driver command |
| `model_id` | `oracle:in-process` / `oracle:python` / `scripted-builtin` / `external-cmd` |
| `notes` | honesty strings |

## Related

- Protocol: `harness/README.md`  
- H1 live driver: `@oracle` CI path  
- METRICS row 38 (H2-4)
