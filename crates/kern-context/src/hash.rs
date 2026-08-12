//! Deterministic content hashing.
//!
//! FNV-1a (64-bit) is used for *content identity*, cache validation, stale detection, and
//! the canonical `ContextPack` hash. It is deterministic and dependency-free. It is **not**
//! cryptographic and is not a signature — it exists to detect change and enable
//! reproducible replay, not to resist a motivated forger. Where tamper-resistance is later
//! required, this is the single place to swap in a cryptographic digest.

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// FNV-1a 64-bit over bytes.
#[must_use]
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut h = FNV_OFFSET;
    for &b in bytes {
        h ^= u64::from(b);
        h = h.wrapping_mul(FNV_PRIME);
    }
    h
}

/// A prefixed, lowercase-hex content hash string, e.g. `fnv1a64:9d2c...`.
#[must_use]
pub fn content_hash(bytes: &[u8]) -> String {
    format!("fnv1a64:{:016x}", fnv1a64(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_and_sensitive() {
        assert_eq!(content_hash(b"hello"), content_hash(b"hello"));
        assert_ne!(content_hash(b"hello"), content_hash(b"hellp"));
        assert!(content_hash(b"x").starts_with("fnv1a64:"));
    }

    #[test]
    fn empty_is_offset_basis() {
        assert_eq!(fnv1a64(b""), FNV_OFFSET);
    }
}
