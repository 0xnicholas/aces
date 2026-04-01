//! Policy definitions for permission evaluation.
//!
//! This module provides the `Policy` trait and various implementations
//! for evaluating whether actions are permitted.

use agent_protocol::{Action, Capability, CapabilitySet};
use thiserror::Error;

/// Errors that can occur during policy evaluation.
#[derive(Error, Debug, Clone, PartialEq)]
pub enum PolicyError {
    /// Action is not permitted by the capability set
    #[error("action not permitted: missing capability {0:?}")]
    NotPermitted(Capability),

    /// Capability amplification detected
    #[error("capability amplification detected: attempted to exceed parent capabilities")]
    Amplification,

    /// Unknown action type
    #[error("unknown action type: {0}")]
    UnknownAction(String),

    /// Policy violation with custom message
    #[error("policy violation: {0}")]
    Violation(String),
}

/// The policy trait for evaluating permissions.
///
/// Implementations of this trait define different permission policies
/// that can be used with the permission engine.
///
/// # Example
///
/// ```rust
/// use permission_engine::{Policy, PolicyError, DefaultPolicy};
/// use agent_protocol::{CapabilitySet, Capability, Action};
///
/// let policy = DefaultPolicy::new();
///
/// let caps = CapabilitySet::default()
///     .with_capability(Capability::ToolRead { tool_id: "calc".to_string() });
///
/// let action = Action::ToolCall {
///     tool_id: "calc".to_string(),
///     params: serde_json::json!({}),
/// };
///
/// assert!(policy.evaluate(&caps, &action).is_ok());
/// ```
pub trait Policy: Send + Sync {
    /// Evaluate whether an action is permitted.
    ///
    /// # Arguments
    ///
    /// * `caps` - The capability set to evaluate against
    /// * `action` - The action to evaluate
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the action is permitted.
    /// Returns `Err(PolicyError)` if the action is denied.
    fn evaluate(&self, caps: &CapabilitySet, action: &Action) -> Result<(), PolicyError>;
}

/// The default policy implementation.
///
/// This policy enforces the standard capability-based security model:
/// - Tool calls require ToolRead capability for that tool
/// - Agent calls require AgentInvoke capability for that agent
/// - Context operations require ContextAccess capability for that scope
#[derive(Debug, Clone, Default)]
pub struct DefaultPolicy;

impl DefaultPolicy {
    /// Create a new default policy.
    pub fn new() -> Self {
        Self
    }
}

impl Policy for DefaultPolicy {
    fn evaluate(&self, caps: &CapabilitySet, action: &Action) -> Result<(), PolicyError> {
        let required = match action {
            Action::ToolCall { tool_id, .. } => Capability::ToolRead {
                tool_id: tool_id.clone(),
            },
            Action::CallAgent { target_id, .. } => Capability::AgentInvoke {
                agent_id: *target_id,
            },
            Action::ContextRead { key } => {
                let scope = key.split('/').next().unwrap_or("default").to_string();
                Capability::ContextAccess { scope }
            }
            Action::ContextWrite { key, .. } => {
                let scope = key.split('/').next().unwrap_or("default").to_string();
                Capability::ContextAccess { scope }
            }
            _ => {
                return Err(PolicyError::UnknownAction(
                    "unsupported action variant".to_string(),
                ))
            }
        };

        if caps.contains(&required) {
            Ok(())
        } else {
            Err(PolicyError::NotPermitted(required))
        }
    }
}

/// A policy that permits all actions.
///
/// This is useful for testing or scenarios where no permission
/// checking is desired.
///
/// # Warning
///
/// Do not use this policy in production as it bypasses all security checks.
#[derive(Debug, Clone, Default)]
pub struct PermitAllPolicy;

impl PermitAllPolicy {
    /// Create a new permit-all policy.
    pub fn new() -> Self {
        Self
    }
}

impl Policy for PermitAllPolicy {
    fn evaluate(&self, _caps: &CapabilitySet, _action: &Action) -> Result<(), PolicyError> {
        Ok(())
    }
}

/// A policy that denies all actions.
///
/// This is useful for scenarios where you want to explicitly
/// deny all operations by default.
#[derive(Debug, Clone, Default)]
pub struct DenyAllPolicy;

impl DenyAllPolicy {
    /// Create a new deny-all policy.
    pub fn new() -> Self {
        Self
    }
}

impl Policy for DenyAllPolicy {
    fn evaluate(&self, _caps: &CapabilitySet, _action: &Action) -> Result<(), PolicyError> {
        Err(PolicyError::Violation(
            "all actions denied by policy".to_string(),
        ))
    }
}

/// A policy that wraps another policy and logs all evaluations.
///
/// This is useful for debugging and auditing permission checks.
pub struct LoggingPolicy<P: Policy> {
    inner: P,
}

impl<P: Policy> LoggingPolicy<P> {
    /// Create a new logging policy wrapping the given policy.
    pub fn new(inner: P) -> Self {
        Self { inner }
    }
}

impl<P: Policy> Policy for LoggingPolicy<P> {
    fn evaluate(&self, caps: &CapabilitySet, action: &Action) -> Result<(), PolicyError> {
        let result = self.inner.evaluate(caps, action);

        match &result {
            Ok(()) => {
                tracing::info!("Permission granted: action={:?}, caps={:?}", action, caps);
            }
            Err(e) => {
                tracing::warn!(
                    "Permission denied: action={:?}, caps={:?}, reason={}",
                    action,
                    caps,
                    e
                );
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_protocol::AgentId;

    #[test]
    fn test_default_policy_permits_tool_call() {
        let policy = DefaultPolicy::new();

        let caps = CapabilitySet::default().with_capability(Capability::ToolRead {
            tool_id: "calc".to_string(),
        });

        let action = Action::ToolCall {
            tool_id: "calc".to_string(),
            params: serde_json::json!({}),
        };

        assert!(policy.evaluate(&caps, &action).is_ok());
    }

    #[test]
    fn test_default_policy_denies_tool_call() {
        let policy = DefaultPolicy::new();

        let caps = CapabilitySet::default().with_capability(Capability::ToolRead {
            tool_id: "calc".to_string(),
        });

        let action = Action::ToolCall {
            tool_id: "other".to_string(),
            params: serde_json::json!({}),
        };

        let result = policy.evaluate(&caps, &action);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PolicyError::NotPermitted(..)));
    }

    #[test]
    fn test_default_policy_permits_agent_call() {
        let policy = DefaultPolicy::new();
        let agent_id = AgentId::new();

        let caps = CapabilitySet::default().with_capability(Capability::AgentInvoke { agent_id });

        let action = Action::CallAgent {
            target_id: agent_id,
            action: Box::new(Action::ToolCall {
                tool_id: "test".to_string(),
                params: serde_json::json!({}),
            }),
        };

        assert!(policy.evaluate(&caps, &action).is_ok());
    }

    #[test]
    fn test_permit_all_policy() {
        let policy = PermitAllPolicy::new();

        let caps = CapabilitySet::default();

        let action = Action::ToolCall {
            tool_id: "any".to_string(),
            params: serde_json::json!({}),
        };

        assert!(policy.evaluate(&caps, &action).is_ok());
    }

    #[test]
    fn test_deny_all_policy() {
        let policy = DenyAllPolicy::new();

        let caps = CapabilitySet::default().with_capability(Capability::ToolRead {
            tool_id: "calc".to_string(),
        });

        let action = Action::ToolCall {
            tool_id: "calc".to_string(),
            params: serde_json::json!({}),
        };

        let result = policy.evaluate(&caps, &action);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PolicyError::Violation(..)));
    }

    #[test]
    fn test_policy_error_display() {
        let err = PolicyError::NotPermitted(Capability::ToolRead {
            tool_id: "test".to_string(),
        });
        assert!(err.to_string().contains("ToolRead"));

        let err = PolicyError::Amplification;
        assert!(err.to_string().contains("amplification"));

        let err = PolicyError::UnknownAction("custom".to_string());
        assert!(err.to_string().contains("custom"));

        let err = PolicyError::Violation("test".to_string());
        assert!(err.to_string().contains("test"));
    }

    #[test]
    fn test_logging_policy() {
        let inner = DefaultPolicy::new();
        let policy = LoggingPolicy::new(inner);

        let caps = CapabilitySet::default().with_capability(Capability::ToolRead {
            tool_id: "calc".to_string(),
        });

        let action = Action::ToolCall {
            tool_id: "calc".to_string(),
            params: serde_json::json!({}),
        };

        // Should permit the action (and log it)
        assert!(policy.evaluate(&caps, &action).is_ok());

        // Should deny unauthorized actions (and log it)
        let denied_action = Action::ToolCall {
            tool_id: "other".to_string(),
            params: serde_json::json!({}),
        };
        assert!(policy.evaluate(&caps, &denied_action).is_err());
    }
}
