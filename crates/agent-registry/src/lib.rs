//! Agent Registry - Agent lifecycle management and lookup
//!
//! This crate provides the agent registry functionality for the Agent Kernel.
//! It manages agent lifecycle, handle validation, and agent lookup required
//! for the dispatch path and Agent-to-Agent calls.
//!
//! # Core Responsibilities
//!
//! - **Agent Storage**: Store active agent instances with metadata
//! - **Handle Validation**: Validate `KernelHandle` on every operation (dispatch path step 1)
//! - **Agent Lookup**: Resolve `AgentId` for Agent-to-Agent calls
//! - **Lifecycle Tracking**: Track agent states: Active, Suspended, Terminated, Revoked
//! - **Capability Retrieval**: Get `CapabilitySet` for permission checks
//!
//! # Architecture
//!
//! ```text
//! spawn() ──► register() ──► AgentEntry stored
//!                            │
//! invoke() ──► is_valid() ───┤
//!                            │
//! revoke() ──► revoke() ─────► mark as Revoked
//! ```
//!
//! # Usage
//!
//! ```rust
//! use agent_registry::{AgentRegistry, InMemoryRegistry, AgentStatus};
//! use agent_protocol::{AgentId, AgentDef, CapabilitySet, KernelHandle};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let registry = InMemoryRegistry::new();
//!
//! // Register an agent
//! let agent_id = AgentId::new();
//! let handle = KernelHandle::new(agent_id, CapabilitySet::default());
//! let handle_id = handle.handle_id.clone();
//! let def = AgentDef::new("my-agent");
//! let caps = CapabilitySet::default();
//!
//! registry.register(agent_id, handle, def, caps).await?;
//!
//! // Validate handle
//! let is_valid = registry.is_valid(handle_id).await;
//! assert!(is_valid);
//!
//! // Get agent entry
//! let entry = registry.get(agent_id).await?;
//! assert_eq!(entry.status, AgentStatus::Active);
//! # Ok(())
//! # }
//! ```
//!
//! # Performance
//!
//! - Handle validation: < 0.1ms (O(1) HashMap lookup)
//! - Agent lookup: O(1) average case
//! - Thread-safe: Uses `tokio::sync::RwLock` for concurrent access
//!
//! # License
//!
//! Apache 2.0 - See LICENSE-APACHE file for details.

pub mod entry;
pub mod error;
pub mod memory;
pub mod registry;

pub use entry::{AgentEntry, AgentStats, AgentStatus};
pub use error::RegistryError;
pub use memory::InMemoryRegistry;
pub use registry::AgentRegistry;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crate_exports() {
        // Ensure all key types are exported
        let _ = AgentStatus::Active;
        let _: RegistryError = RegistryError::AgentNotFound(agent_protocol::AgentId::new());
    }
}
