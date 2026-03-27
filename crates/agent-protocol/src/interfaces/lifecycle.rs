//! AgentLifecycle interface - manage Agent existence (fork/exec/kill analogy)

use crate::errors::ProtocolError;
use crate::types::{AgentDef, AgentId, CapabilitySet, KernelHandle};

/// Interface for managing Agent lifecycle operations
///
/// Analogous to POSIX fork/exec/kill operations
pub trait AgentLifecycle {
    /// Spawn a new Agent instance bound to the given capabilities.
    /// Capabilities are immutable for the lifetime of the returned handle.
    fn spawn(
        &self,
        def: AgentDef,
        caps: CapabilitySet,
    ) -> impl std::future::Future<Output = Result<KernelHandle, ProtocolError>> + Send;

    /// Suspend an Agent, pausing all its operations.
    fn suspend(
        &self,
        agent_id: AgentId,
    ) -> impl std::future::Future<Output = Result<(), ProtocolError>> + Send;

    /// Resume a suspended Agent.
    fn resume(
        &self,
        agent_id: AgentId,
    ) -> impl std::future::Future<Output = Result<(), ProtocolError>> + Send;

    /// Terminate an Agent immediately.
    fn terminate(
        &self,
        agent_id: AgentId,
    ) -> impl std::future::Future<Output = Result<(), ProtocolError>> + Send;
}
