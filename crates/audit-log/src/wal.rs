//! Write-Ahead Log (WAL) implementation.
//!
//! This module provides a file-based WAL implementation for durable
//! audit log storage. Entries are appended to a log file with
//! fsync guarantees.
//!
//! # File Format
//!
//! The WAL file uses a simple binary format:
/// ```text
/// [Entry Length: 8 bytes][Entry Data: variable][CRC32: 4 bytes]
/// ```
///
/// Each entry is length-prefixed and followed by a CRC32 checksum.
use crate::{compute_entry_hash, genesis_hash, AuditLog, AuditLogConfig, AuditLogError};
use agent_protocol::{Action, ActionResult, LogEntry, RunId, SpanId};
use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt, BufReader};
use tokio::sync::Mutex;

/// Configuration for the WAL audit log.
#[derive(Debug, Clone)]
pub struct WalConfig {
    /// Base configuration
    pub audit_config: AuditLogConfig,
    /// Path to the WAL directory
    pub wal_dir: PathBuf,
    /// Maximum WAL file size before rotation (bytes)
    pub max_file_size: u64,
    /// Number of WAL files to retain
    pub retention_count: usize,
    /// Fsync after every write (default: true)
    pub sync_on_write: bool,
}

impl Default for WalConfig {
    fn default() -> Self {
        Self {
            audit_config: AuditLogConfig::default(),
            wal_dir: PathBuf::from("./audit_logs"),
            max_file_size: 100 * 1024 * 1024, // 100MB
            retention_count: 5,
            sync_on_write: true,
        }
    }
}

/// Errors specific to WAL operations.
#[derive(Debug, Clone)]
pub enum WalError {
    /// IO error
    Io(String),
    /// Serialization error
    Serialization(String),
    /// Corrupted entry
    CorruptedEntry { position: u64, reason: String },
    /// File rotation failed
    RotationFailed(String),
}

impl std::fmt::Display for WalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WalError::Io(msg) => write!(f, "WAL IO error: {}", msg),
            WalError::Serialization(msg) => write!(f, "WAL serialization error: {}", msg),
            WalError::CorruptedEntry { position, reason } => {
                write!(
                    f,
                    "WAL corrupted entry at position {}: {}",
                    position, reason
                )
            }
            WalError::RotationFailed(msg) => write!(f, "WAL rotation failed: {}", msg),
        }
    }
}

impl std::error::Error for WalError {}

/// A file-based WAL audit log implementation.
///
/// This implementation provides durable storage of audit log entries
/// with fsync guarantees. It's suitable for production use.
///
/// # Example
///
/// ```rust,no_run
/// use audit_log::{WalAuditLog, WalConfig, AuditLog};
/// use agent_protocol::{RunId, SpanId, Action};
/// use std::path::PathBuf;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let config = WalConfig {
///     wal_dir: PathBuf::from("/var/log/audit"),
///     ..Default::default()
/// };
///
/// let log = WalAuditLog::open(config).await?;
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
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct WalAuditLog {
    config: WalConfig,
    state: Arc<Mutex<WalState>>,
}

#[derive(Debug)]
struct WalState {
    current_file: File,
    current_size: u64,
    next_sequence: u64,
    entries: Vec<LogEntry>, // In-memory cache for queries
}

impl WalAuditLog {
    /// Open or create a WAL audit log at the configured path.
    pub async fn open(config: WalConfig) -> Result<Self, WalError> {
        // Create directory if it doesn't exist
        tokio::fs::create_dir_all(&config.wal_dir)
            .await
            .map_err(|e| WalError::Io(e.to_string()))?;

        let wal_file = config.wal_dir.join("current.wal");

        // Open or create the WAL file
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(&wal_file)
            .await
            .map_err(|e| WalError::Io(e.to_string()))?;

        let metadata = file
            .metadata()
            .await
            .map_err(|e| WalError::Io(e.to_string()))?;
        let current_size = metadata.len();

        // TODO: Recover existing entries from WAL file
        // For now, start fresh
        let next_sequence = 1;

        Ok(Self {
            config: config.clone(),
            state: Arc::new(Mutex::new(WalState {
                current_file: file,
                current_size,
                next_sequence,
                entries: Vec::new(),
            })),
        })
    }

    /// Write an entry to the WAL file.
    async fn write_entry(file: &mut File, entry: &LogEntry, sync: bool) -> Result<(), WalError> {
        // Serialize entry to JSON
        let data = serde_json::to_vec(entry).map_err(|e| WalError::Serialization(e.to_string()))?;

        // Write length prefix (8 bytes, big-endian)
        let len = data.len() as u64;
        file.write_all(&len.to_be_bytes())
            .await
            .map_err(|e| WalError::Io(e.to_string()))?;

        // Write data
        file.write_all(&data)
            .await
            .map_err(|e| WalError::Io(e.to_string()))?;

        // Write CRC32 checksum (4 bytes)
        let crc = crc32fast::hash(&data);
        file.write_all(&crc.to_be_bytes())
            .await
            .map_err(|e| WalError::Io(e.to_string()))?;

        // Fsync if requested
        if sync {
            file.sync_all()
                .await
                .map_err(|e| WalError::Io(e.to_string()))?;
        }

        Ok(())
    }

    /// Read all entries from the WAL file.
    #[allow(dead_code)] // Will be used for WAL recovery
    async fn read_entries(file: &mut File) -> Result<Vec<LogEntry>, WalError> {
        let mut entries = Vec::new();
        let mut position = 0u64;

        file.seek(std::io::SeekFrom::Start(0))
            .await
            .map_err(|e| WalError::Io(e.to_string()))?;

        let mut reader = BufReader::new(file);

        loop {
            // Read length prefix
            let mut len_buf = [0u8; 8];
            match reader.read_exact(&mut len_buf).await {
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(WalError::Io(e.to_string())),
            }
            let len = u64::from_be_bytes(len_buf) as usize;

            // Read data
            let mut data = vec![0u8; len];
            reader
                .read_exact(&mut data)
                .await
                .map_err(|e| WalError::Io(e.to_string()))?;

            // Read and verify CRC32
            let mut crc_buf = [0u8; 4];
            reader
                .read_exact(&mut crc_buf)
                .await
                .map_err(|e| WalError::Io(e.to_string()))?;
            let stored_crc = u32::from_be_bytes(crc_buf);
            let computed_crc = crc32fast::hash(&data);

            if stored_crc != computed_crc {
                return Err(WalError::CorruptedEntry {
                    position,
                    reason: format!(
                        "CRC mismatch: expected {}, got {}",
                        stored_crc, computed_crc
                    ),
                });
            }

            // Deserialize entry
            let entry: LogEntry = serde_json::from_slice(&data)
                .map_err(|e| WalError::Serialization(e.to_string()))?;

            entries.push(entry);
            position += 8 + len as u64 + 4;
        }

        Ok(entries)
    }
}

#[async_trait]
impl AuditLog for WalAuditLog {
    async fn append(
        &self,
        run_id: RunId,
        span_id: SpanId,
        parent_span_id: Option<SpanId>,
        action: Action,
        result: Option<ActionResult>,
    ) -> Result<LogEntry, AuditLogError> {
        use std::time::{SystemTime, UNIX_EPOCH};

        let mut state = self.state.lock().await;
        let sequence = state.next_sequence;
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Compute integrity hash
        let integrity = if sequence == 1 {
            genesis_hash()
        } else {
            let prev_entry = state
                .entries
                .last()
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

        // Write to WAL
        Self::write_entry(&mut state.current_file, &entry, self.config.sync_on_write)
            .await
            .map_err(|e| AuditLogError::StorageError(e.to_string()))?;

        // Update state
        let entry_size = serde_json::to_vec(&entry)
            .map_err(|e| AuditLogError::StorageError(e.to_string()))?
            .len() as u64;
        state.current_size += 8 + entry_size + 4;
        state.next_sequence += 1;
        state.entries.push(entry.clone());

        // Check if rotation is needed
        if state.current_size >= self.config.max_file_size {
            // TODO: Implement file rotation
            // For now, just log a warning
            tracing::warn!(
                "WAL file size exceeded limit ({} bytes), rotation not yet implemented",
                state.current_size
            );
        }

        Ok(entry)
    }

    async fn query<F>(&self, filter: F) -> Result<Vec<LogEntry>, AuditLogError>
    where
        F: Fn(&LogEntry) -> bool + Send,
    {
        // Verify chain if enabled
        if self.config.audit_config.verify_on_query {
            self.verify_chain().await?;
        }

        let state = self.state.lock().await;
        let entries: Vec<LogEntry> = state
            .entries
            .iter()
            .filter(|e| filter(e))
            .cloned()
            .collect();

        Ok(entries)
    }

    async fn current_sequence(&self) -> Result<u64, AuditLogError> {
        let state = self.state.lock().await;
        Ok(state.next_sequence)
    }

    async fn verify_chain(&self) -> Result<(), AuditLogError> {
        let state = self.state.lock().await;

        if state.entries.is_empty() {
            return Ok(());
        }

        for (i, entry) in state.entries.iter().enumerate() {
            let seq = entry.sequence;

            // First entry should have genesis hash
            if seq == 1 {
                if entry.integrity != genesis_hash() {
                    return Err(AuditLogError::IntegrityCheckFailed {
                        seq,
                        expected: genesis_hash().to_vec(),
                        actual: entry.integrity.to_vec(),
                    });
                }
            } else {
                // Verify against previous entry
                let prev_entry = &state.entries[i - 1];
                let expected_hash = compute_entry_hash(prev_entry);
                if entry.integrity != expected_hash {
                    return Err(AuditLogError::IntegrityCheckFailed {
                        seq,
                        expected: expected_hash.to_vec(),
                        actual: entry.integrity.to_vec(),
                    });
                }
            }

            // Verify sequence is contiguous
            if i > 0 {
                let expected_seq = state.entries[i - 1].sequence + 1;
                if seq != expected_seq {
                    return Err(AuditLogError::SequenceMismatch {
                        expected: expected_seq,
                        actual: seq,
                    });
                }
            }
        }

        Ok(())
    }

    async fn get_entry(&self, sequence: u64) -> Result<Option<LogEntry>, AuditLogError> {
        let state = self.state.lock().await;
        Ok(state
            .entries
            .iter()
            .find(|e| e.sequence == sequence)
            .cloned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_action() -> Action {
        Action::ToolCall {
            tool_id: "test-tool".to_string(),
            params: serde_json::json!({}),
        }
    }

    #[tokio::test]
    async fn test_wal_audit_log_open() {
        let temp_dir = TempDir::new().unwrap();
        let config = WalConfig {
            wal_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let log = WalAuditLog::open(config).await.unwrap();
        assert_eq!(log.current_sequence().await.unwrap(), 1);
    }

    #[tokio::test]
    async fn test_wal_audit_log_append() {
        let temp_dir = TempDir::new().unwrap();
        let config = WalConfig {
            wal_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let log = WalAuditLog::open(config).await.unwrap();

        let entry = log
            .append(RunId::new(), SpanId::new(), None, test_action(), None)
            .await
            .unwrap();

        assert_eq!(entry.sequence, 1);
    }

    #[tokio::test]
    async fn test_wal_audit_log_query() {
        let temp_dir = TempDir::new().unwrap();
        let config = WalConfig {
            wal_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let log = WalAuditLog::open(config).await.unwrap();

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
    async fn test_wal_audit_log_verify_chain() {
        let temp_dir = TempDir::new().unwrap();
        let config = WalConfig {
            wal_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let log = WalAuditLog::open(config).await.unwrap();

        log.append(RunId::new(), SpanId::new(), None, test_action(), None)
            .await
            .unwrap();
        log.append(RunId::new(), SpanId::new(), None, test_action(), None)
            .await
            .unwrap();

        log.verify_chain().await.unwrap();
    }
}
