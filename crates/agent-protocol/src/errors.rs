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
