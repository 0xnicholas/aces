//! Permission Engine Evaluator
//!
//! This module provides the core permission evaluation logic,
//! including async trait definitions and the default implementation.

use crate::policy::Policy;
use agent_protocol::{Action, CapabilitySet};
use async_trait::async_trait;

/// Result of a permission evaluation.
#[derive(Debug, Clone, PartialEq)]
pub enum EvaluationResult {
    /// Action is permitted
    Permitted,
    /// Action is denied with reason
    Denied { reason: String },
}

impl EvaluationResult {
    /// Returns true if the action is permitted.
    pub fn is_permitted(&self) -> bool {
        matches!(self, EvaluationResult::Permitted)
    }

    /// Returns true if the action is denied.
    pub fn is_denied(&self) -> bool {
        !self.is_permitted()
    }
}

/// The permission engine trait.
///
/// This trait defines the interface for evaluating whether actions
/// are permitted given a set of capabilities.
///
/// # Example
///
/// ```rust
/// use permission_engine::{PermissionEngine, DefaultPermissionEngine, EvaluationResult};
/// use agent_protocol::{CapabilitySet, Capability, Action};
///
/// # async fn example() {
/// let engine = DefaultPermissionEngine::new();
///
/// let caps = CapabilitySet::default()
///     .with_capability(Capability::ToolRead { tool_id: "calc".to_string() });
///
/// let action = Action::ToolCall {
///     tool_id: "calc".to_string(),
///     params: serde_json::json!({}),
/// };
///
/// let result = engine.evaluate(&caps, &action).await;
/// assert!(result.is_permitted());
/// # }
/// ```
#[async_trait]
pub trait PermissionEngine: Send + Sync {
    /// Evaluate whether an action is permitted given a capability set.
    ///
    /// # Arguments
    ///
    /// * `caps` - The capability set to evaluate against
    /// * `action` - The action to evaluate
    ///
    /// # Returns
    ///
    /// Returns `EvaluationResult::Permitted` if the action is allowed,
    /// or `EvaluationResult::Denied` with a reason if not.
    async fn evaluate(&self, caps: &CapabilitySet, action: &Action) -> EvaluationResult;

    /// Evaluate multiple actions in batch.
    ///
    /// This is more efficient than calling `evaluate` multiple times
    /// when checking multiple actions with the same capability set.
    ///
    /// # Arguments
    ///
    /// * `caps` - The capability set to evaluate against
    /// * `actions` - The actions to evaluate
    ///
    /// # Returns
    ///
    /// Returns a vector of `EvaluationResult` in the same order as the input actions.
    async fn evaluate_batch(
        &self,
        caps: &CapabilitySet,
        actions: &[Action],
    ) -> Vec<EvaluationResult> {
        let mut results = Vec::with_capacity(actions.len());
        for action in actions {
            results.push(self.evaluate(caps, action).await);
        }
        results
    }
}

/// The default permission engine implementation.
///
/// This implementation uses a `Policy` to evaluate actions.
/// It provides a flexible, policy-driven approach to permission evaluation.
#[derive(Debug)]
pub struct DefaultPermissionEngine<P: Policy> {
    policy: P,
}

impl DefaultPermissionEngine<crate::policy::DefaultPolicy> {
    /// Create a new default permission engine with the default policy.
    pub fn new() -> Self {
        Self {
            policy: crate::policy::DefaultPolicy::new(),
        }
    }
}

impl<P: Policy> DefaultPermissionEngine<P> {
    /// Create a new permission engine with a custom policy.
    pub fn with_policy(policy: P) -> Self {
        Self { policy }
    }

    /// Get a reference to the underlying policy.
    pub fn policy(&self) -> &P {
        &self.policy
    }
}

impl Default for DefaultPermissionEngine<crate::policy::DefaultPolicy> {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl<P: Policy> PermissionEngine for DefaultPermissionEngine<P> {
    async fn evaluate(&self, caps: &CapabilitySet, action: &Action) -> EvaluationResult {
        match self.policy.evaluate(caps, action) {
            Ok(()) => EvaluationResult::Permitted,
            Err(e) => EvaluationResult::Denied {
                reason: e.to_string(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_protocol::{Capability, CapabilitySet};

    #[tokio::test]
    async fn test_default_engine_permits_allowed_action() {
        let engine = DefaultPermissionEngine::new();

        let caps = CapabilitySet::default().with_capability(Capability::ToolRead {
            tool_id: "calc".to_string(),
        });

        let action = Action::ToolCall {
            tool_id: "calc".to_string(),
            params: serde_json::json!({}),
        };

        let result = engine.evaluate(&caps, &action).await;
        assert!(result.is_permitted());
    }

    #[tokio::test]
    async fn test_default_engine_denies_unallowed_action() {
        let engine = DefaultPermissionEngine::new();

        let caps = CapabilitySet::default().with_capability(Capability::ToolRead {
            tool_id: "calc".to_string(),
        });

        let action = Action::ToolCall {
            tool_id: "other".to_string(),
            params: serde_json::json!({}),
        };

        let result = engine.evaluate(&caps, &action).await;
        assert!(result.is_denied());
    }

    #[tokio::test]
    async fn test_batch_evaluation() {
        let engine = DefaultPermissionEngine::new();

        let caps = CapabilitySet::default().with_capability(Capability::ToolRead {
            tool_id: "calc".to_string(),
        });

        let actions = vec![
            Action::ToolCall {
                tool_id: "calc".to_string(),
                params: serde_json::json!({}),
            },
            Action::ToolCall {
                tool_id: "other".to_string(),
                params: serde_json::json!({}),
            },
        ];

        let results = engine.evaluate_batch(&caps, &actions).await;
        assert_eq!(results.len(), 2);
        assert!(results[0].is_permitted());
        assert!(results[1].is_denied());
    }
}
