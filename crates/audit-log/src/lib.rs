//! Audit Log - Write-Ahead Log with SHA-256 integrity chain
//!
//! This crate provides the audit log implementation for the Agent Kernel.
//! It maintains an append-only log with cryptographic integrity verification
//! using a SHA-256 hash chain.
//!
//! # Key Features
//!
//! - **Write-Ahead Log (WAL)**: Entries are durably written before returning
//! - **Integrity Chain**: Each entry contains the SHA-256 hash of the previous entry
//! - **Verification**: Full chain verification on every query
//! - **Immutability**: Once written, entries cannot be modified
//!
//! # Architecture
//!
//! ```text
//! Entry 1          Entry 2          Entry 3
//! ┌─────────┐     ┌─────────┐     ┌─────────┐
//! │ Seq: 1  │────▶│ Seq: 2  │────▶│ Seq: 3  │
//! │ Data    │     │ Data    │     │ Data    │
//! │ Hash: 0 │     │ Hash: H1│     │ Hash: H2│
//! └─────────┘     └─────────┘     └─────────┘
//!
//! H1 = SHA256(Entry 1)
//! H2 = SHA256(Entry 2)
//! ```
//!
//! # Usage
//!
//! ```rust,no_run
//! use audit_log::{AuditLog, MemoryAuditLog, AuditLogConfig};
//! use agent_protocol::{RunId, SpanId, Action};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create an in-memory audit log
//! let log = MemoryAuditLog::new(AuditLogConfig::default());
//!
//! // Append an entry
//! let entry = log.append(
//!     RunId::new(),
//!     SpanId::new(),
//!     None,
//!     Action::ToolCall {
//!         tool_id: "test".to_string(),
//!         params: serde_json::json!({}),
//!     },
//!     None,
//! ).await?;
//!
//! // Query entries
//! let entries = log.query(|_| true).await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Security
//!
//! The integrity chain ensures that any tampering with historical entries
//! is immediately detected. Verification is mandatory on every query.
//!
//! # License
//!
//! Apache 2.0 - See LICENSE-APACHE file for details.

use agent_protocol::{Action, ActionResult, AuditFilter, LogEntry, ProtocolError, RunId, SpanId};
use async_trait::async_trait;
use sha2::{Digest, Sha256};
use thiserror::Error;

mod memory;
mod wal;

pub use memory::MemoryAuditLog;
pub use wal::{WalAuditLog, WalConfig, WalError};

/// Configuration for the audit log.
#[derive(Debug, Clone)]
pub struct AuditLogConfig {
    /// Enable integrity verification on query (default: true)
    pub verify_on_query: bool,
    /// Maximum number of entries to keep in memory (0 = unlimited)
    pub max_memory_entries: usize,
    /// Buffer size for WAL writes
    pub write_buffer_size: usize,
}

impl Default for AuditLogConfig {
    fn default() -> Self {
        Self {
            verify_on_query: true,
            max_memory_entries: 0,
            write_buffer_size: 4096,
        }
    }
}

/// Errors specific to audit log operations.
#[derive(Error, Debug, Clone)]
pub enum AuditLogError {
    /// Integrity check failed - chain is broken
    #[error("integrity check failed at sequence {seq}: expected {expected:x?}, got {actual:x?}")]
    IntegrityCheckFailed {
        seq: u64,
        expected: Vec<u8>,
        actual: Vec<u8>,
    },

    /// Storage operation failed
    #[error("storage error: {0}")]
    StorageError(String),

    /// Sequence number mismatch
    #[error("sequence mismatch: expected {expected}, got {actual}")]
    SequenceMismatch { expected: u64, actual: u64 },

    /// Entry not found
    #[error("entry not found: sequence {0}")]
    EntryNotFound(u64),

    /// Log is closed
    #[error("audit log is closed")]
    Closed,
}

impl From<AuditLogError> for ProtocolError {
    fn from(err: AuditLogError) -> Self {
        match err {
            AuditLogError::IntegrityCheckFailed {
                seq,
                expected,
                actual,
            } => ProtocolError::AuditIntegrityError {
                seq,
                expected,
                actual,
            },
            _ => ProtocolError::ProtocolViolation {
                detail: err.to_string(),
            },
        }
    }
}

/// The audit log trait - defines the interface for all audit log implementations.
///
/// All implementations must:
/// - Maintain the integrity chain (SHA-256 hashes)
/// - Verify the chain on query (if configured)
/// - Support WAL semantics (durable writes)
#[async_trait]
pub trait AuditLog: Send + Sync {
    /// Append a new entry to the audit log.
    ///
    /// This operation:
    /// 1. Assigns the next sequence number
    /// 2. Computes the integrity hash (SHA-256 of previous entry)
    /// 3. Durably writes the entry (WAL semantics)
    /// 4. Returns the completed entry
    ///
    /// # Arguments
    ///
    /// * `run_id` - The run identifier
    /// * `span_id` - The span identifier
    /// * `parent_span_id` - Optional parent span
    /// * `action` - The action being logged
    /// * `result` - Optional result of the action
    ///
    /// # Returns
    ///
    /// Returns the completed `LogEntry` on success, or `AuditLogError` on failure.
    async fn append(
        &self,
        run_id: RunId,
        span_id: SpanId,
        parent_span_id: Option<SpanId>,
        action: Action,
        result: Option<ActionResult>,
    ) -> Result<LogEntry, AuditLogError>;

    /// Query the audit log with a filter predicate.
    ///
    /// This operation verifies the integrity chain before returning results
    /// (if `verify_on_query` is enabled in config).
    ///
    /// # Arguments
    ///
    /// * `filter` - A predicate function to filter entries
    ///
    /// # Returns
    ///
    /// Returns a vector of matching `LogEntry` items on success.
    /// Returns `AuditLogError::IntegrityCheckFailed` if verification fails.
    async fn query<F>(&self, filter: F) -> Result<Vec<LogEntry>, AuditLogError>
    where
        F: Fn(&LogEntry) -> bool + Send;

    /// Query using an AuditFilter (from agent-protocol).
    ///
    /// This is a convenience method that converts the AuditFilter into
    /// a predicate and calls `query`.
    async fn query_with_filter(&self, filter: AuditFilter) -> Result<Vec<LogEntry>, AuditLogError> {
        self.query(|entry: &LogEntry| {
            // Apply filter conditions
            if let Some(from_ts) = filter.from_timestamp {
                if entry.timestamp < from_ts {
                    return false;
                }
            }
            if let Some(to_ts) = filter.to_timestamp {
                if entry.timestamp > to_ts {
                    return false;
                }
            }
            if let Some(agent_id) = filter.agent_id {
                // Note: LogEntry doesn't have agent_id field directly
                // This would need to be stored separately or derived
                // For now, skip this filter
                let _ = agent_id;
            }
            if let Some(run_id) = filter.run_id {
                if entry.run_id != run_id {
                    return false;
                }
            }
            true
        })
        .await
    }

    /// Get the current sequence number (next entry will have seq + 1).
    async fn current_sequence(&self) -> Result<u64, AuditLogError>;

    /// Verify the integrity of the entire chain.
    ///
    /// Walks through all entries and verifies that each entry's integrity
    /// hash matches the SHA-256 hash of the previous entry.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the chain is valid.
    /// Returns `AuditLogError::IntegrityCheckFailed` if any entry is corrupted.
    async fn verify_chain(&self) -> Result<(), AuditLogError>;

    /// Get a specific entry by sequence number.
    async fn get_entry(&self, sequence: u64) -> Result<Option<LogEntry>, AuditLogError>;
}

/// Compute the SHA-256 hash of a log entry.
///
/// This hash is used as the integrity field for the next entry in the chain.
pub fn compute_entry_hash(entry: &LogEntry) -> [u8; 32] {
    let mut hasher = Sha256::new();

    // Hash all fields that constitute the entry
    hasher.update(entry.sequence.to_le_bytes());
    hasher.update(entry.timestamp.to_le_bytes());
    hasher.update(entry.run_id.to_string().as_bytes());
    hasher.update(entry.span_id.to_string().as_bytes());
    if let Some(parent) = &entry.parent_span_id {
        hasher.update(parent.to_string().as_bytes());
    }

    // Hash the action (serialized as JSON for consistency)
    let action_json = serde_json::to_vec(&entry.action).unwrap_or_default();
    hasher.update(&action_json);

    // Hash the result if present
    if let Some(result) = &entry.result {
        let result_json = serde_json::to_vec(result).unwrap_or_default();
        hasher.update(&result_json);
    }

    // Hash the integrity field itself (chain of hashes)
    hasher.update(entry.integrity);

    let result = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result);
    hash
}

/// Create the genesis hash (all zeros) for the first entry.
pub fn genesis_hash() -> [u8; 32] {
    [0u8; 32]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_action() -> Action {
        Action::ToolCall {
            tool_id: "test-tool".to_string(),
            params: serde_json::json!({}),
        }
    }

    #[test]
    fn test_genesis_hash() {
        let hash = genesis_hash();
        assert_eq!(hash, [0u8; 32]);
    }

    #[test]
    fn test_compute_entry_hash_deterministic() {
        let entry = LogEntry {
            sequence: 1,
            timestamp: 1234567890,
            run_id: RunId::new(),
            span_id: SpanId::new(),
            parent_span_id: None,
            action: test_action(),
            result: None,
            integrity: genesis_hash(),
        };

        let hash1 = compute_entry_hash(&entry);
        let hash2 = compute_entry_hash(&entry);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_compute_entry_hash_different_entries() {
        let entry1 = LogEntry {
            sequence: 1,
            timestamp: 1234567890,
            run_id: RunId::new(),
            span_id: SpanId::new(),
            parent_span_id: None,
            action: test_action(),
            result: None,
            integrity: genesis_hash(),
        };

        let entry2 = LogEntry {
            sequence: 2,
            timestamp: 1234567890,
            run_id: RunId::new(),
            span_id: SpanId::new(),
            parent_span_id: None,
            action: test_action(),
            result: None,
            integrity: genesis_hash(),
        };

        let hash1 = compute_entry_hash(&entry1);
        let hash2 = compute_entry_hash(&entry2);
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_audit_log_error_converts_to_protocol_error() {
        let audit_err = AuditLogError::IntegrityCheckFailed {
            seq: 42,
            expected: vec![1, 2, 3],
            actual: vec![4, 5, 6],
        };

        let protocol_err: ProtocolError = audit_err.into();
        match protocol_err {
            ProtocolError::AuditIntegrityError { seq, .. } => {
                assert_eq!(seq, 42);
            }
            _ => panic!("Expected AuditIntegrityError"),
        }
    }
}
