//! Content-addressed build cache under `.cadrion/cache/`.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::hashutil::{sha256_bytes, sha256_file, sha256_hex, HashHex};

/// Cache key inputs (FR-105): source + params + cadrion + kernel versions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CacheKey {
    pub source_sha256: HashHex,
    pub params_sha256: HashHex,
    pub cadrion_version: String,
    pub kernel_id: String,
    pub kernel_version: String,
    /// Optional IR schema version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ir_version: Option<u32>,
}

impl CacheKey {
    pub fn from_source(
        source: &str,
        params_json: &str,
        cadrion_version: impl Into<String>,
        kernel_id: impl Into<String>,
        kernel_version: impl Into<String>,
        ir_version: Option<u32>,
    ) -> Self {
        Self {
            source_sha256: sha256_hex(source),
            params_sha256: sha256_hex(params_json),
            cadrion_version: cadrion_version.into(),
            kernel_id: kernel_id.into(),
            kernel_version: kernel_version.into(),
            ir_version,
        }
    }

    /// Stable directory name under the cache root.
    pub fn digest(&self) -> HashHex {
        let payload = serde_json::to_vec(self).expect("CacheKey serializes");
        sha256_bytes(&payload)
    }
}

/// What a successful cache put stores.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CachePut {
    /// Relative or absolute path to primary artifact (STEP).
    pub artifact_path: PathBuf,
    /// Optional IR JSON path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ir_path: Option<PathBuf>,
    /// Facts JSON blob (optional).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub facts_json: Option<String>,
}

/// Stored cache entry metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CacheEntry {
    pub key: CacheKey,
    pub key_digest: HashHex,
    pub artifact_sha256: HashHex,
    pub artifact_path: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ir_path: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub facts_json: Option<String>,
    pub created_unix_ms: u64,
}

/// Filesystem build cache.
#[derive(Debug, Clone)]
pub struct BuildCache {
    root: PathBuf,
}

impl BuildCache {
    /// Open or create a cache at `root` (typically `<project>/.cadrion/cache`).
    pub fn open(root: impl Into<PathBuf>) -> std::io::Result<Self> {
        let root = root.into();
        fs::create_dir_all(&root)?;
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn entry_dir(&self, digest: &str) -> PathBuf {
        self.root.join(digest)
    }

    fn meta_path(&self, digest: &str) -> PathBuf {
        self.entry_dir(digest).join("entry.json")
    }

    /// Lookup a complete entry; verifies artifact hash still matches.
    pub fn get(&self, key: &CacheKey) -> std::io::Result<Option<CacheEntry>> {
        let digest = key.digest();
        let meta = self.meta_path(&digest);
        if !meta.is_file() {
            return Ok(None);
        }
        let text = fs::read_to_string(&meta)?;
        let entry: CacheEntry = serde_json::from_str(&text)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        if entry.key_digest != digest {
            return Ok(None);
        }
        let art = if entry.artifact_path.is_absolute() {
            entry.artifact_path.clone()
        } else {
            self.entry_dir(&digest).join(&entry.artifact_path)
        };
        if !art.is_file() {
            return Ok(None);
        }
        let hash = sha256_file(&art)?;
        if hash != entry.artifact_sha256 {
            // stale / corrupted
            return Ok(None);
        }
        Ok(Some(entry))
    }

    /// Store artifact bytes (and optional IR) under the key digest.
    pub fn put(
        &self,
        key: &CacheKey,
        artifact_bytes: &[u8],
        artifact_name: &str,
        ir_json: Option<&str>,
        facts_json: Option<String>,
    ) -> std::io::Result<CacheEntry> {
        let digest = key.digest();
        let dir = self.entry_dir(&digest);
        fs::create_dir_all(&dir)?;
        let art_path = dir.join(artifact_name);
        fs::write(&art_path, artifact_bytes)?;
        let art_hash = sha256_bytes(artifact_bytes);

        let ir_path = if let Some(ir) = ir_json {
            let p = dir.join("ir.json");
            fs::write(&p, ir)?;
            Some(PathBuf::from("ir.json"))
        } else {
            None
        };

        let entry = CacheEntry {
            key: key.clone(),
            key_digest: digest.clone(),
            artifact_sha256: art_hash,
            artifact_path: PathBuf::from(artifact_name),
            ir_path,
            facts_json,
            created_unix_ms: now_ms(),
        };
        let meta = serde_json::to_vec_pretty(&entry)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        fs::write(self.meta_path(&digest), meta)?;
        Ok(entry)
    }

    /// Resolve absolute artifact path for an entry.
    pub fn artifact_abs(&self, entry: &CacheEntry) -> PathBuf {
        if entry.artifact_path.is_absolute() {
            entry.artifact_path.clone()
        } else {
            self.entry_dir(&entry.key_digest).join(&entry.artifact_path)
        }
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn put_get_hit_and_miss() {
        let dir = tempfile_dir();
        let cache = BuildCache::open(&dir).unwrap();
        let key = CacheKey::from_source(
            "def gen_step():\n  return box(1,2,3)\n",
            "{\"w\":1}",
            "0.1.0",
            "mock",
            "0.1.0-mock",
            Some(0),
        );
        assert!(cache.get(&key).unwrap().is_none());

        let entry = cache
            .put(
                &key,
                b"ISO-10303-21; fake step",
                "part.step",
                Some("{}"),
                None,
            )
            .unwrap();
        let hit = cache.get(&key).unwrap().expect("hit");
        assert_eq!(hit.key_digest, entry.key_digest);
        assert_eq!(hit.artifact_sha256, entry.artifact_sha256);
        let abs = cache.artifact_abs(&hit);
        assert!(abs.is_file());
        assert_eq!(std::fs::read(abs).unwrap(), b"ISO-10303-21; fake step");

        // param change → miss
        let key2 = CacheKey::from_source(
            "def gen_step():\n  return box(1,2,3)\n",
            "{\"w\":2}",
            "0.1.0",
            "mock",
            "0.1.0-mock",
            Some(0),
        );
        assert!(cache.get(&key2).unwrap().is_none());
    }

    #[test]
    fn corruption_misses() {
        let dir = tempfile_dir();
        let cache = BuildCache::open(&dir).unwrap();
        let key = CacheKey::from_source("src", "{}", "0.1.0", "mock", "0", None);
        let entry = cache.put(&key, b"abc", "a.step", None, None).unwrap();
        let abs = cache.artifact_abs(&entry);
        std::fs::write(abs, b"tampered").unwrap();
        assert!(cache.get(&key).unwrap().is_none());
    }

    fn tempfile_dir() -> PathBuf {
        let mut d = std::env::temp_dir();
        d.push(format!(
            "cadrion-cache-test-{}",
            std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        d
    }
}
