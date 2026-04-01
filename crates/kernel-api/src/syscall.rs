//! AgentSyscall trait - The core interface between Runtimes and the Kernel
//!
//! This module defines the [`AgentSyscall`] trait, which is the contract
//! that all Kernel implementations must fulfill. It provides exactly four
//! methods as specified in the architecture.

use agent_protocol::{
    Action, ActionResult, AgentDef, AuditFilter, CapabilitySet, KernelHandle, LogEntry,
    ProtocolError, RunSummary,
};
use async_trait::async_trait;

/// The core trait that all Kernel implementations must implement.
///
/// This trait defines the four public methods exposed to Runtime callers.
/// All methods are async and return [`Result`] with [`ProtocolError`] as the
/// error type.
///
/// # Implementation Notes
///
/// - All implementations must be [`Send`] + [`Sync`] to allow sharing across threads
/// - The trait uses `async_trait` for async support
/// - All methods must uphold the six semantic constraints defined in the protocol
///
/// # Example
///
/// ```rust
/// use kernel_api::AgentSyscall;
/// use agent_protocol::*;
/// use async_trait::async_trait;
///
/// struct MyKernel;
///
/// #[async_trait]
/// impl AgentSyscall for MyKernel {
///     async fn spawn(
///         &self,
///         def: AgentDef,
///         caps: CapabilitySet,
///     ) -> Result<KernelHandle, ProtocolError> {
///         // Implementation here
///         # todo!()
///     }
///
///     async fn invoke(
///         &self,
///         handle: &KernelHandle,
///         action: Action,
///     ) -> Result<ActionResult, ProtocolError> {
///         // Implementation here
///         # todo!()
///     }
///
///     async fn revoke(
///         &self,
///         handle: KernelHandle,
///     ) -> Result<RunSummary, ProtocolError> {
///         // Implementation here
///         # todo!()
///     }
///
///     async fn query_audit(
///         &self,
///         filter: AuditFilter,
///     ) -> Result<Vec<LogEntry>, ProtocolError> {
///         // Implementation here
///         # todo!()
///     }
/// }
/// ```
#[async_trait]
pub trait AgentSyscall: Send + Sync {
    /// Spawn a new Agent instance bound to the given capabilities.
    ///
    /// Capabilities are immutable for the lifetime of the returned handle.
    /// The spawned Agent receives a unique [`KernelHandle`] that must be
    /// used for all subsequent operations.
    ///
    /// # Arguments
    ///
    /// * `def` - The agent definition including name and configuration
    /// * `caps` - The capability set to grant to the new agent
    ///
    /// # Returns
    ///
    /// Returns a [`KernelHandle`] on success, or a [`ProtocolError`] if:
    /// - The agent definition is invalid
    /// - Resources are exhausted
    /// - The caller lacks permission to spawn agents
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use kernel_api::AgentSyscall;
    /// use agent_protocol::{AgentDef, CapabilitySet, Capability};
    ///
    /// # async fn example<K: AgentSyscall>(kernel: &K) -> Result<(), Box<dyn std::error::Error>> {
    /// let def = AgentDef::new("calculator-agent");
    /// let caps = CapabilitySet::default()
    ///     .with_capability(Capability::ToolRead { tool_id: "calculator".to_string() });
    ///
    /// let handle = kernel.spawn(def, caps).await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn spawn(
        &self,
        def: AgentDef,
        caps: CapabilitySet,
    ) -> Result<KernelHandle, ProtocolError>;

    /// Execute an action on behalf of the Agent identified by handle.
    ///
    /// This is the single critical path that follows:
    /// 1. Identity check - validate the handle
    /// 2. Permission check - verify capabilities
    /// 3. Scheduler - acquire resources
    /// 4. Sandbox - execute in isolation
    /// 5. Audit write - log the action (WAL)
    /// 6. Return result
    ///
    /// # Arguments
    ///
    /// * `handle` - The kernel handle of the agent
    /// * `action` - The action to execute
    ///
    /// # Returns
    ///
    /// Returns the [`ActionResult`] on success, or a [`ProtocolError`] if:
    /// - The handle is invalid or revoked
    /// - The action is not permitted by capabilities
    /// - Resources are exhausted
    /// - The action times out
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use kernel_api::AgentSyscall;
    /// use agent_protocol::{Action, KernelHandle};
    ///
    /// # async fn example<K: AgentSyscall>(kernel: &K, handle: &KernelHandle) -> Result<(), Box<dyn std::error::Error>> {
    /// let action = Action::ToolCall {
    ///     tool_id: "calculator".to_string(),
    ///     params: serde_json::json!({"expression": "1 + 2"}),
    /// };
    ///
    /// let result = kernel.invoke(handle, action).await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn invoke(
        &self,
        handle: &KernelHandle,
        action: Action,
    ) -> Result<ActionResult, ProtocolError>;

    /// Revoke an Agent.
    ///
    /// This operation:
    /// - Flushes in-flight actions
    /// - Marks the handle as invalid
    /// - Writes a Revocation entry to the audit log
    /// - Returns a summary of the agent's execution
    ///
    /// # Arguments
    ///
    /// * `handle` - The kernel handle to revoke (consumed)
    ///
    /// # Returns
    ///
    /// Returns a [`RunSummary`] on success, or a [`ProtocolError`] if:
    /// - The handle is already revoked
    /// - Flushing in-flight actions fails
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use kernel_api::AgentSyscall;
    /// use agent_protocol::KernelHandle;
    ///
    /// # async fn example<K: AgentSyscall>(kernel: &K, handle: KernelHandle) -> Result<(), Box<dyn std::error::Error>> {
    /// let summary = kernel.revoke(handle).await?;
    /// println!("Agent executed {} actions", summary.actions_executed);
    /// # Ok(())
    /// # }
    /// ```
    async fn revoke(&self, handle: KernelHandle) -> Result<RunSummary, ProtocolError>;

    /// Query the audit log.
    ///
    /// This is a read-only operation that returns immutable audit entries.
    /// The integrity chain is verified on every query.
    ///
    /// # Arguments
    ///
    /// * `filter` - Filter criteria for the query
    ///
    /// # Returns
    ///
    /// Returns a vector of [`LogEntry`] matching the filter, or a [`ProtocolError`] if:
    /// - The audit log is corrupted (integrity check fails)
    /// - The filter is invalid
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use kernel_api::AgentSyscall;
    /// use agent_protocol::AuditFilter;
    ///
    /// # async fn example<K: AgentSyscall>(kernel: &K) -> Result<(), Box<dyn std::error::Error>> {
    /// let filter = AuditFilter::default();
    /// let entries = kernel.query_audit(filter).await?;
    ///
    /// for entry in entries {
    ///     println!("Action at {}: {:?}", entry.timestamp, entry.action);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    async fn query_audit(&self, filter: AuditFilter) -> Result<Vec<LogEntry>, ProtocolError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_protocol::AgentId;

    // Test that the trait is object-safe
    #[test]
    fn test_trait_is_object_safe() {
        fn _assert_object_safe(_: &dyn AgentSyscall) {}
    }

    // Test that we can create a trait object
    #[test]
    fn test_can_create_trait_object() {
        struct DummyKernel;

        #[async_trait]
        impl AgentSyscall for DummyKernel {
            async fn spawn(
                &self,
                _def: AgentDef,
                _caps: CapabilitySet,
            ) -> Result<KernelHandle, ProtocolError> {
                Ok(KernelHandle::new(AgentId::new(), CapabilitySet::empty()))
            }

            async fn invoke(
                &self,
                _handle: &KernelHandle,
                _action: Action,
            ) -> Result<ActionResult, ProtocolError> {
                Ok(ActionResult::Success(serde_json::json!(null)))
            }

            async fn revoke(&self, _handle: KernelHandle) -> Result<RunSummary, ProtocolError> {
                Ok(RunSummary {
                    run_id: agent_protocol::RunId::new(),
                    actions_executed: 0,
                    final_status: "revoked".to_string(),
                })
            }

            async fn query_audit(
                &self,
                _filter: AuditFilter,
            ) -> Result<Vec<LogEntry>, ProtocolError> {
                Ok(vec![])
            }
        }

        let _kernel: &dyn AgentSyscall = &DummyKernel;
    }
}
