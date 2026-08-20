#!/usr/bin/env python3
"""Fair live-harness driver: prompt → OpenAI-compat chat → write $CADRION_HARNESS_PART.

Does **not** read CADRION_HARNESS_TASK_FILE (oracle-only). Optional self-check via
$CADRION_BIN build --json, then one repair completion if eval/execute failed.

Env:
  CADRION_HARNESS_PROMPT / _PART / _TASK_ID / _LOOP / _MAX_LOOPS / _WORKDIR
  OPENAI_BASE_URL   default http://127.0.0.1:8888/v1
  OPENAI_API_KEY    default cadrion-harness (ApexRouter accepts any non-empty)
  CADRION_HARNESS_LLM / OPENAI_MODEL   default auto
  CADRION_BIN       optional cadrion binary for self-check
  CADRION_HARNESS_MAX_TOKENS   completion budget (default 8192; thinking burns this)
"""
from __future__ import annotations

import json
import os
import re
import subprocess
import sys
import urllib.error
import urllib.request
from pathlib import Path

SYSTEM = """You are a Cadrion CAD author. Reply with ONE hermetic Starlark part and nothing else.

Rules:
- Units millimetres. File must define `def gen_step():` that returns `solid(..., label=...)`.
- No imports, no load(), no network, no comments that are not Starlark.
- Prefer `P = params(...)` with float fields when dimensions are named.
- Primitives: `box(dx, dy, dz, at=CENTER|(x,y,z))`, `cylinder(radius, height, at=...)`.
  `cylinder` takes **radius**, not diameter — an 8 mm hole is `cylinder(4.0, h, at=...)`.
- Booleans: `cut(a, b)`, `union(a, b)`. Wrap the result: `return solid(shape, label="name")`.
- Placement: `CENTER` is the origin; `at=(x, y, z)` is the solid's center (mm).
- Labels: use the exact word the user asked for (block, plate, pin, cube, brick,
  calibration, l_bracket, union_body, …). If they only say “labeled X”, label is X.
- Through-holes: make the cylinder taller than the plate and drop it by ~1 mm so it
  cuts cleanly (`h+2.0`, `at=(0.0, 0.0, -1.0)`).
- When they give volume bounds (e.g. “stay above 110k after hole”), size the hole
  small enough that plate minus hole stays in range. A 100×60×20 plate is 120000 mm³.
- When they give final size (e.g. 30×20×10), use those numbers exactly.
- Output only the Starlark source, optionally in a ```star fence. No prose.
"""


def extract_starlark(text: str) -> str:
    """Pull Starlark from a chat completion; thinking wrappers are discarded."""
    raw = (text or "").strip()
    raw = re.sub(r"<think>.*?</think>", "", raw, flags=re.S | re.I)
    raw = re.sub(r"<\|?redacted_thinking\|?>.*?<\|?redacted_thinking\|?>", "", raw, flags=re.S)
    raw = raw.strip()
    fences = re.findall(r"```(?:starlark|star|python|py)?\s*\n(.*?)```", raw, flags=re.S | re.I)
    for block in fences:
        if "def gen_step" in block:
            return block.strip() + "\n"
    if fences:
        return fences[-1].strip() + "\n"
    idx = raw.find("def gen_step")
    if idx >= 0:
        return raw[idx:].strip() + "\n"
    return raw + ("\n" if raw and not raw.endswith("\n") else "")


def _message_text(msg: dict) -> str:
    parts: list[str] = []
    for key in ("content", "reasoning_content"):
        val = msg.get(key)
        if isinstance(val, str) and val.strip():
            parts.append(val)
        elif isinstance(val, list):
            for item in val:
                if isinstance(item, dict):
                    t = item.get("text") or item.get("content")
                    if isinstance(t, str) and t.strip():
                        parts.append(t)
    return "\n".join(parts)


def chat(base: str, key: str, model: str, messages: list[dict], max_tokens: int) -> str:
    url = base.rstrip("/") + "/chat/completions"
    body = json.dumps(
        {
            "model": model,
            "messages": messages,
            "temperature": 0.2,
            "max_tokens": max_tokens,
        }
    ).encode()
    req = urllib.request.Request(
        url,
        data=body,
        headers={
            "Content-Type": "application/json",
            "Authorization": f"Bearer {key}",
        },
        method="POST",
    )
    try:
        with urllib.request.urlopen(req, timeout=600) as resp:
            payload = json.loads(resp.read().decode())
    except urllib.error.HTTPError as e:
        err = e.read().decode(errors="replace")
        raise RuntimeError(f"chat HTTP {e.code}: {err[:400]}") from e
    choices = payload.get("choices") or []
    if not choices:
        raise RuntimeError(f"chat: empty choices: {str(payload)[:300]}")
    text = _message_text(choices[0].get("message") or {})
    if not text.strip():
        raise RuntimeError("chat: empty content and reasoning_content")
    return text


def self_check(cadrion: str, part: Path) -> str | None:
    try:
        proc = subprocess.run(
            [cadrion, "build", str(part), "--json"],
            capture_output=True,
            text=True,
            timeout=60,
        )
    except (OSError, subprocess.TimeoutExpired) as e:
        print(f"openai_starlark: self-check skipped ({e})", file=sys.stderr)
        return None
    raw = (proc.stdout or "").strip() or (proc.stderr or "").strip()
    try:
        data = json.loads(raw) if raw.startswith("{") else {}
    except json.JSONDecodeError:
        data = {}
    if data.get("ok") is True:
        return None
    diags = data.get("diagnostics") or raw[:500]
    return json.dumps(diags, default=str)[:800]


def write_part(path: Path, source: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(source if source.endswith("\n") else source + "\n")


def main() -> int:
    if "--self-test" in sys.argv:
        sample = "<think>nope</think>\n```star\ndef gen_step():\n    return solid(box(1.0, 2.0, 3.0, at=CENTER), label=\"t\")\n```\n"
        got = extract_starlark(sample)
        assert "def gen_step" in got and "nope" not in got, got
        print("openai_starlark: self-test ok")
        return 0

    prompt = os.environ.get("CADRION_HARNESS_PROMPT", "").strip()
    part = os.environ.get("CADRION_HARNESS_PART")
    task_id = os.environ.get("CADRION_HARNESS_TASK_ID", "?")
    loop_n = os.environ.get("CADRION_HARNESS_LOOP", "1")
    if not part:
        print("openai_starlark: missing CADRION_HARNESS_PART", file=sys.stderr)
        return 2
    if not prompt:
        print("openai_starlark: missing CADRION_HARNESS_PROMPT", file=sys.stderr)
        return 2

    base = os.environ.get("OPENAI_BASE_URL", "http://127.0.0.1:8888/v1")
    key = os.environ.get("OPENAI_API_KEY", "cadrion-harness")
    model = os.environ.get("CADRION_HARNESS_LLM") or os.environ.get("OPENAI_MODEL") or "auto"
    cadrion = os.environ.get("CADRION_BIN", "").strip()
    max_tokens = int(os.environ.get("CADRION_HARNESS_MAX_TOKENS", "8192"))
    out = Path(part)

    user = f"Task {task_id} (attempt {loop_n}). Write part.cad.star for:\n\n{prompt}\n"
    messages = [
        {"role": "system", "content": SYSTEM},
        {"role": "user", "content": user},
    ]
    try:
        text = chat(base, key, model, messages, max_tokens=max_tokens)
    except Exception as e:
        print(f"openai_starlark: {e}", file=sys.stderr)
        return 3
    source = extract_starlark(text)
    if "def gen_step" not in source:
        print("openai_starlark: model returned no gen_step()", file=sys.stderr)
        return 4
    write_part(out, source)

    if cadrion:
        err = self_check(cadrion, out)
        if err:
            print(f"openai_starlark: build failed, repairing: {err}", file=sys.stderr)
            messages.append({"role": "assistant", "content": source})
            messages.append(
                {
                    "role": "user",
                    "content": (
                        "cadrion build failed. Fix the Starlark only. Diagnostics:\n"
                        f"{err}\n"
                    ),
                }
            )
            try:
                text = chat(base, key, model, messages, max_tokens=max_tokens)
                source = extract_starlark(text)
                if "def gen_step" in source:
                    write_part(out, source)
            except Exception as e:
                print(f"openai_starlark: repair chat failed ({e}); keeping first write", file=sys.stderr)

    print(
        f"openai_starlark: task={task_id} loop={loop_n} wrote {out} ({out.stat().st_size} bytes)",
        file=sys.stderr,
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
