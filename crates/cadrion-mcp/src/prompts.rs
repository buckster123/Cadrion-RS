//! MCP prompts (H5-6). Doctrine text, not a fourth face.

use serde_json::{json, Value};

pub struct Prompt {
    pub name: &'static str,
    pub description: &'static str,
    pub text: &'static str,
}

pub const PROMPTS: &[Prompt] = &[
    Prompt {
        name: "cadrion-loop",
        description: "Write → build → inspect → snapshot. Numeric before visual.",
        text: r#"Cadrion loop (one part, one path):

1. Write an explicit `.cad.star` with `def gen_step():` (MCP `write_source` or the agent's FS tools).
2. `build` that path. Never scan a directory.
3. `inspect_refs` / `measure` (numeric facts before any picture).
4. `snapshot` after visible geometry changes and review the images.
5. Only then export / fab_check. Printer start stays CLI-gated (`confirm=START`).

Defaults: millimeters, XY base, +Z up. Mock kernel is default CI; STEP needs OCCT.
Longer doctrine: cadrion://doc/status · cadrion://doc/stdlib
"#,
    },
    Prompt {
        name: "write-source-policy",
        description: "When write_source is on/off (stdio vs HTTP).",
        text: r#"write_source is gated by transport (H7 / OQ-5):

- stdio (`cadrion mcp`): OFF unless CADRION_MCP_WRITE_SOURCE=1
- HTTP (`cadrion serve mcp`): ON unless CADRION_MCP_WRITE_SOURCE=0

Local agents already have filesystem tools — prefer those on stdio.
HTTP agents need MCP write. `read_source` stays on both.

Full policy: cadrion://doc/write-source-policy
"#,
    },
    Prompt {
        name: "hermetic-load",
        description: "Starlark is hermetic: user load() is refused (D9).",
        text: r#"Cadrion Starlark is hermetic (D9):

- `load()` / ambient filesystem modules are refused (`CADRION-E-HERMETIC-LOAD`).
- Stdlib names (`box`, `cylinder`, `solid`, `cut`, …) are already global.
- `use("cadrion.patterns")` is not a user library path — do not invent `load("foo.star")`.
- Parameters: `P = params(...)` and build-time `set` / `--set`. No env, no network, no clock.

This prompt teaches; it does not replace the skill pack.
See cadrion://doc/stdlib
"#,
    },
];

pub fn list_prompts() -> Value {
    json!({
        "prompts": PROMPTS.iter().map(|p| json!({
            "name": p.name,
            "description": p.description,
        })).collect::<Vec<_>>()
    })
}

pub fn get_prompt(name: &str) -> Result<Value, String> {
    let p = PROMPTS
        .iter()
        .find(|p| p.name == name)
        .ok_or_else(|| format!("unknown prompt {name:?}"))?;
    Ok(json!({
        "description": p.description,
        "messages": [{
            "role": "user",
            "content": {"type": "text", "text": p.text}
        }]
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_is_nonempty_and_get_returns_each() {
        let list = list_prompts();
        let arr = list["prompts"].as_array().unwrap();
        assert_eq!(arr.len(), PROMPTS.len());
        assert!(arr.len() >= 3);
        for p in PROMPTS {
            let got = get_prompt(p.name).unwrap();
            let text = got["messages"][0]["content"]["text"].as_str().unwrap();
            assert!(!text.is_empty(), "{}", p.name);
        }
        assert!(get_prompt("not-a-prompt").is_err());
    }
}
