#!/usr/bin/env python3
"""Oracle live-harness driver — CI plumbing, not an LLM.

Reads CADRION_HARNESS_TASK_FILE and writes the *last loop's* write content to
CADRION_HARNESS_PART. Proves the live --cmd protocol end-to-end without a model.

Real agents: ignore TASK_FILE solutions; only use CADRION_HARNESS_PROMPT + WORKDIR.
"""
from __future__ import annotations

import json
import os
import sys
from pathlib import Path


def main() -> int:
    task_file = os.environ.get("CADRION_HARNESS_TASK_FILE")
    workdir = os.environ.get("CADRION_HARNESS_WORKDIR")
    part = os.environ.get("CADRION_HARNESS_PART")
    prompt = os.environ.get("CADRION_HARNESS_PROMPT", "")
    task_id = os.environ.get("CADRION_HARNESS_TASK_ID", "?")
    loop_n = os.environ.get("CADRION_HARNESS_LOOP", "1")

    if not task_file or not part:
        print("oracle: missing CADRION_HARNESS_TASK_FILE or CADRION_HARNESS_PART", file=sys.stderr)
        return 2

    task = json.loads(Path(task_file).read_text())
    loops = task.get("loops") or []
    if not loops:
        print(f"oracle: task {task_id} has no scripted loops to oracle from", file=sys.stderr)
        return 2

    # Prefer last loop (successful repair path) for one-shot pass.
    steps = loops[-1]
    writes = [s for s in steps if s.get("op") == "write"]
    if not writes:
        print(f"oracle: no write step in last loop of {task_id}", file=sys.stderr)
        return 2

    w = writes[-1]
    rel = w.get("path", "part.cad.star")
    content = w.get("content", "")
    out = Path(part)
    # If PART is absolute use it; else under workdir
    if not out.is_absolute() and workdir:
        out = Path(workdir) / rel
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(content)
    print(
        f"oracle: task={task_id} loop={loop_n} wrote {out} ({len(content)} bytes) "
        f"prompt_chars={len(prompt)}",
        file=sys.stderr,
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
