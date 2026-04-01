//! Error types for kernel-core
//!
//! Defines errors that can occur during kernel operations.

use agent_protocol::ProtocolError;
use thiserror::Error;

/// Errors that can occur in kernel operations.
#[derive(Error, Debug, Clone)]
pub enum KernelError {
    /// Subsystem initialization failed
    #[error("subsystem initialization failed: {0}")]
    InitializationFailed(String),

    /// Dispatch path error
    #[error("dispatch error: {0}")]
    DispatchError(String),

    /// Registry error
    #[error("registry error: {0}")]
    RegistryError(String),

    /// Permission error
    #[error("permission error: {0}")]
    PermissionError(String),

    /// Scheduler error
    #[error("scheduler error: {0}")]
    SchedulerError(String),

    /// Audit error
    #[error("audit error: {0}")]
    AuditError(String),

    /// Internal error
    #[error("internal error: {0}")]
    Internal(String),
}

impl From<KernelError> for ProtocolError {
    fn from(err: KernelError) -> Self {
        match err {
            KernelError::PermissionError(_) => ProtocolError::PolicyViolation {
                action: agent_protocol::Action::ToolCall {
                    tool_id: "kernel".to_string(),
                    params: serde_json::json!({}),
                },
                missing_cap: agent_protocol::Capability::ToolRead {
                    tool_id: "kernel".to_string(),
                },
                agent_id: agent_protocol::AgentId::new(),
            },
            _ => ProtocolError::ProtocolViolation {
                detail: err.to_string(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = KernelError::InitializationFailed("test".to_string());
        assert!(err.to_string().contains("initialization failed"));
    }
}
