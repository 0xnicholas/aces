//! Error taxonomy for the Agent Protocol
//!
//! All errors returned across the Protocol boundary are structured variants
//! of the `ProtocolError` enum. No untyped strings are permitted.

use std::time::Duration;
use thiserror::Error;

/// All errors returned across the Protocol boundary
#[derive(Error, Debug, Clone, PartialEq)]
pub enum ProtocolError {
    /// Capability check failed; action not permitted
    #[error("PolicyViolation: action not permitted")]
    PolicyViolation {
        action: String,
        missing_cap: String,
        agent_id: String,
    },

    /// Scheduler budget exhausted; retry after backoff
    #[error("ResourceExhausted: {resource}, retry after: {retry_after:?}")]
    ResourceExhausted {
        resource: String,
        retry_after: Option<Duration>,
    },

    /// HITL confirmation pending; call confirm() to resume
    #[error("Interrupted: confirmation required")]
    Interrupted { token: String, rejected: bool },

    /// Action exceeded the configured time limit
    #[error("Timeout: action exceeded limit of {limit:?}")]
    Timeout { action: String, limit: Duration },

    /// Context window budget exceeded
    #[error("ContextOverflow: {current} tokens exceeds limit of {limit}")]
    ContextOverflow { current: u64, limit: u64 },

    /// Cancelled by an upstream cancel() call
    #[error("Cancelled: run {run_id} was cancelled")]
    Cancelled { run_id: String },

    /// Audit chain verification failed
    #[error("AuditIntegrityError: chain broken at seq {seq}")]
    AuditIntegrityError {
        seq: u64,
        expected: Vec<u8>,
        actual: Vec<u8>,
    },

    /// KernelHandle is expired or revoked
    #[error("InvalidHandle: {reason}")]
    InvalidHandle { reason: String },

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
