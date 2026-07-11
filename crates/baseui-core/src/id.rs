//! Process-unique identifiers.
//!
//! [`Id`] values are handed out by a single monotonically increasing atomic
//! counter, making them cheap to generate and guaranteed unique for the life of
//! the process. They are used to give widgets, nodes, and other framework
//! objects a stable identity independent of their position in a tree.

use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(1);

/// A cheap, process-unique identifier.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Id(u64);

impl Id {
    /// Allocate a fresh, never-before-seen id.
    #[inline]
    pub fn next() -> Self {
        Id(COUNTER.fetch_add(1, Ordering::Relaxed))
    }

    /// The raw numeric value. Useful for hashing into external maps or for
    /// debugging; do not rely on any particular numbering scheme.
    #[inline]
    pub fn raw(self) -> u64 {
        self.0
    }
}

impl std::fmt::Debug for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Id({})", self.0)
    }
}

impl std::fmt::Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_and_increasing() {
        let a = Id::next();
        let b = Id::next();
        assert_ne!(a, b);
        assert!(b > a);
    }
}
