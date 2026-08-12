//! Deadlines.
//!
//! `now` is always passed in explicitly rather than read from the clock inside the
//! predicate, so expiry is testable without sleeping and cannot silently depend on
//! ambient wall-clock state.

use std::time::{Duration, SystemTime};

/// A point in time after which an authority is no longer valid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Deadline(SystemTime);

impl Deadline {
    /// A deadline at an absolute instant.
    #[must_use]
    pub fn at(instant: SystemTime) -> Self {
        Self(instant)
    }

    /// A deadline `ttl` after `now`.
    #[must_use]
    pub fn after(now: SystemTime, ttl: Duration) -> Self {
        Self(now + ttl)
    }

    /// The absolute instant.
    #[must_use]
    pub fn instant(&self) -> SystemTime {
        self.0
    }

    /// True when `now` is at or past the deadline.
    #[must_use]
    pub fn is_expired(&self, now: SystemTime) -> bool {
        now >= self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_expired_before_and_expired_after() {
        let t0 = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000);
        let d = Deadline::after(t0, Duration::from_secs(10));
        assert!(!d.is_expired(t0));
        assert!(!d.is_expired(t0 + Duration::from_secs(9)));
        assert!(d.is_expired(t0 + Duration::from_secs(10)));
        assert!(d.is_expired(t0 + Duration::from_secs(11)));
    }
}
