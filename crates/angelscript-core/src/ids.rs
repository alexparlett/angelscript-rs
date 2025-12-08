//! Identifier types for script compilation units.
//!
//! This module provides identifiers used to track the origin of script-defined
//! types and functions during compilation and at runtime.

use std::fmt;

/// Identifies a script compilation unit.
///
/// A compilation unit represents a single script file or module being compiled.
/// This is used to track where script-defined types and functions originate from,
/// enabling proper scoping and error reporting.
///
/// # Example
///
/// ```
/// use angelscript_core::UnitId;
///
/// let unit = UnitId::new(0);
/// assert_eq!(unit.index(), 0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UnitId(u32);

impl UnitId {
    /// Create a new unit ID with the given index.
    #[inline]
    pub const fn new(index: u32) -> Self {
        Self(index)
    }

    /// Get the underlying index.
    #[inline]
    pub const fn index(self) -> u32 {
        self.0
    }
}

impl fmt::Display for UnitId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unit_{}", self.0)
    }
}

impl From<u32> for UnitId {
    fn from(index: u32) -> Self {
        Self::new(index)
    }
}

impl From<UnitId> for u32 {
    fn from(id: UnitId) -> Self {
        id.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_id_creation() {
        let unit = UnitId::new(42);
        assert_eq!(unit.index(), 42);
    }

    #[test]
    fn unit_id_display() {
        let unit = UnitId::new(5);
        assert_eq!(format!("{}", unit), "unit_5");
    }

    #[test]
    fn unit_id_equality() {
        let a = UnitId::new(1);
        let b = UnitId::new(1);
        let c = UnitId::new(2);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn unit_id_from_u32() {
        let unit: UnitId = 10.into();
        assert_eq!(unit.index(), 10);
    }

    #[test]
    fn u32_from_unit_id() {
        let unit = UnitId::new(20);
        let index: u32 = unit.into();
        assert_eq!(index, 20);
    }
}
