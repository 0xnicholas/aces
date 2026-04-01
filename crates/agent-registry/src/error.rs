//! Error types for the agent registry.
//!
//! Defines errors that can occur during registry operations
//! and their conversion to ProtocolError.

use agent_protocol::{AgentId, HandleId, ProtocolError};
use thiserror::Error;

/// Errors that can occur during registry operations.
#[derive(Error, Debug, Clone, PartialEq)]
pub enum RegistryError {
    /// Agent not found in registry
    #[error("agent not found: {0:?}")]
    AgentNotFound(AgentId),

    /// Handle not found in registry
    #[error("handle not found: {0:?}")]
    HandleNotFound(HandleId),

    /// Handle has expired
    #[error("handle expired: {0:?}")]
    HandleExpired(HandleId),

    /// Handle has been revoked
    #[error("handle revoked: {0:?}")]
    HandleRevoked(HandleId),

    /// Agent already exists
    #[error("agent already exists: {0:?}")]
    DuplicateAgent(AgentId),

    /// Handle already exists
    #[error("handle already exists: {0:?}")]
    DuplicateHandle(HandleId),

    /// Registry capacity exceeded
    #[error("registry capacity exceeded")]
    CapacityExceeded,

    /// Invalid agent status transition
    #[error("invalid status transition from {from:?} to {to:?}")]
    InvalidStatusTransition {
        from: crate::entry::AgentStatus,
        to: crate::entry::AgentStatus,
    },

    /// Internal error
    #[error("internal error: {0}")]
    Internal(String),
}

impl From<RegistryError> for ProtocolError {
    fn from(err: RegistryError) -> Self {
        use agent_protocol::HandleInvalidReason;

        match err {
            RegistryError::AgentNotFound(_) | RegistryError::HandleNotFound(_) => {
                ProtocolError::InvalidHandle {
                    reason: HandleInvalidReason::Unrecognised,
                }
            }
            RegistryError::HandleExpired(_) => ProtocolError::InvalidHandle {
                reason: HandleInvalidReason::Expired,
            },
            RegistryError::HandleRevoked(_) => ProtocolError::InvalidHandle {
                reason: HandleInvalidReason::Revoked,
            },
            RegistryError::DuplicateAgent(_) | RegistryError::DuplicateHandle(_) => {
                ProtocolError::ProtocolViolation {
                    detail: err.to_string(),
                }
            }
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
        let agent_id = AgentId::new();
        let err = RegistryError::AgentNotFound(agent_id);
        assert!(err.to_string().contains("agent not found"));

        let handle_id = HandleId::new();
        let err = RegistryError::HandleRevoked(handle_id);
        assert!(err.to_string().contains("handle revoked"));
    }

    #[test]
    fn test_error_conversion_not_found() {
        let agent_id = AgentId::new();
        let registry_err = RegistryError::AgentNotFound(agent_id);

        let protocol_err: ProtocolError = registry_err.into();
        assert!(matches!(
            protocol_err,
            ProtocolError::InvalidHandle {
                reason: agent_protocol::HandleInvalidReason::Unrecognised,
            }
        ));
    }

    #[test]
    fn test_error_conversion_expired() {
        let handle_id = HandleId::new();
        let registry_err = RegistryError::HandleExpired(handle_id);

        let protocol_err: ProtocolError = registry_err.into();
        assert!(matches!(
            protocol_err,
            ProtocolError::InvalidHandle {
                reason: agent_protocol::HandleInvalidReason::Expired,
            }
        ));
    }

    #[test]
    fn test_error_conversion_revoked() {
        let handle_id = HandleId::new();
        let registry_err = RegistryError::HandleRevoked(handle_id);

        let protocol_err: ProtocolError = registry_err.into();
        assert!(matches!(
            protocol_err,
            ProtocolError::InvalidHandle {
                reason: agent_protocol::HandleInvalidReason::Revoked,
            }
        ));
    }

    #[test]
    fn test_error_conversion_duplicate() {
        let agent_id = AgentId::new();
        let registry_err = RegistryError::DuplicateAgent(agent_id);

        let protocol_err: ProtocolError = registry_err.into();
        assert!(matches!(
            protocol_err,
            ProtocolError::ProtocolViolation { .. }
        ));
    }
}
