//! Agent entry and status definitions.
//!
//! Defines the data structures for storing agent information in the registry.

use agent_protocol::{AgentDef, AgentId, CapabilitySet, HandleId, KernelHandle};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Agent lifecycle status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum AgentStatus {
    /// Agent is active and can accept invoke calls
    #[default]
    Active,
    /// Agent is suspended (checkpointed) and not accepting calls
    Suspended,
    /// Agent is permanently terminated
    Terminated,
    /// Agent handle has been explicitly revoked
    Revoked,
}

impl AgentStatus {
    /// Returns true if the agent can accept invoke calls.
    pub fn can_invoke(&self) -> bool {
        matches!(self, AgentStatus::Active)
    }

    /// Returns true if the agent is in a final state (cannot be reactivated).
    pub fn is_final(&self) -> bool {
        matches!(self, AgentStatus::Terminated | AgentStatus::Revoked)
    }

    /// Check if transition to new status is valid.
    pub fn can_transition_to(&self, new_status: AgentStatus) -> bool {
        use AgentStatus::*;

        match (self, new_status) {
            // Same status: always allowed (check this first!)
            (old, new) if old == &new => true,
            // From Active: can go to Suspended, Terminated, or Revoked
            (Active, Suspended | Terminated | Revoked) => true,
            // From Suspended: can go to Active, Terminated, or Revoked
            (Suspended, Active | Terminated | Revoked) => true,
            // From final states: no transitions allowed (except to self, checked above)
            (Terminated | Revoked, _) => false,
            // Everything else is invalid
            _ => false,
        }
    }
}

/// Entry stored in the agent registry.
#[derive(Debug, Clone)]
pub struct AgentEntry {
    /// Unique agent identifier
    pub agent_id: AgentId,
    /// Handle identifier for this agent instance
    pub handle_id: HandleId,
    /// Agent definition
    pub def: AgentDef,
    /// Capability set (immutable after spawn)
    pub caps: CapabilitySet,
    /// Current agent status
    pub status: AgentStatus,
    /// When the agent was created
    pub created_at: Instant,
    /// When the handle was revoked (if applicable)
    pub revoked_at: Option<Instant>,
    /// Number of times this agent has been invoked
    pub invocation_count: u64,
}

impl AgentEntry {
    /// Create a new agent entry from components.
    pub fn new(
        agent_id: AgentId,
        handle: KernelHandle,
        def: AgentDef,
        caps: CapabilitySet,
    ) -> Self {
        Self {
            agent_id,
            handle_id: handle.handle_id,
            def,
            caps,
            status: AgentStatus::Active,
            created_at: Instant::now(),
            revoked_at: None,
            invocation_count: 0,
        }
    }

    /// Get the agent ID.
    pub fn agent_id(&self) -> AgentId {
        self.agent_id
    }

    /// Get the handle ID.
    pub fn handle_id(&self) -> HandleId {
        self.handle_id.clone()
    }

    /// Get the capability set.
    pub fn capabilities(&self) -> &CapabilitySet {
        &self.caps
    }

    /// Check if the agent is active and can accept calls.
    pub fn is_active(&self) -> bool {
        self.status == AgentStatus::Active
    }

    /// Check if the handle is valid (active and not revoked/expired).
    pub fn is_valid(&self) -> bool {
        self.status == AgentStatus::Active
    }

    /// Mark the agent as revoked.
    pub fn revoke(&mut self) {
        self.status = AgentStatus::Revoked;
        self.revoked_at = Some(Instant::now());
    }

    /// Increment invocation count.
    pub fn record_invocation(&mut self) {
        self.invocation_count += 1;
    }

    /// Get the age of this agent.
    pub fn age(&self) -> std::time::Duration {
        self.created_at.elapsed()
    }
}

/// Statistics about the agent registry.
#[derive(Debug, Clone, Default)]
pub struct AgentStats {
    /// Total number of agents ever registered
    pub total_registered: u64,
    /// Number of currently active agents
    pub active_count: usize,
    /// Number of suspended agents
    pub suspended_count: usize,
    /// Number of terminated agents
    pub terminated_count: usize,
    /// Number of revoked agents
    pub revoked_count: usize,
    /// Total number of invocations across all agents
    pub total_invocations: u64,
}

impl AgentStats {
    /// Total number of agents currently in registry.
    pub fn total_agents(&self) -> usize {
        self.active_count + self.suspended_count + self.terminated_count + self.revoked_count
    }

    /// Number of agents that can accept calls.
    pub fn runnable_count(&self) -> usize {
        self.active_count + self.suspended_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_status_default() {
        let status: AgentStatus = Default::default();
        assert_eq!(status, AgentStatus::Active);
    }

    #[test]
    fn test_agent_status_can_invoke() {
        assert!(AgentStatus::Active.can_invoke());
        assert!(!AgentStatus::Suspended.can_invoke());
        assert!(!AgentStatus::Terminated.can_invoke());
        assert!(!AgentStatus::Revoked.can_invoke());
    }

    #[test]
    fn test_agent_status_is_final() {
        assert!(!AgentStatus::Active.is_final());
        assert!(!AgentStatus::Suspended.is_final());
        assert!(AgentStatus::Terminated.is_final());
        assert!(AgentStatus::Revoked.is_final());
    }

    #[test]
    fn test_agent_status_transitions() {
        // Active can go to Suspended, Terminated, Revoked
        assert!(AgentStatus::Active.can_transition_to(AgentStatus::Suspended));
        assert!(AgentStatus::Active.can_transition_to(AgentStatus::Terminated));
        assert!(AgentStatus::Active.can_transition_to(AgentStatus::Revoked));

        // Suspended can go to Active, Terminated, Revoked
        assert!(AgentStatus::Suspended.can_transition_to(AgentStatus::Active));
        assert!(AgentStatus::Suspended.can_transition_to(AgentStatus::Terminated));
        assert!(AgentStatus::Suspended.can_transition_to(AgentStatus::Revoked));

        // Final states cannot transition
        assert!(!AgentStatus::Terminated.can_transition_to(AgentStatus::Active));
        assert!(!AgentStatus::Revoked.can_transition_to(AgentStatus::Active));
        assert!(!AgentStatus::Terminated.can_transition_to(AgentStatus::Revoked));

        // Same status is always allowed
        assert!(AgentStatus::Active.can_transition_to(AgentStatus::Active));
        assert!(AgentStatus::Terminated.can_transition_to(AgentStatus::Terminated));
    }

    #[test]
    fn test_agent_entry_creation() {
        let agent_id = AgentId::new();
        let handle = KernelHandle::new(agent_id, CapabilitySet::default());
        let def = AgentDef::new("test-agent");
        let caps = CapabilitySet::default();

        let entry = AgentEntry::new(agent_id, handle, def, caps);

        assert_eq!(entry.agent_id, agent_id);
        assert_eq!(entry.status, AgentStatus::Active);
        assert!(entry.is_active());
        assert!(entry.is_valid());
        assert_eq!(entry.invocation_count, 0);
    }

    #[test]
    fn test_agent_entry_revoke() {
        let agent_id = AgentId::new();
        let handle = KernelHandle::new(agent_id, CapabilitySet::default());
        let def = AgentDef::new("test-agent");
        let caps = CapabilitySet::default();

        let mut entry = AgentEntry::new(agent_id, handle, def, caps);
        assert!(entry.is_valid());

        entry.revoke();
        assert!(!entry.is_valid());
        assert_eq!(entry.status, AgentStatus::Revoked);
        assert!(entry.revoked_at.is_some());
    }

    #[test]
    fn test_agent_entry_record_invocation() {
        let agent_id = AgentId::new();
        let handle = KernelHandle::new(agent_id, CapabilitySet::default());
        let def = AgentDef::new("test-agent");
        let caps = CapabilitySet::default();

        let mut entry = AgentEntry::new(agent_id, handle, def, caps);

        entry.record_invocation();
        assert_eq!(entry.invocation_count, 1);

        entry.record_invocation();
        assert_eq!(entry.invocation_count, 2);
    }

    #[test]
    fn test_agent_stats() {
        let stats = AgentStats {
            active_count: 5,
            suspended_count: 3,
            terminated_count: 2,
            revoked_count: 1,
            total_invocations: 100,
            ..Default::default()
        };

        assert_eq!(stats.total_agents(), 11);
        assert_eq!(stats.runnable_count(), 8);
    }
}
