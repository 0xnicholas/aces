//! Permission Engine - Capability evaluation and policy enforcement
//!
//! This crate provides the permission evaluation logic for the Agent Kernel.
//! It enforces the capability-based security model defined in the Agent Protocol,
//! including the critical "capability non-amplification" constraint from ADR-002.
//!
//! # Key Features
//!
//! - **Action Evaluation**: Check if a capability set permits a specific action
//! - **Capability Intersection**: Compute delegated capabilities (caller ∩ target)
//! - **Policy Enforcement**: Enforce security policies on all actions
//! - **Non-amplification**: Ensure sub-agents never exceed parent capabilities
//!
//! # Architecture
//!
//! ```text
//! Agent A (capabilities: {ToolRead("calc"), ToolWrite("calc")})
//!         │
//!         ▼ invokes Agent B
//! Agent B (capabilities: delegated = caller_caps ∩ target_caps)
//!         │
//!         ▼ attempts ToolRead("calc")
//!     PermissionEngine::evaluate() -> Permitted
//! ```
//!
//! # Usage
//!
//! ```rust
//! use permission_engine::{PermissionEngine, DefaultPermissionEngine};
//! use agent_protocol::{CapabilitySet, Capability, Action};
//!
//! # async fn example() {
//! let engine = DefaultPermissionEngine::new();
//!
//! // Create capability set
//! let caps = CapabilitySet::default()
//!     .with_capability(Capability::ToolRead { tool_id: "calc".to_string() });
//!
//! // Check if action is permitted
//! let action = Action::ToolCall {
//!     tool_id: "calc".to_string(),
//!     params: serde_json::json!({"expr": "1+1"}),
//! };
//!
//! let result = engine.evaluate(&caps, &action).await;
//! assert!(result.is_permitted());
//! # }
//! ```
//!
//! # Capability Non-Amplification (ADR-002)
//!
//! When Agent A calls Agent B, the capabilities delegated to B are:
//!
//! ```text
//! delegated_caps = caller_caps ∩ target_caps
//! ```
//!
//! This is enforced automatically. No caller API is required.
//! It must never be possible for a sub-Agent to hold capabilities its parent did not have.
//!
//! # License
//!
//! Apache 2.0 - See LICENSE-APACHE file for details.

use agent_protocol::{Action, Capability, CapabilitySet, ProtocolError};

mod evaluator;
mod policy;

pub use evaluator::{DefaultPermissionEngine, EvaluationResult, PermissionEngine};
pub use policy::{DefaultPolicy, PermitAllPolicy, Policy, PolicyError};

/// Compute the delegated capabilities when a caller invokes a target.
///
/// This implements the ADR-002 "capability non-amplification" rule:
///
/// ```text
/// delegated_caps = caller_caps ∩ target_caps
/// ```
///
/// # Arguments
///
/// * `caller_caps` - The capability set of the calling agent
/// * `target_caps` - The capability set requested by/for the target agent
///
/// # Returns
///
/// Returns a new `CapabilitySet` containing only the intersection of both sets.
/// This ensures the target can never receive capabilities the caller doesn't have.
///
/// # Example
///
/// ```rust
/// use permission_engine::compute_delegated_capabilities;
/// use agent_protocol::{CapabilitySet, Capability};
///
/// let caller_caps = CapabilitySet::default()
///     .with_capability(Capability::ToolRead { tool_id: "calc".to_string() })
///     .with_capability(Capability::ToolWrite { tool_id: "calc".to_string() });
///
/// let target_caps = CapabilitySet::default()
///     .with_capability(Capability::ToolRead { tool_id: "calc".to_string() })
///     .with_capability(Capability::ToolRead { tool_id: "other".to_string() });
///
/// let delegated = compute_delegated_capabilities(&caller_caps, &target_caps);
///
/// // Only ToolRead("calc") is in the intersection
/// assert!(delegated.contains(&Capability::ToolRead { tool_id: "calc".to_string() }));
/// assert!(!delegated.contains(&Capability::ToolWrite { tool_id: "calc".to_string() }));
/// assert!(!delegated.contains(&Capability::ToolRead { tool_id: "other".to_string() }));
/// ```
pub fn compute_delegated_capabilities(
    caller_caps: &CapabilitySet,
    target_caps: &CapabilitySet,
) -> CapabilitySet {
    // ADR-002: delegated_caps = caller_caps ∩ target_caps
    caller_caps.intersection(target_caps)
}

/// Check if a capability set permits a specific action.
///
/// This is the core permission check that maps actions to required capabilities.
///
/// # Action to Capability Mapping
///
/// | Action | Required Capability |
/// |--------|-------------------|
/// | `ToolCall { tool_id }` | `ToolRead { tool_id }` |
/// | `CallAgent { target_id }` | `AgentInvoke { target_id }` |
/// | `ContextRead { key }` | `ContextAccess { scope }` |
/// | `ContextWrite { key, .. }` | `ContextAccess { scope }` |
///
/// # Arguments
///
/// * `caps` - The capability set to check against
/// * `action` - The action to evaluate
///
/// # Returns
///
/// Returns `true` if the action is permitted, `false` otherwise.
///
/// # Example
///
/// ```rust
/// use permission_engine::action_permitted;
/// use agent_protocol::{CapabilitySet, Capability, Action};
///
/// let caps = CapabilitySet::default()
///     .with_capability(Capability::ToolRead { tool_id: "calc".to_string() });
///
/// let action = Action::ToolCall {
///     tool_id: "calc".to_string(),
///     params: serde_json::json!({}),
/// };
///
/// assert!(action_permitted(&caps, &action));
/// ```
pub fn action_permitted(caps: &CapabilitySet, action: &Action) -> bool {
    let required = required_capability_for_action(action);
    match required {
        Some(cap) => caps.contains(&cap),
        None => false, // Unknown actions require explicit capability
    }
}

/// Get the required capability for a specific action.
///
/// This function maps each action variant to its corresponding capability requirement.
/// Returns `None` if the action is not recognized.
fn required_capability_for_action(action: &Action) -> Option<Capability> {
    match action {
        Action::ToolCall { tool_id, .. } => Some(Capability::ToolRead {
            tool_id: tool_id.clone(),
        }),
        Action::CallAgent { target_id, .. } => Some(Capability::AgentInvoke {
            agent_id: *target_id,
        }),
        Action::ContextRead { key } => {
            // Extract scope from key (e.g., "memory/conversation" -> "memory")
            let scope = key.split('/').next().unwrap_or("default").to_string();
            Some(Capability::ContextAccess { scope })
        }
        Action::ContextWrite { key, .. } => {
            // Extract scope from key
            let scope = key.split('/').next().unwrap_or("default").to_string();
            Some(Capability::ContextAccess { scope })
        }
        _ => None, // Future action variants not yet supported
    }
}

/// Validates that a capability set satisfies all requirements for an action.
///
/// This function performs a full validation including:
/// - Checking the action is permitted
/// - Verifying no capability amplification occurs
///
/// # Arguments
///
/// * `caller_caps` - The capabilities of the calling agent
/// * `target_caps` - The capabilities being delegated to the target
/// * `action` - The action to validate
///
/// # Returns
///
/// Returns `Ok(())` if validation passes.
/// Returns `Err(ProtocolError::PolicyViolation)` if validation fails.
pub fn validate_action(
    caller_caps: &CapabilitySet,
    target_caps: &CapabilitySet,
    action: &Action,
) -> Result<(), ProtocolError> {
    // First, check for capability amplification
    let delegated = compute_delegated_capabilities(caller_caps, target_caps);
    if delegated != *target_caps {
        return Err(ProtocolError::PolicyViolation {
            action: action.clone(),
            missing_cap: Capability::ToolRead {
                tool_id: "amplification-detected".to_string(),
            },
            agent_id: agent_protocol::AgentId::new(), // TODO: Get actual agent ID
        });
    }

    // Then, check if the action is permitted
    if !action_permitted(caller_caps, action) {
        return Err(ProtocolError::PolicyViolation {
            action: action.clone(),
            missing_cap: required_capability_for_action(action).unwrap_or(Capability::ToolRead {
                tool_id: "unknown".to_string(),
            }),
            agent_id: agent_protocol::AgentId::new(), // TODO: Get actual agent ID
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_protocol::{AgentId, CapabilitySet};

    #[test]
    fn test_compute_delegated_capabilities() {
        let caller_caps = CapabilitySet::default()
            .with_capability(Capability::ToolRead {
                tool_id: "calc".to_string(),
            })
            .with_capability(Capability::ToolWrite {
                tool_id: "calc".to_string(),
            });

        let target_caps = CapabilitySet::default()
            .with_capability(Capability::ToolRead {
                tool_id: "calc".to_string(),
            })
            .with_capability(Capability::ToolRead {
                tool_id: "other".to_string(),
            });

        let delegated = compute_delegated_capabilities(&caller_caps, &target_caps);

        assert!(delegated.contains(&Capability::ToolRead {
            tool_id: "calc".to_string(),
        }));
        assert!(!delegated.contains(&Capability::ToolWrite {
            tool_id: "calc".to_string(),
        }));
        assert!(!delegated.contains(&Capability::ToolRead {
            tool_id: "other".to_string(),
        }));
    }

    #[test]
    fn test_action_permitted_tool_call() {
        let caps = CapabilitySet::default().with_capability(Capability::ToolRead {
            tool_id: "calc".to_string(),
        });

        let permitted_action = Action::ToolCall {
            tool_id: "calc".to_string(),
            params: serde_json::json!({}),
        };

        let denied_action = Action::ToolCall {
            tool_id: "other".to_string(),
            params: serde_json::json!({}),
        };

        assert!(action_permitted(&caps, &permitted_action));
        assert!(!action_permitted(&caps, &denied_action));
    }

    #[test]
    fn test_action_permitted_call_agent() {
        let target_id = AgentId::new();
        let other_id = AgentId::new();

        let caps = CapabilitySet::default().with_capability(Capability::AgentInvoke {
            agent_id: target_id,
        });

        let permitted_action = Action::CallAgent {
            target_id,
            action: Box::new(Action::ToolCall {
                tool_id: "test".to_string(),
                params: serde_json::json!({}),
            }),
        };

        let denied_action = Action::CallAgent {
            target_id: other_id,
            action: Box::new(Action::ToolCall {
                tool_id: "test".to_string(),
                params: serde_json::json!({}),
            }),
        };

        assert!(action_permitted(&caps, &permitted_action));
        assert!(!action_permitted(&caps, &denied_action));
    }

    #[test]
    fn test_action_permitted_context_operations() {
        let caps = CapabilitySet::default().with_capability(Capability::ContextAccess {
            scope: "memory".to_string(),
        });

        let permitted_read = Action::ContextRead {
            key: "memory/conversation".to_string(),
        };

        let permitted_write = Action::ContextWrite {
            key: "memory/data".to_string(),
            value: serde_json::json!({}),
        };

        let denied_read = Action::ContextRead {
            key: "files/config".to_string(),
        };

        assert!(action_permitted(&caps, &permitted_read));
        assert!(action_permitted(&caps, &permitted_write));
        assert!(!action_permitted(&caps, &denied_read));
    }

    #[test]
    fn test_validate_action_permitted() {
        let caller_caps = CapabilitySet::default().with_capability(Capability::ToolRead {
            tool_id: "calc".to_string(),
        });

        let target_caps = CapabilitySet::default();

        let action = Action::ToolCall {
            tool_id: "calc".to_string(),
            params: serde_json::json!({}),
        };

        let result = validate_action(&caller_caps, &target_caps, &action);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_action_denied() {
        let caller_caps = CapabilitySet::default().with_capability(Capability::ToolRead {
            tool_id: "calc".to_string(),
        });

        let target_caps = CapabilitySet::default();

        let action = Action::ToolCall {
            tool_id: "other".to_string(),
            params: serde_json::json!({}),
        };

        let result = validate_action(&caller_caps, &target_caps, &action);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ProtocolError::PolicyViolation { .. }
        ));
    }

    #[test]
    fn test_validate_action_amplification_detected() {
        let caller_caps = CapabilitySet::default().with_capability(Capability::ToolRead {
            tool_id: "calc".to_string(),
        });

        // Target tries to get more capabilities than caller has
        let target_caps = CapabilitySet::default()
            .with_capability(Capability::ToolRead {
                tool_id: "calc".to_string(),
            })
            .with_capability(Capability::ToolWrite {
                tool_id: "calc".to_string(),
            });

        let action = Action::ToolCall {
            tool_id: "calc".to_string(),
            params: serde_json::json!({}),
        };

        let result = validate_action(&caller_caps, &target_caps, &action);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ProtocolError::PolicyViolation { .. }
        ));
    }
}
