//! Priority levels for request scheduling.
//!
//! Defines the 4-level priority system from ADR-006.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Request priority levels.
///
/// Lower numeric values indicate higher priority.
/// Critical (0) is the highest, Low (3) is the lowest.
///
/// # Priority Semantics
///
/// | Priority | Value | Use Case | Example |
/// |----------|-------|----------|---------|
/// | Critical | 0 | User-facing, blocking | Chat response, UI interaction |
/// | High | 1 | Important business logic | Workflow step, data sync |
/// | Normal | 2 | Standard operations | Background task, batch job |
/// | Low | 3 | Best-effort, deferrable | Analytics, cleanup, indexing |
///
/// # Example
///
/// ```rust
/// use scheduler::Priority;
///
/// let priority = Priority::Critical;
/// assert_eq!(priority as u8, 0);
///
/// // Default is Normal
/// let default_priority: Priority = Default::default();
/// assert_eq!(default_priority, Priority::Normal);
/// ```
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
#[repr(u8)]
pub enum Priority {
    /// User-facing, blocking operations
    Critical = 0,
    /// Important business logic
    High = 1,
    /// Standard operations (default)
    #[default]
    Normal = 2,
    /// Best-effort, deferrable work
    Low = 3,
}

impl fmt::Display for Priority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Priority::Critical => write!(f, "critical"),
            Priority::High => write!(f, "high"),
            Priority::Normal => write!(f, "normal"),
            Priority::Low => write!(f, "low"),
        }
    }
}

impl Priority {
    /// Returns all priority variants in order from highest to lowest.
    pub fn all() -> [Self; 4] {
        [Self::Critical, Self::High, Self::Normal, Self::Low]
    }

    /// Returns the priority level as a u8.
    pub fn as_u8(&self) -> u8 {
        *self as u8
    }

    /// Returns true if this is the highest priority (Critical).
    pub fn is_critical(&self) -> bool {
        matches!(self, Self::Critical)
    }

    /// Returns true if this is the lowest priority (Low).
    pub fn is_low(&self) -> bool {
        matches!(self, Self::Low)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_priority_ordering() {
        assert!(Priority::Critical < Priority::High);
        assert!(Priority::High < Priority::Normal);
        assert!(Priority::Normal < Priority::Low);
    }

    #[test]
    fn test_priority_default() {
        let priority: Priority = Default::default();
        assert_eq!(priority, Priority::Normal);
    }

    #[test]
    fn test_priority_all() {
        let all = Priority::all();
        assert_eq!(all.len(), 4);
        assert_eq!(all[0], Priority::Critical);
        assert_eq!(all[3], Priority::Low);
    }

    #[test]
    fn test_priority_as_u8() {
        assert_eq!(Priority::Critical.as_u8(), 0);
        assert_eq!(Priority::Normal.as_u8(), 2);
        assert_eq!(Priority::Low.as_u8(), 3);
    }

    #[test]
    fn test_priority_display() {
        assert_eq!(Priority::Critical.to_string(), "critical");
        assert_eq!(Priority::Normal.to_string(), "normal");
    }

    #[test]
    fn test_priority_is_critical() {
        assert!(Priority::Critical.is_critical());
        assert!(!Priority::High.is_critical());
    }

    #[test]
    fn test_priority_is_low() {
        assert!(Priority::Low.is_low());
        assert!(!Priority::Normal.is_low());
    }
}
