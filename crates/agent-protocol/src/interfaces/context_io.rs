//! ContextIO interface - memory access (mmap/lseek analogy)

use crate::errors::ProtocolError;
use crate::types::AgentId;
use serde_json::Value;

/// Interface for Agent context/memory operations
///
/// Analogous to POSIX mmap/lseek operations
pub trait ContextIO {
    /// Read a value from the context store.
    fn read(
        &self,
        agent_id: AgentId,
        key: String,
    ) -> impl std::future::Future<Output = Result<Option<Value>, ProtocolError>> + Send;

    /// Write a value to the context store.
    fn write(
        &self,
        agent_id: AgentId,
        key: String,
        value: Value,
    ) -> impl std::future::Future<Output = Result<(), ProtocolError>> + Send;

    /// Search for keys matching a pattern.
    fn search(
        &self,
        agent_id: AgentId,
        pattern: String,
    ) -> impl std::future::Future<Output = Result<Vec<String>, ProtocolError>> + Send;

    /// Create a snapshot of the context.
    fn snapshot(
        &self,
        agent_id: AgentId,
    ) -> impl std::future::Future<Output = Result<Value, ProtocolError>> + Send;

    /// Restore context from a snapshot.
    fn restore(
        &self,
        agent_id: AgentId,
        snapshot: Value,
    ) -> impl std::future::Future<Output = Result<(), ProtocolError>> + Send;
}
