//! Error types for the scheduler.
//!
//! Defines errors that can occur during scheduling operations
//! and their conversion to ProtocolError.

use agent_protocol::{ProtocolError, ResourceKind};
use std::fmt;
use std::time::Duration;
use thiserror::Error;

use crate::priority::Priority;

/// Errors that can occur in token bucket operations.
#[derive(Error, Debug, Clone, PartialEq)]
pub enum TokenBucketError {
    /// Not enough tokens available
    #[error("insufficient tokens: need {needed}, have {available}, retry after {retry_after:?}")]
    InsufficientTokens {
        needed: u64,
        available: u64,
        retry_after: Duration,
    },

    /// Token bucket is at capacity
    #[error("bucket at capacity: {capacity}")]
    AtCapacity { capacity: u64 },

    /// Invalid configuration
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),
}

/// Errors that can occur during scheduling operations.
#[derive(Error, Debug, Clone)]
pub enum SchedulerError {
    /// Queue is full at the specified priority level
    #[error("queue full at priority {priority}, max depth: {max_depth}")]
    QueueFull {
        priority: Priority,
        max_depth: usize,
    },

    /// Rate limit exceeded for a resource
    #[error("rate limit exceeded for {resource:?}, retry after: {retry_after:?}")]
    RateLimitExceeded {
        resource: ResourceKind,
        retry_after: Duration,
    },

    /// Unknown resource type
    #[error("unknown resource type: {0:?}")]
    UnknownResource(ResourceKind),

    /// Task not found
    #[error("task not found: {0}")]
    TaskNotFound(TaskId),

    /// Scheduler is closed
    #[error("scheduler is closed")]
    Closed,

    /// Timeout waiting for resources
    #[error("timeout waiting for resources after {0:?}")]
    Timeout(Duration),

    /// Internal error
    #[error("internal error: {0}")]
    Internal(String),
}

/// Unique identifier for scheduled tasks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskId(pub uuid::Uuid);

impl TaskId {
    /// Create a new unique task ID.
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

impl Default for TaskId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for TaskId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<SchedulerError> for ProtocolError {
    fn from(err: SchedulerError) -> Self {
        match err {
            SchedulerError::QueueFull { .. } => ProtocolError::ResourceExhausted {
                resource: ResourceKind::ComputeQuota,
                retry_after: Some(Duration::from_secs(1)),
            },
            SchedulerError::RateLimitExceeded { retry_after, .. } => {
                ProtocolError::ResourceExhausted {
                    resource: ResourceKind::ToolCallRate,
                    retry_after: Some(retry_after),
                }
            }
            SchedulerError::Timeout(duration) => ProtocolError::ResourceExhausted {
                resource: ResourceKind::ComputeQuota,
                retry_after: Some(duration),
            },
            _ => ProtocolError::ProtocolViolation {
                detail: err.to_string(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_bucket_error_display() {
        let err = TokenBucketError::InsufficientTokens {
            needed: 5,
            available: 2,
            retry_after: Duration::from_millis(100),
        };
        let msg = err.to_string();
        assert!(msg.contains("insufficient tokens"));
        assert!(msg.contains("5"));
        assert!(msg.contains("2"));
    }

    #[test]
    fn test_scheduler_error_display() {
        let err = SchedulerError::QueueFull {
            priority: Priority::Normal,
            max_depth: 100,
        };
        assert!(err.to_string().contains("queue full"));
        assert!(err.to_string().contains("normal"));
    }

    #[test]
    fn test_task_id_creation() {
        let id1 = TaskId::new();
        let id2 = TaskId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_scheduler_error_conversion() {
        let scheduler_err = SchedulerError::QueueFull {
            priority: Priority::Critical,
            max_depth: 10,
        };

        let protocol_err: ProtocolError = scheduler_err.into();
        assert!(matches!(
            protocol_err,
            ProtocolError::ResourceExhausted { .. }
        ));
    }

    #[test]
    fn test_rate_limit_error_conversion() {
        let scheduler_err = SchedulerError::RateLimitExceeded {
            resource: ResourceKind::LlmConcurrency,
            retry_after: Duration::from_secs(5),
        };

        let protocol_err: ProtocolError = scheduler_err.into();
        match protocol_err {
            ProtocolError::ResourceExhausted { retry_after, .. } => {
                assert_eq!(retry_after, Some(Duration::from_secs(5)));
            }
            _ => panic!("Expected ResourceExhausted"),
        }
    }
}
