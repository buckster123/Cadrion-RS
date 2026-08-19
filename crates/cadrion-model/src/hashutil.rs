//! Hashing helpers — content addresses for cache keys.

use sha2::{Digest, Sha256};
use std::path::Path;

/// Lowercase hex SHA-256.
pub type HashHex = String;

/// SHA-256 of raw bytes → hex.
pub fn sha256_bytes(data: &[u8]) -> HashHex {
    let mut h = Sha256::new();
    h.update(data);
    hex::encode(h.finalize())
}

/// SHA-256 hex of a string.
pub fn sha256_hex(s: &str) -> HashHex {
    sha256_bytes(s.as_bytes())
}

/// SHA-256 of file contents.
pub fn sha256_file(path: &Path) -> std::io::Result<HashHex> {
    let data = std::fs::read(path)?;
    Ok(sha256_bytes(&data))
}

/// Hash canonical JSON (value serialized, then hashed). Not JCS — stable enough via serde_json
/// default map key order when built from BTreeMap.
pub fn canonical_json_hash<T: serde::Serialize>(value: &T) -> Result<HashHex, serde_json::Error> {
    let bytes = serde_json::to_vec(value)?;
    Ok(sha256_bytes(&bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn stable_params_hash() {
        let mut a = BTreeMap::new();
        a.insert("width", 100.0);
        a.insert("depth", 60.0);
        let mut b = BTreeMap::new();
        b.insert("depth", 60.0);
        b.insert("width", 100.0);
        assert_eq!(
            canonical_json_hash(&a).unwrap(),
            canonical_json_hash(&b).unwrap()
        );
    }
}
