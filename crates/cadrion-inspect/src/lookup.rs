//! Shared ref lookup for inspect tools.

use cadrion_model::parse_selector;

use crate::refs::RefEntry;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LookupError {
    #[error("selector: {0}")]
    Selector(String),
    #[error("unknown ref {0}")]
    UnknownRef(String),
}

pub fn lookup_in_report<'a>(refs: &'a [RefEntry], raw: &str) -> Result<&'a RefEntry, LookupError> {
    let sel = parse_selector(raw).map_err(|e| LookupError::Selector(e.to_string()))?;
    let token = sel.to_string();
    refs.iter()
        .find(|r| r.selector == token)
        .ok_or_else(|| LookupError::UnknownRef(raw.to_string()))
}
