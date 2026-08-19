//! STEP I/O options (AP242 write / AP214|AP242 read).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepWriteOpts {
    /// Prefer AP242; backends may fall back with a diagnostic note.
    pub schema: StepSchema,
    /// Pin header timestamp for reproducible bytes when true.
    pub reproducible: bool,
    /// Optional product name / label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl Default for StepWriteOpts {
    fn default() -> Self {
        Self {
            schema: StepSchema::Ap242,
            reproducible: false,
            name: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepSchema {
    Ap242,
    Ap214,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepReadKind {
    Part,
    Assembly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepReadOpts {
    pub kind: StepReadKind,
}

impl Default for StepReadOpts {
    fn default() -> Self {
        Self {
            kind: StepReadKind::Part,
        }
    }
}
