//! Error taxonomy for the Agent Protocol
//!
//! All errors returned across the Protocol boundary are structured variants
//! of the `ProtocolError` enum. No untyped strings are permitted.

use crate::types::{Action, AgentId, Capability, RunId};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;

/// Resource types that can be exhausted
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResourceKind {
    LlmConcurrency,
    ToolCallRate,
    ContextBudget,
    ComputeQuota,
}

/// Reasons a handle might be invalid
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HandleInvalidReason {
    Expired,
    Revoked,
    Unrecognised,
}

/// Human decision for interrupt confirmation
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HumanDecision {
    Approve,
    ApproveWithModification { modified_action: Action },
    Reject { reason: String },
}

/// Opaque token for interrupt operations
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct InterruptToken(pub String);

/// All errors returned across the Protocol boundary
#[non_exhaustive]
#[derive(Error, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ProtocolError {
    /// Capability check failed; action not permitted
    #[error("PolicyViolation: action not permitted")]
    PolicyViolation {
        action: Action,
        missing_cap: Capability,
        agent_id: AgentId,
    },

    /// Scheduler budget exhausted; retry after backoff
    #[error("ResourceExhausted: {resource:?}, retry after: {retry_after:?}")]
    ResourceExhausted {
        resource: ResourceKind,
        retry_after: Option<Duration>,
    },

    /// HITL confirmation pending; call confirm() to resume
    #[error("Interrupted: confirmation required")]
    Interrupted {
        token: InterruptToken,
        rejected: bool,
    },

    /// Action exceeded the configured time limit
    #[error("Timeout: action exceeded limit of {limit:?}")]
    Timeout { action: Action, limit: Duration },

    /// Context window budget exceeded
    #[error("ContextOverflow: {current} tokens exceeds limit of {limit}")]
    ContextOverflow { current: u64, limit: u64 },

    /// Cancelled by an upstream cancel() call
    #[error("Cancelled: run {run_id} was cancelled")]
    Cancelled { run_id: RunId },

    /// Audit chain verification failed
    #[error("AuditIntegrityError: chain broken at seq {seq}")]
    AuditIntegrityError {
        seq: u64,
        expected: Vec<u8>,
        actual: Vec<u8>,
    },

    /// KernelHandle is expired or revoked
    #[error("InvalidHandle: {reason:?}")]
    InvalidHandle { reason: HandleInvalidReason },

    /// Protocol violation (malformed request)
    #[error("ProtocolViolation: {detail}")]
    ProtocolViolation { detail: String },
}

/// Priority ordering for errors when multiple could apply
/// Higher number = check first
pub fn error_priority(err: &ProtocolError) -> u8 {
    match err {
        ProtocolError::InvalidHandle { .. } => 4,     // Check first
        ProtocolError::PolicyViolation { .. } => 3,   // Check second
        ProtocolError::ResourceExhausted { .. } => 2, // Check third
        _ => 1,                                       // All others last
    }
}

/// Compare two errors by priority (for sorting)
pub fn compare_by_priority(a: &ProtocolError, b: &ProtocolError) -> std::cmp::Ordering {
    error_priority(b).cmp(&error_priority(a)) // Reverse for descending order
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Action, AgentId, Capability, RunId};

    #[test]
    fn error_priority_invalid_handle_highest() {
        let err = ProtocolError::InvalidHandle {
            reason: HandleInvalidReason::Expired,
        };
        assert_eq!(error_priority(&err), 4);
    }

    #[test]
    fn error_priority_policy_violation_second() {
        let err = ProtocolError::PolicyViolation {
            action: Action::ContextRead {
                key: "test".to_string(),
            },
            missing_cap: Capability::ContextAccess {
                scope: "test".to_string(),
            },
            agent_id: AgentId::new(),
        };
        assert_eq!(error_priority(&err), 3);
    }

    #[test]
    fn error_priority_resource_exhausted_third() {
        let err = ProtocolError::ResourceExhausted {
            resource: ResourceKind::ComputeQuota,
            retry_after: None,
        };
        assert_eq!(error_priority(&err), 2);
    }

    #[test]
    fn error_priority_others_lowest() {
        let err = ProtocolError::Cancelled {
            run_id: RunId::new(),
        };
        assert_eq!(error_priority(&err), 1);
    }

    #[test]
    fn compare_by_priority_sorts_correctly() {
        let invalid_handle = ProtocolError::InvalidHandle {
            reason: HandleInvalidReason::Revoked,
        };
        let policy_violation = ProtocolError::PolicyViolation {
            action: Action::ContextRead {
                key: "test".to_string(),
            },
            missing_cap: Capability::ContextAccess {
                scope: "test".to_string(),
            },
            agent_id: AgentId::new(),
        };
        let cancelled = ProtocolError::Cancelled {
            run_id: RunId::new(),
        };

        let mut errors = vec![&cancelled, &invalid_handle, &policy_violation];
        errors.sort_by(|a, b| compare_by_priority(a, b));

        assert!(matches!(errors[0], ProtocolError::InvalidHandle { .. }));
        assert!(matches!(errors[1], ProtocolError::PolicyViolation { .. }));
        assert!(matches!(errors[2], ProtocolError::Cancelled { .. }));
    }

    #[test]
    fn protocol_error_all_variants_display() {
        // Test that all error variants can be formatted
        let errors = vec![
            ProtocolError::InvalidHandle {
                reason: HandleInvalidReason::Expired,
            },
            ProtocolError::PolicyViolation {
                action: Action::ContextRead {
                    key: "test".to_string(),
                },
                missing_cap: Capability::ContextAccess {
                    scope: "test".to_string(),
                },
                agent_id: AgentId::new(),
            },
            ProtocolError::ResourceExhausted {
                resource: ResourceKind::LlmConcurrency,
                retry_after: None,
            },
            ProtocolError::Interrupted {
                token: InterruptToken("test".to_string()),
                rejected: false,
            },
            ProtocolError::Timeout {
                action: Action::ContextRead {
                    key: "test".to_string(),
                },
                limit: Duration::from_secs(30),
            },
            ProtocolError::ContextOverflow {
                current: 100,
                limit: 50,
            },
            ProtocolError::Cancelled {
                run_id: RunId::new(),
            },
            ProtocolError::AuditIntegrityError {
                seq: 1,
                expected: vec![1, 2, 3],
                actual: vec![4, 5, 6],
            },
            ProtocolError::ProtocolViolation {
                detail: "test".to_string(),
            },
        ];

        for err in errors {
            let _ = format!("{}", err);
        }
    }

    #[test]
    fn resource_kind_variants_exist() {
        // Verify all ResourceKind variants exist
        let _ = ResourceKind::LlmConcurrency;
        let _ = ResourceKind::ToolCallRate;
        let _ = ResourceKind::ContextBudget;
        let _ = ResourceKind::ComputeQuota;
    }

    #[test]
    fn handle_invalid_reason_variants_exist() {
        // Verify all HandleInvalidReason variants exist
        let _ = HandleInvalidReason::Expired;
        let _ = HandleInvalidReason::Revoked;
        let _ = HandleInvalidReason::Unrecognised;
    }

    #[test]
    fn human_decision_variants() {
        let approve = HumanDecision::Approve;
        let reject = HumanDecision::Reject {
            reason: "test".to_string(),
        };
        let modify = HumanDecision::ApproveWithModification {
            modified_action: Action::ContextRead {
                key: "modified".to_string(),
            },
        };

        // Just verify they can be created
        let _ = (approve, reject, modify);
    }
}
