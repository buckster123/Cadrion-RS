//! Selectors, content hashing, and filesystem build cache.

#![deny(unsafe_code)]

mod cache;
mod hashutil;
mod selector;

pub use cache::{BuildCache, CacheEntry, CacheKey, CachePut};
pub use hashutil::{canonical_json_hash, sha256_bytes, sha256_file, sha256_hex, HashHex};
pub use selector::{
    assign_face_indices, assign_solid_indices, parse_selector, sort_key_face, sort_key_solid,
    EntityKind, Selector, SelectorError,
};

/// Crate version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
