//! PartProvider trait + local filesystem provider.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::lock::{PartsLock, PartsLockEntry};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartCandidate {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub dims_mm: std::collections::BTreeMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartMeta {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    pub sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartRef {
    pub provider: String,
    pub id: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("not found: {0}")]
    NotFound(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Msg(String),
}

pub trait PartProvider: Send + Sync {
    fn id(&self) -> &str;
    fn search(&self, query: &str) -> Result<Vec<PartCandidate>, ProviderError>;
    fn fetch(&self, id: &str) -> Result<PartMeta, ProviderError>;
}

/// Scans a directory of `.step`/`.stp` files; id = stem.
pub struct LocalFsProvider {
    root: PathBuf,
}

impl LocalFsProvider {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn pin_to_lock(&self, id: &str, _lock_key: &str) -> Result<PartsLockEntry, ProviderError> {
        let meta = self.fetch(id)?;
        let rel = meta
            .path
            .strip_prefix(&self.root)
            .unwrap_or(&meta.path)
            .to_string_lossy()
            .into_owned();
        Ok(PartsLockEntry {
            provider: self.id().into(),
            id: meta.id,
            version: None,
            sha256: meta.sha256,
            path: rel,
            license: meta.license,
        })
    }
}

impl PartProvider for LocalFsProvider {
    fn id(&self) -> &str {
        "local"
    }

    fn search(&self, query: &str) -> Result<Vec<PartCandidate>, ProviderError> {
        let q = query.to_ascii_lowercase();
        let mut out = Vec::new();
        if !self.root.is_dir() {
            return Ok(out);
        }
        for ent in fs::read_dir(&self.root)? {
            let ent = ent?;
            let name = ent.file_name().to_string_lossy().into_owned();
            let lower = name.to_ascii_lowercase();
            if !(lower.ends_with(".step") || lower.ends_with(".stp")) {
                continue;
            }
            if q.is_empty() || lower.contains(&q) {
                let stem = Path::new(&name)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or(&name)
                    .to_string();
                out.push(PartCandidate {
                    id: stem.clone(),
                    name: stem,
                    aliases: vec![],
                    dims_mm: Default::default(),
                });
            }
        }
        out.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(out)
    }

    fn fetch(&self, id: &str) -> Result<PartMeta, ProviderError> {
        for ext in [".step", ".stp", ".STEP", ".STP"] {
            let p = self.root.join(format!("{id}{ext}"));
            if p.is_file() {
                let bytes = fs::read(&p)?;
                let sha = hex::encode(Sha256::digest(&bytes));
                return Ok(PartMeta {
                    id: id.into(),
                    name: id.into(),
                    path: p,
                    sha256: sha,
                    license: None,
                });
            }
        }
        Err(ProviderError::NotFound(id.into()))
    }
}

/// Helper: ensure all assembly component lock keys verify.
#[allow(dead_code)]
pub fn verify_assembly_locks(
    lock: &PartsLock,
    keys: &[String],
    root: &Path,
) -> Result<(), crate::PartsLockError> {
    for k in keys {
        crate::verify_lock_entry(lock, k, root)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn local_search_fetch() {
        let mut dir = std::env::temp_dir();
        dir.push(format!(
            "cadrion-prov-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("washer_m6.step"), b"STEP").unwrap();
        let p = LocalFsProvider::new(&dir);
        let hits = p.search("washer").unwrap();
        assert_eq!(hits.len(), 1);
        let m = p.fetch("washer_m6").unwrap();
        assert_eq!(m.sha256.len(), 64);
        let _ = fs::remove_dir_all(&dir);
    }
}
