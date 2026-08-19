//! Structured diagnostics — the JSON agents see on `--json` failures.

use serde::{Deserialize, Serialize};

/// Severity for a diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

/// Source location (1-based line/col when known).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    pub file: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub col: Option<u32>,
}

/// One diagnostic object (design.md / PRD §10).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub code: String,
    pub severity: Severity,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub span: Option<Span>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub docs_url: Option<String>,
}

impl Diagnostic {
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            severity: Severity::Error,
            message: message.into(),
            target: None,
            span: None,
            refs: Vec::new(),
            hint: None,
            docs_url: None,
        }
    }

    pub fn with_target(mut self, target: impl Into<String>) -> Self {
        self.target = Some(target.into());
        self
    }

    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    pub fn with_span(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }
}

/// Parse a starlark/anyhow error string into a diagnostic (best-effort spans).
pub fn diagnostic_from_error(target: &str, err: &str) -> Diagnostic {
    let (line, col) = parse_line_col(err);
    let mut d = Diagnostic::error("CADRION-E-EVAL", err.to_string()).with_target(target);
    if line.is_some() || col.is_some() {
        d = d.with_span(Span {
            file: target.to_string(),
            line,
            col,
        });
    }
    // Common hints
    if err.contains("not found") && err.contains("gen_step") {
        d = d.with_hint("define def gen_step(): ... that returns a solid shape id");
    } else if err.contains("load(") || err.to_lowercase().contains("load ") {
        d = d.with_hint("model code is hermetic — load() is disabled in cadrion-lang");
    }
    d
}

fn parse_line_col(err: &str) -> (Option<u32>, Option<u32>) {
    // starlark often formats "file:line:col: message" or "--> file:line:col"
    for part in err.split_whitespace() {
        let p = part.trim_matches(|c| c == '`' || c == '\'' || c == ',');
        let bits: Vec<&str> = p.split(':').collect();
        if bits.len() >= 3 {
            if let (Ok(line), Ok(col)) =
                (bits[bits.len() - 2].parse(), bits[bits.len() - 1].parse())
            {
                if line > 0 && col > 0 {
                    return (Some(line), Some(col));
                }
            }
        }
        if bits.len() >= 2 {
            if let Ok(line) = bits[bits.len() - 1].parse::<u32>() {
                if line > 0 && bits[0].contains('.') {
                    return (Some(line), None);
                }
            }
        }
    }
    (None, None)
}
