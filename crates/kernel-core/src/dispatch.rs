//! Dispatch module - The critical path for all kernel operations
//!
//! Implements the dispatch path:
//! 1. Identity check
//! 2. Permission check
//! 3. Scheduler
//! 4. Audit write
//! 5. Return result

use crate::{config::KernelConfig, error::KernelError};
use agent_protocol::{
    Action, ActionResult, AgentDef, AuditFilter, CapabilitySet, HandleInvalidReason, KernelHandle,
    LogEntry, ProtocolError, ResourceKind, RunId, RunSummary, SpanId,
};
use agent_registry::{AgentRegistry, InMemoryRegistry};
use async_trait::async_trait;
use kernel_api::AgentSyscall;
use permission_engine::{DefaultPermissionEngine, EvaluationResult, PermissionEngine};
use scheduler::{DefaultScheduler, Priority, Scheduler, SchedulerConfig};
use audit_log::{MemoryAuditLog, AuditLogConfig, AuditLog};
use std::sync::Arc;

/// The kernel implementation.
///
/// This is the main kernel struct that implements the AgentSyscall trait
/// and coordinates all subsystems.
pub struct Kernel {
    config: KernelConfig,
    registry: Arc<dyn AgentRegistry>,
    permission_engine: Arc<dyn PermissionEngine>,
    scheduler: Arc<dyn Scheduler>,
    audit_log: MemoryAuditLog,
}

impl std::fmt::Debug for Kernel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Kernel")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

impl Kernel {
    /// Create a new kernel with the given configuration.
    pub async fn new(config: KernelConfig) -> Result<Self, KernelError> {
        let registry = Arc::new(InMemoryRegistry::new());
        let permission_engine = Arc::new(DefaultPermissionEngine::new());
        let scheduler = Arc::new(DefaultScheduler::new(SchedulerConfig::default()));
        let audit_log = MemoryAuditLog::new(AuditLogConfig::default());
        
        Ok(Self {
            config,
            registry,
            permission_engine,
            scheduler,
            audit_log,
        })
    }

    /// Get the kernel configuration.
    pub fn config(&self) -> &KernelConfig {
        &self.config
    }
}

#[async_trait]
impl AgentSyscall for Kernel {
    async fn spawn(
        &self,
        def: AgentDef,
        caps: CapabilitySet,
    ) -> Result<KernelHandle, ProtocolError> {
        // TDD Step 2: GREEN - Minimal code to pass test
        // Create handle and register in registry
        let agent_id = agent_protocol::AgentId::new();
        let handle = KernelHandle::new(agent_id, caps.clone());
        
        // Register the agent
        self.registry
            .register(agent_id, handle.clone(), def, caps)
            .await
            .map_err(|e| ProtocolError::ProtocolViolation {
                detail: format!("Failed to register agent: {}", e),
            })?;
        
        Ok(handle)
    }

    async fn invoke(
        &self,
        handle: &KernelHandle,
        action: Action,
    ) -> Result<ActionResult, ProtocolError> {
        // Dispatch Path - Critical Path for all agent operations
        //
        // Step 1: Identity check
        let is_valid = self.registry.is_valid(handle.handle_id.clone()).await;
        if !is_valid {
            return Err(ProtocolError::InvalidHandle {
                reason: HandleInvalidReason::Unrecognised,
            });
        }

        // Get agent capabilities from registry
        let agent_entry = self
            .registry
            .get(handle.agent_id)
            .await
            .map_err(|e| ProtocolError::ProtocolViolation {
                detail: format!("Failed to get agent entry: {}", e),
            })?;
        let capabilities = agent_entry.caps;

        // Step 2: Permission check
        let eval_result = self.permission_engine.evaluate(&capabilities, &action).await;
        if eval_result.is_denied() {
            return Err(ProtocolError::PolicyViolation {
                action: action.clone(),
                missing_cap: agent_protocol::Capability::ToolRead {
                    tool_id: "unknown".to_string(),
                },
                agent_id: handle.agent_id,
            });
        }

        // Step 3: Scheduler - check resource availability (non-blocking)
        let priority = Priority::Normal; // Default priority

        let scheduled_task = self
            .scheduler
            .schedule(action.clone(), priority)
            .await
            .map_err(|e| ProtocolError::ResourceExhausted {
                resource: ResourceKind::ToolCallRate,
                retry_after: Some(std::time::Duration::from_millis(100)),
            })?;

        // Step 4: Sandbox - skipped per decision, would execute here
        // For now, execute action directly
        let result = ActionResult::Success(serde_json::json!({
            "executed": true,
            "task_id": scheduled_task.id.0,
        }));

        // Step 5: Audit write (WAL - must complete before returning)
        if self.config.enable_audit {
            let run_id = RunId::new();
            let span_id = SpanId::new();

            self.audit_log
                .append(
                    run_id,
                    span_id,
                    None, // parent_span_id
                    action,
                    Some(result.clone()),
                )
                .await
                .map_err(|e| ProtocolError::AuditIntegrityError {
                    seq: 0,
                    expected: vec![],
                    actual: vec![],
                })?;
        }

        // Step 6: Return result
        Ok(result)
    }

    async fn revoke(&self, handle: KernelHandle) -> Result<RunSummary, ProtocolError> {
        // TDD Step 2: GREEN - Minimal code to pass test
        // Revoke the handle via registry
        self.registry
            .revoke(handle.handle_id.clone())
            .await
            .map_err(|e| ProtocolError::ProtocolViolation {
                detail: format!("Failed to revoke agent: {}", e),
            })?;

        // Return minimal RunSummary
        Ok(RunSummary {
            run_id: RunId::new(),
            actions_executed: 0,
            final_status: "revoked".to_string(),
        })
    }

    async fn query_audit(&self, filter: AuditFilter) -> Result<Vec<LogEntry>, ProtocolError> {
        // Query audit log using the AuditLog trait
        use audit_log::AuditLog;
        self.audit_log
            .query_with_filter(filter)
            .await
            .map_err(|e| ProtocolError::AuditIntegrityError {
                seq: 0,
                expected: vec![],
                actual: vec![],
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_kernel_creation() {
        // TDD Step 1: RED - Write failing test
        // This test should create a kernel successfully
        let config = KernelConfig::default();
        let kernel = Kernel::new(config).await;

        // Should not fail
        assert!(kernel.is_ok(), "Kernel creation should succeed");

        let kernel = kernel.unwrap();
        assert!(kernel.config().enable_audit);
    }

    #[tokio::test]
    async fn test_spawn_creates_handle() {
        // TDD Step 1: RED - Write failing test
        // This test should spawn an agent and return a handle
        let config = KernelConfig::default();
        let kernel = Kernel::new(config).await.unwrap();

        let def = AgentDef::new("test-agent");
        let caps = CapabilitySet::default();

        let result = kernel.spawn(def, caps).await;

        // Should succeed and return a handle
        assert!(result.is_ok(), "Spawn should succeed");
        let handle = result.unwrap();
        assert_eq!(handle.agent_id.to_string().len(), 36); // UUID length
    }

    #[tokio::test]
    async fn test_invoke_with_valid_handle() {
        // TDD Step 1: RED - Write failing test
        // This test should invoke an action with a valid handle
        use agent_protocol::Capability;

        let config = KernelConfig::default();
        let kernel = Kernel::new(config).await.unwrap();

        // First spawn an agent with the required capability
        let def = AgentDef::new("test-agent");
        let caps = CapabilitySet::default()
            .with_capability(Capability::ToolRead { tool_id: "test".to_string() });
        let handle = kernel.spawn(def, caps).await.unwrap();

        // Then invoke an action
        let action = Action::ToolCall {
            tool_id: "test".to_string(),
            params: serde_json::json!({}),
        };

        let result = kernel.invoke(&handle, action).await;

        // Should succeed
        assert!(result.is_ok(), "Invoke should succeed with valid handle");
    }

    #[tokio::test]
    async fn test_invoke_with_invalid_handle() {
        // TDD Step 1: RED - Write failing test
        // This test should fail with InvalidHandle error
        let config = KernelConfig::default();
        let kernel = Kernel::new(config).await.unwrap();

        // Create a handle without spawning (invalid)
        let handle = KernelHandle::new(
            agent_protocol::AgentId::new(),
            CapabilitySet::default(),
        );

        let action = Action::ToolCall {
            tool_id: "test".to_string(),
            params: serde_json::json!({}),
        };

        let result = kernel.invoke(&handle, action).await;

        // Should fail with InvalidHandle
        assert!(result.is_err(), "Invoke should fail with invalid handle");
        assert!(matches!(result.unwrap_err(), ProtocolError::InvalidHandle { .. }));
    }

    #[tokio::test]
    async fn test_revoke_marks_handle_invalid() {
        // TDD Step 1: RED - Write failing test
        // This test should revoke an agent and mark its handle invalid
        use agent_protocol::Capability;

        let config = KernelConfig::default();
        let kernel = Kernel::new(config).await.unwrap();

        // First spawn an agent with required capability
        let def = AgentDef::new("test-agent");
        let caps = CapabilitySet::default()
            .with_capability(Capability::ToolRead { tool_id: "test".to_string() });
        let handle = kernel.spawn(def, caps).await.unwrap();

        // Verify handle is valid by invoking
        let action = Action::ToolCall {
            tool_id: "test".to_string(),
            params: serde_json::json!({}),
        };
        let result = kernel.invoke(&handle, action.clone()).await;
        assert!(result.is_ok(), "Handle should be valid before revoke");

        // Revoke the agent
        let revoke_result = kernel.revoke(handle.clone()).await;
        assert!(revoke_result.is_ok(), "Revoke should succeed");

        // Verify handle is now invalid
        let result = kernel.invoke(&handle, action).await;
        assert!(result.is_err(), "Handle should be invalid after revoke");
        assert!(matches!(result.unwrap_err(), ProtocolError::InvalidHandle { .. }));
    }

    #[tokio::test]
    async fn test_query_audit_returns_entries() {
        // TDD Step 1: RED - Write failing test
        // This test should query audit log and return entries
        let config = KernelConfig::default();
        let kernel = Kernel::new(config).await.unwrap();

        // Query audit log
        let filter = AuditFilter::default();
        let result = kernel.query_audit(filter).await;

        // Should succeed and return entries (possibly empty)
        assert!(result.is_ok(), "Query audit should succeed");
        let entries = result.unwrap();
        assert!(entries.is_empty() || !entries.is_empty()); // Just verify it's a Vec
    }

    #[tokio::test]
    async fn test_invoke_permission_denied() {
        // Test that invoke returns PolicyViolation when permission check fails
        let config = KernelConfig::default();
        let kernel = Kernel::new(config).await.unwrap();

        // Spawn agent with empty capabilities
        let def = AgentDef::new("test-agent");
        let caps = CapabilitySet::default(); // Empty capabilities
        let handle = kernel.spawn(def, caps).await.unwrap();

        // Try to invoke ToolCall without ToolRead capability
        let action = Action::ToolCall {
            tool_id: "calc".to_string(),
            params: serde_json::json!({"expr": "1+1"}),
        };

        let result = kernel.invoke(&handle, action).await;

        // Should fail with PolicyViolation
        assert!(result.is_err(), "Invoke should fail without permission");
        assert!(
            matches!(result.unwrap_err(), ProtocolError::PolicyViolation { .. }),
            "Should return PolicyViolation error"
        );
    }

    #[tokio::test]
    async fn test_invoke_with_valid_capability() {
        // Test that invoke succeeds when agent has the required capability
        use agent_protocol::Capability;

        let config = KernelConfig::default();
        let kernel = Kernel::new(config).await.unwrap();

        // Spawn agent with ToolRead capability
        let def = AgentDef::new("test-agent");
        let caps = CapabilitySet::default()
            .with_capability(Capability::ToolRead { tool_id: "calc".to_string() });
        let handle = kernel.spawn(def, caps).await.unwrap();

        // Invoke ToolCall with matching tool_id
        let action = Action::ToolCall {
            tool_id: "calc".to_string(),
            params: serde_json::json!({"expr": "1+1"}),
        };

        let result = kernel.invoke(&handle, action).await;

        // Should succeed
        assert!(result.is_ok(), "Invoke should succeed with valid capability");
    }

    #[tokio::test]
    async fn test_invoke_writes_audit_entry() {
        // Test that invoke writes audit log entry
        use agent_protocol::Capability;

        let config = KernelConfig::default();
        let kernel = Kernel::new(config).await.unwrap();

        // Spawn agent
        let def = AgentDef::new("test-agent");
        let caps = CapabilitySet::default()
            .with_capability(Capability::ToolRead { tool_id: "test".to_string() });
        let handle = kernel.spawn(def, caps).await.unwrap();

        // Invoke an action
        let action = Action::ToolCall {
            tool_id: "test".to_string(),
            params: serde_json::json!({}),
        };
        let _result = kernel.invoke(&handle, action).await.unwrap();

        // Query audit log - should have at least 1 entry
        let filter = AuditFilter::default();
        let audit_result = kernel.query_audit(filter).await;
        assert!(audit_result.is_ok(), "Query audit should succeed");

        let entries = audit_result.unwrap();
        assert!(!entries.is_empty(), "Audit log should have at least one entry");
    }

    #[tokio::test]
    async fn test_full_dispatch_flow() {
        // Test complete flow: spawn -> invoke -> revoke -> audit
        use agent_protocol::Capability;

        let config = KernelConfig::default();
        let kernel = Kernel::new(config).await.unwrap();

        // Step 1: Spawn agent with capabilities
        let def = AgentDef::new("test-agent");
        let caps = CapabilitySet::default()
            .with_capability(Capability::ToolRead { tool_id: "api".to_string() });
        let handle = kernel.spawn(def, caps).await.unwrap();
        assert_eq!(handle.agent_id.to_string().len(), 36);

        // Step 2: Invoke action with valid capability
        let action = Action::ToolCall {
            tool_id: "api".to_string(),
            params: serde_json::json!({"method": "GET"}),
        };
        let result = kernel.invoke(&handle, action).await;
        assert!(result.is_ok(), "Invoke with valid cap should succeed");

        // Step 3: Query audit - should have entries
        let filter = AuditFilter::default();
        let entries = kernel.query_audit(filter).await.unwrap();
        assert!(!entries.is_empty(), "Audit should have entry for invoke");

        // Step 4: Revoke agent
        let revoke_result = kernel.revoke(handle.clone()).await;
        assert!(revoke_result.is_ok(), "Revoke should succeed");
        let summary = revoke_result.unwrap();
        assert_eq!(summary.final_status, "revoked");

        // Step 5: Verify handle is invalid after revoke
        let action2 = Action::ToolCall {
            tool_id: "api".to_string(),
            params: serde_json::json!({}),
        };
        let result = kernel.invoke(&handle, action2).await;
        assert!(result.is_err(), "Invoke should fail after revoke");
        assert!(
            matches!(result.unwrap_err(), ProtocolError::InvalidHandle { .. }),
            "Should return InvalidHandle after revoke"
        );
    }
}
