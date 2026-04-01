//! In-memory audit log implementation.
//!
//! This implementation stores all entries in memory. It's useful for testing
//! and scenarios where durability is not required.

use crate::{compute_entry_hash, genesis_hash, AuditLog, AuditLogConfig, AuditLogError};
use agent_protocol::{Action, ActionResult, LogEntry, RunId, SpanId};
use async_trait::async_trait;
use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

/// An in-memory implementation of the audit log.
///
/// All entries are stored in a BTreeMap keyed by sequence number.
/// This provides O(log n) lookup and maintains order.
///
/// # Example
///
/// ```rust
/// use audit_log::{MemoryAuditLog, AuditLog, AuditLogConfig};
/// use agent_protocol::{RunId, SpanId, Action};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let log = MemoryAuditLog::new(AuditLogConfig::default());
///
/// let entry = log.append(
///     RunId::new(),
///     SpanId::new(),
///     None,
///     Action::ToolCall {
///         tool_id: "test".to_string(),
///         params: serde_json::json!({}),
///     },
///     None,
/// ).await?;
///
/// assert_eq!(entry.sequence, 1);
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct MemoryAuditLog {
    state: Arc<RwLock<MemoryState>>,
    config: AuditLogConfig,
}

#[derive(Debug)]
struct MemoryState {
    entries: BTreeMap<u64, LogEntry>,
    next_sequence: u64,
}

impl MemoryAuditLog {
    /// Create a new in-memory audit log.
    pub fn new(config: AuditLogConfig) -> Self {
        Self {
            state: Arc::new(RwLock::new(MemoryState {
                entries: BTreeMap::new(),
                next_sequence: 1,
            })),
            config,
        }
    }

    /// Get the number of entries in the log.
    pub fn len(&self) -> usize {
        self.state.read().unwrap().entries.len()
    }

    /// Check if the log is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get all entries as a vector (in sequence order).
    pub fn get_all_entries(&self) -> Vec<LogEntry> {
        self.state
            .read()
            .unwrap()
            .entries
            .values()
            .cloned()
            .collect()
    }
}

#[async_trait]
impl AuditLog for MemoryAuditLog {
    async fn append(
        &self,
        run_id: RunId,
        span_id: SpanId,
        parent_span_id: Option<SpanId>,
        action: Action,
        result: Option<ActionResult>,
    ) -> Result<LogEntry, AuditLogError> {
        let mut state = self.state.write().unwrap();
        let sequence = state.next_sequence;
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Compute integrity hash
        let integrity = if sequence == 1 {
            genesis_hash()
        } else {
            // Get the previous entry and compute its hash
            let prev_entry =
                state
                    .entries
                    .get(&(sequence - 1))
                    .ok_or(AuditLogError::SequenceMismatch {
                        expected: sequence - 1,
                        actual: 0,
                    })?;
            compute_entry_hash(prev_entry)
        };

        let entry = LogEntry {
            sequence,
            timestamp,
            run_id,
            span_id,
            parent_span_id,
            action,
            result,
            integrity,
        };

        // Store the entry
        state.entries.insert(sequence, entry.clone());
        state.next_sequence += 1;

        // Check memory limit
        if self.config.max_memory_entries > 0
            && state.entries.len() > self.config.max_memory_entries
        {
            // Remove oldest entries (lowest sequence numbers)
            let to_remove: Vec<u64> = state
                .entries
                .keys()
                .take(state.entries.len() - self.config.max_memory_entries)
                .cloned()
                .collect();
            for seq in to_remove {
                state.entries.remove(&seq);
            }
        }

        Ok(entry)
    }

    async fn query<F>(&self, filter: F) -> Result<Vec<LogEntry>, AuditLogError>
    where
        F: Fn(&LogEntry) -> bool + Send,
    {
        // Verify chain if enabled
        if self.config.verify_on_query {
            self.verify_chain().await?;
        }

        let state = self.state.read().unwrap();
        let entries: Vec<LogEntry> = state
            .entries
            .values()
            .filter(|e| filter(e))
            .cloned()
            .collect();

        Ok(entries)
    }

    async fn current_sequence(&self) -> Result<u64, AuditLogError> {
        let state = self.state.read().unwrap();
        Ok(state.next_sequence)
    }

    async fn verify_chain(&self) -> Result<(), AuditLogError> {
        let state = self.state.read().unwrap();

        if state.entries.is_empty() {
            return Ok(());
        }

        // Get entries in order
        let sequences: Vec<u64> = state.entries.keys().cloned().collect();

        for (i, seq) in sequences.iter().enumerate() {
            let entry = state.entries.get(seq).unwrap();

            // First entry should have genesis hash
            if *seq == 1 {
                if entry.integrity != genesis_hash() {
                    return Err(AuditLogError::IntegrityCheckFailed {
                        seq: *seq,
                        expected: genesis_hash().to_vec(),
                        actual: entry.integrity.to_vec(),
                    });
                }
            } else {
                // Verify against previous entry
                let prev_seq = seq - 1;
                let prev_entry =
                    state
                        .entries
                        .get(&prev_seq)
                        .ok_or(AuditLogError::SequenceMismatch {
                            expected: prev_seq,
                            actual: 0,
                        })?;

                let expected_hash = compute_entry_hash(prev_entry);
                if entry.integrity != expected_hash {
                    return Err(AuditLogError::IntegrityCheckFailed {
                        seq: *seq,
                        expected: expected_hash.to_vec(),
                        actual: entry.integrity.to_vec(),
                    });
                }
            }

            // Verify sequence is contiguous
            if i > 0 && sequences[i - 1] != seq - 1 {
                return Err(AuditLogError::SequenceMismatch {
                    expected: seq - 1,
                    actual: sequences[i - 1],
                });
            }
        }

        Ok(())
    }

    async fn get_entry(&self, sequence: u64) -> Result<Option<LogEntry>, AuditLogError> {
        let state = self.state.read().unwrap();
        Ok(state.entries.get(&sequence).cloned())
    }
}

impl Clone for MemoryAuditLog {
    fn clone(&self) -> Self {
        Self {
            state: Arc::clone(&self.state),
            config: self.config.clone(),
        }
    }
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

    #[tokio::test]
    async fn test_memory_audit_log_append() {
        let log = MemoryAuditLog::new(AuditLogConfig::default());

        let entry = log
            .append(RunId::new(), SpanId::new(), None, test_action(), None)
            .await
            .unwrap();

        assert_eq!(entry.sequence, 1);
        assert_eq!(entry.integrity, genesis_hash());
    }

    #[tokio::test]
    async fn test_memory_audit_log_multiple_entries() {
        let log = MemoryAuditLog::new(AuditLogConfig::default());

        let entry1 = log
            .append(RunId::new(), SpanId::new(), None, test_action(), None)
            .await
            .unwrap();
        let entry2 = log
            .append(RunId::new(), SpanId::new(), None, test_action(), None)
            .await
            .unwrap();

        assert_eq!(entry1.sequence, 1);
        assert_eq!(entry2.sequence, 2);
        assert_ne!(entry1.integrity, entry2.integrity);
    }

    #[tokio::test]
    async fn test_memory_audit_log_query() {
        let log = MemoryAuditLog::new(AuditLogConfig::default());

        log.append(RunId::new(), SpanId::new(), None, test_action(), None)
            .await
            .unwrap();
        log.append(RunId::new(), SpanId::new(), None, test_action(), None)
            .await
            .unwrap();

        let entries = log.query(|_| true).await.unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[tokio::test]
    async fn test_memory_audit_log_verify_chain() {
        let log = MemoryAuditLog::new(AuditLogConfig::default());

        log.append(RunId::new(), SpanId::new(), None, test_action(), None)
            .await
            .unwrap();
        log.append(RunId::new(), SpanId::new(), None, test_action(), None)
            .await
            .unwrap();

        log.verify_chain().await.unwrap();
    }

    #[tokio::test]
    async fn test_memory_audit_log_get_entry() {
        let log = MemoryAuditLog::new(AuditLogConfig::default());

        let entry = log
            .append(RunId::new(), SpanId::new(), None, test_action(), None)
            .await
            .unwrap();

        let retrieved = log.get_entry(1).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().sequence, entry.sequence);
    }

    #[tokio::test]
    async fn test_memory_audit_log_current_sequence() {
        let log = MemoryAuditLog::new(AuditLogConfig::default());

        assert_eq!(log.current_sequence().await.unwrap(), 1);

        log.append(RunId::new(), SpanId::new(), None, test_action(), None)
            .await
            .unwrap();

        assert_eq!(log.current_sequence().await.unwrap(), 2);
    }

    #[tokio::test]
    async fn test_memory_audit_log_chain_integrity() {
        let log = MemoryAuditLog::new(AuditLogConfig::default());

        // Add entries
        let entry1 = log
            .append(RunId::new(), SpanId::new(), None, test_action(), None)
            .await
            .unwrap();
        let _entry2 = log
            .append(RunId::new(), SpanId::new(), None, test_action(), None)
            .await
            .unwrap();

        // Verify second entry's integrity is hash of first
        let hash1 = compute_entry_hash(&entry1);
        let retrieved = log.get_entry(2).await.unwrap().unwrap();
        assert_eq!(retrieved.integrity, hash1);
    }

    #[tokio::test]
    async fn test_memory_audit_log_memory_limit() {
        let config = AuditLogConfig {
            max_memory_entries: 2,
            ..Default::default()
        };
        let log = MemoryAuditLog::new(config);

        log.append(RunId::new(), SpanId::new(), None, test_action(), None)
            .await
            .unwrap();
        log.append(RunId::new(), SpanId::new(), None, test_action(), None)
            .await
            .unwrap();
        log.append(RunId::new(), SpanId::new(), None, test_action(), None)
            .await
            .unwrap();

        // Should only have 2 entries (the most recent)
        assert_eq!(log.len(), 2);

        // Verify chain is broken due to missing entry
        let result = log.verify_chain().await;
        assert!(result.is_err());
    }
}
