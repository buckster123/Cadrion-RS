//! `parts.lock` — pinned catalog parts with checksums (fail closed).

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartsLock {
    pub version: u32,
    #[serde(default)]
    pub parts: BTreeMap<String, PartsLockEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartsLockEntry {
    /// Provider id (e.g. `local`, `stepparts`).
    pub provider: String,
    /// Remote or local part id.
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// SHA-256 hex of the STEP (or artifact) bytes.
    pub sha256: String,
    /// Relative path in project cache or tree.
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum PartsLockError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("lock missing entry '{0}'")]
    MissingEntry(String),
    #[error("checksum mismatch for '{0}': lock={1} file={2}")]
    ChecksumMismatch(String, String, String),
    #[error("artifact missing at {0}")]
    ArtifactMissing(String),
}

impl PartsLock {
    pub fn empty() -> Self {
        Self {
            version: 1,
            parts: BTreeMap::new(),
        }
    }
}

pub fn load_parts_lock(path: &Path) -> Result<PartsLock, PartsLockError> {
    let text = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&text)?)
}

/// Fail-closed verify: entry must exist, file must exist, hash must match.
pub fn verify_lock_entry(
    lock: &PartsLock,
    key: &str,
    project_root: &Path,
) -> Result<(), PartsLockError> {
    let entry = lock
        .parts
        .get(key)
        .ok_or_else(|| PartsLockError::MissingEntry(key.into()))?;
    let art = project_root.join(&entry.path);
    if !art.is_file() {
        return Err(PartsLockError::ArtifactMissing(art.display().to_string()));
    }
    let bytes = fs::read(&art)?;
    let got = hex::encode(Sha256::digest(&bytes));
    if !got.eq_ignore_ascii_case(&entry.sha256) {
        return Err(PartsLockError::ChecksumMismatch(
            key.into(),
            entry.sha256.clone(),
            got,
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn lock_roundtrip_and_verify() {
        let mut dir = std::env::temp_dir();
        dir.push(format!(
            "cadrion-lock-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let art = dir.join("bolt.step");
        fs::write(&art, b"ISO-FAKE-STEP").unwrap();
        let hash = hex::encode(Sha256::digest(b"ISO-FAKE-STEP"));
        let mut lock = PartsLock::empty();
        lock.parts.insert(
            "m6_bolt".into(),
            PartsLockEntry {
                provider: "local".into(),
                id: "bolt-m6".into(),
                version: Some("1".into()),
                sha256: hash,
                path: "bolt.step".into(),
                license: Some("CC0".into()),
            },
        );
        let lock_path = dir.join("parts.lock");
        fs::write(&lock_path, serde_json::to_string_pretty(&lock).unwrap()).unwrap();
        let loaded = load_parts_lock(&lock_path).unwrap();
        verify_lock_entry(&loaded, "m6_bolt", &dir).unwrap();
        assert!(verify_lock_entry(&loaded, "nope", &dir).is_err());
        let _ = fs::remove_dir_all(&dir);
    }
}
