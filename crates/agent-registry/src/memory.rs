//! In-memory agent registry implementation.
//!
//! This implementation stores all agent data in memory using HashMaps.
//! Suitable for testing and single-node deployments.

use crate::{
    entry::{AgentEntry, AgentStats, AgentStatus},
    error::RegistryError,
    registry::AgentRegistry,
};
use agent_protocol::{AgentDef, AgentId, CapabilitySet, HandleId, KernelHandle};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, trace, warn};

/// In-memory agent registry implementation.
///
/// Uses `RwLock<HashMap>` for thread-safe concurrent access.
/// All operations are O(1) average case.
///
/// # Example
///
/// ```rust
/// use agent_registry::InMemoryRegistry;
/// use agent_registry::AgentRegistry;
/// use agent_protocol::{AgentId, AgentDef, CapabilitySet, KernelHandle};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let registry = InMemoryRegistry::new();
///
/// let agent_id = AgentId::new();
/// let handle = KernelHandle::new(agent_id, CapabilitySet::default());
/// let def = AgentDef::new("test-agent");
/// let caps = CapabilitySet::default();
///
/// registry.register(agent_id, handle, def, caps).await?;
///
/// let entry = registry.get(agent_id).await?;
/// assert!(entry.is_active());
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct InMemoryRegistry {
    /// Map from AgentId to AgentEntry
    agents: Arc<RwLock<HashMap<AgentId, AgentEntry>>>,
    /// Map from HandleId to AgentId for reverse lookup
    handles: Arc<RwLock<HashMap<HandleId, AgentId>>>,
    /// Statistics
    stats: Arc<RwLock<AgentStats>>,
    /// Maximum number of agents allowed
    max_capacity: usize,
}

impl InMemoryRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self::with_capacity(10000)
    }

    /// Create a new registry with the specified capacity limit.
    pub fn with_capacity(max_capacity: usize) -> Self {
        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
            handles: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(AgentStats::default())),
            max_capacity,
        }
    }

    /// Get the maximum capacity.
    pub fn max_capacity(&self) -> usize {
        self.max_capacity
    }

    /// Update statistics when an agent is added.
    async fn on_agent_added(&self, status: AgentStatus) {
        let mut stats = self.stats.write().await;
        stats.total_registered += 1;
        match status {
            AgentStatus::Active => stats.active_count += 1,
            AgentStatus::Suspended => stats.suspended_count += 1,
            AgentStatus::Terminated => stats.terminated_count += 1,
            AgentStatus::Revoked => stats.revoked_count += 1,
        }
    }

    /// Update statistics when an agent status changes.
    async fn on_status_changed(&self, old_status: AgentStatus, new_status: AgentStatus) {
        let mut stats = self.stats.write().await;

        // Decrement old status
        match old_status {
            AgentStatus::Active => stats.active_count -= 1,
            AgentStatus::Suspended => stats.suspended_count -= 1,
            AgentStatus::Terminated => stats.terminated_count -= 1,
            AgentStatus::Revoked => stats.revoked_count -= 1,
        }

        // Increment new status
        match new_status {
            AgentStatus::Active => stats.active_count += 1,
            AgentStatus::Suspended => stats.suspended_count += 1,
            AgentStatus::Terminated => stats.terminated_count += 1,
            AgentStatus::Revoked => stats.revoked_count += 1,
        }
    }
}

impl Default for InMemoryRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AgentRegistry for InMemoryRegistry {
    async fn register(
        &self,
        agent_id: AgentId,
        handle: KernelHandle,
        def: AgentDef,
        caps: CapabilitySet,
    ) -> Result<(), RegistryError> {
        trace!(
            "Registering agent {:?} with handle {:?}",
            agent_id,
            handle.handle_id
        );

        // Check capacity
        let current_count = self.agents.read().await.len();
        if current_count >= self.max_capacity {
            warn!(
                "Registry capacity exceeded: {}/{} agents",
                current_count, self.max_capacity
            );
            return Err(RegistryError::CapacityExceeded);
        }

        // Clone handle first, then extract handle_id
        let handle_clone = handle.clone();
        let handle_id = handle.handle_id;

        // Check for duplicate agent
        if self.agents.read().await.contains_key(&agent_id) {
            return Err(RegistryError::DuplicateAgent(agent_id));
        }

        // Check for duplicate handle
        if self.handles.read().await.contains_key(&handle_id) {
            return Err(RegistryError::DuplicateHandle(handle_id));
        }

        // Create entry
        let entry = AgentEntry::new(agent_id, handle_clone, def, caps);
        let status = entry.status;

        // Insert into maps
        self.agents.write().await.insert(agent_id, entry);
        self.handles.write().await.insert(handle_id, agent_id);

        // Update stats
        self.on_agent_added(status).await;

        debug!("Successfully registered agent {:?}", agent_id);
        Ok(())
    }

    async fn get(&self, agent_id: AgentId) -> Result<AgentEntry, RegistryError> {
        trace!("Looking up agent {:?}", agent_id);

        self.agents
            .read()
            .await
            .get(&agent_id)
            .cloned()
            .ok_or_else(|| {
                trace!("Agent {:?} not found", agent_id);
                RegistryError::AgentNotFound(agent_id)
            })
    }

    async fn get_by_handle(&self, handle_id: HandleId) -> Result<AgentEntry, RegistryError> {
        trace!("Looking up handle {:?}", handle_id);

        let agent_id = self
            .handles
            .read()
            .await
            .get(&handle_id)
            .copied()
            .ok_or_else(|| {
                trace!("Handle {:?} not found", handle_id);
                RegistryError::HandleNotFound(handle_id)
            })?;

        self.get(agent_id).await
    }

    async fn is_valid(&self, handle_id: HandleId) -> bool {
        let handle_id_clone = handle_id.clone();
        match self.get_by_handle(handle_id).await {
            Ok(entry) => {
                let valid = entry.is_valid();
                trace!("Handle {:?} validity: {}", handle_id_clone, valid);
                valid
            }
            Err(_) => {
                trace!("Handle {:?} not found, invalid", handle_id_clone);
                false
            }
        }
    }

    async fn update_status(
        &self,
        agent_id: AgentId,
        new_status: AgentStatus,
    ) -> Result<(), RegistryError> {
        trace!("Updating agent {:?} status to {:?}", agent_id, new_status);

        let mut agents = self.agents.write().await;

        if let Some(entry) = agents.get_mut(&agent_id) {
            let old_status = entry.status;

            // Validate transition
            if !old_status.can_transition_to(new_status) {
                warn!(
                    "Invalid status transition for agent {:?}: {:?} -> {:?}",
                    agent_id, old_status, new_status
                );
                return Err(RegistryError::InvalidStatusTransition {
                    from: old_status,
                    to: new_status,
                });
            }

            entry.status = new_status;

            // Update stats
            drop(agents);
            self.on_status_changed(old_status, new_status).await;

            debug!("Agent {:?} status updated to {:?}", agent_id, new_status);
            Ok(())
        } else {
            Err(RegistryError::AgentNotFound(agent_id))
        }
    }

    async fn revoke(&self, handle_id: HandleId) -> Result<(), RegistryError> {
        trace!("Revoking handle {:?}", handle_id);

        let handle_id_clone = handle_id.clone();
        let entry = self.get_by_handle(handle_id).await?;
        let agent_id = entry.agent_id;
        let old_status = entry.status;

        let mut agents = self.agents.write().await;
        if let Some(entry) = agents.get_mut(&agent_id) {
            entry.revoke();

            // Update stats
            drop(agents);
            self.on_status_changed(old_status, AgentStatus::Revoked)
                .await;

            debug!(
                "Handle {:?} revoked for agent {:?}",
                handle_id_clone, agent_id
            );
            Ok(())
        } else {
            Err(RegistryError::AgentNotFound(agent_id))
        }
    }

    async fn list_active(&self) -> Vec<AgentEntry> {
        trace!("Listing active agents");

        self.agents
            .read()
            .await
            .values()
            .filter(|e| e.is_active())
            .cloned()
            .collect()
    }

    async fn get_capabilities(&self, agent_id: AgentId) -> Result<CapabilitySet, RegistryError> {
        trace!("Getting capabilities for agent {:?}", agent_id);

        self.get(agent_id).await.map(|entry| entry.caps)
    }

    async fn stats(&self) -> AgentStats {
        self.stats.read().await.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_registry_creation() {
        let registry = InMemoryRegistry::new();
        let stats = registry.stats().await;
        assert_eq!(stats.total_agents(), 0);
    }

    #[tokio::test]
    async fn test_register_and_get() {
        let registry = InMemoryRegistry::new();

        let agent_id = AgentId::new();
        let handle = KernelHandle::new(agent_id, CapabilitySet::default());
        let handle_id = handle.handle_id.clone();
        let def = AgentDef::new("test-agent");
        let caps = CapabilitySet::default();

        // Register
        registry
            .register(agent_id, handle, def, caps)
            .await
            .unwrap();

        // Get by agent ID
        let entry = registry.get(agent_id).await.unwrap();
        assert_eq!(entry.agent_id, agent_id);
        assert!(entry.is_active());

        // Get by handle
        let entry2 = registry.get_by_handle(handle_id).await.unwrap();
        assert_eq!(entry2.agent_id, agent_id);
    }

    #[tokio::test]
    async fn test_duplicate_agent() {
        let registry = InMemoryRegistry::new();

        let agent_id = AgentId::new();
        let handle1 = KernelHandle::new(agent_id, CapabilitySet::default());
        let handle2 = KernelHandle::new(agent_id, CapabilitySet::default());
        let def = AgentDef::new("test-agent");
        let caps = CapabilitySet::default();

        // First registration should succeed
        registry
            .register(agent_id, handle1, def.clone(), caps.clone())
            .await
            .unwrap();

        // Second registration should fail
        let result = registry.register(agent_id, handle2, def, caps).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RegistryError::DuplicateAgent(..)
        ));
    }

    #[tokio::test]
    async fn test_is_valid() {
        let registry = InMemoryRegistry::new();

        let agent_id = AgentId::new();
        let handle = KernelHandle::new(agent_id, CapabilitySet::default());
        let handle_id = handle.handle_id.clone();
        let def = AgentDef::new("test-agent");
        let caps = CapabilitySet::default();

        registry
            .register(agent_id, handle, def, caps)
            .await
            .unwrap();

        // Should be valid initially
        assert!(registry.is_valid(handle_id.clone()).await);

        // Revoke
        registry.revoke(handle_id.clone()).await.unwrap();

        // Should not be valid after revoke
        assert!(!registry.is_valid(handle_id).await);
    }

    #[tokio::test]
    async fn test_update_status() {
        let registry = InMemoryRegistry::new();

        let agent_id = AgentId::new();
        let handle = KernelHandle::new(agent_id, CapabilitySet::default());
        let def = AgentDef::new("test-agent");
        let caps = CapabilitySet::default();

        registry
            .register(agent_id, handle, def, caps)
            .await
            .unwrap();

        // Suspend
        registry
            .update_status(agent_id, AgentStatus::Suspended)
            .await
            .unwrap();

        let entry = registry.get(agent_id).await.unwrap();
        assert_eq!(entry.status, AgentStatus::Suspended);

        // Resume
        registry
            .update_status(agent_id, AgentStatus::Active)
            .await
            .unwrap();

        let entry = registry.get(agent_id).await.unwrap();
        assert_eq!(entry.status, AgentStatus::Active);
    }

    #[tokio::test]
    async fn test_invalid_status_transition() {
        let registry = InMemoryRegistry::new();

        let agent_id = AgentId::new();
        let handle = KernelHandle::new(agent_id, CapabilitySet::default());
        let handle_id = handle.handle_id.clone();
        let def = AgentDef::new("test-agent");
        let caps = CapabilitySet::default();

        registry
            .register(agent_id, handle, def, caps)
            .await
            .unwrap();

        // Revoke
        registry.revoke(handle_id).await.unwrap();

        // Should not be able to go back to Active
        let result = registry.update_status(agent_id, AgentStatus::Active).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RegistryError::InvalidStatusTransition { .. }
        ));
    }

    #[tokio::test]
    async fn test_list_active() {
        let registry = InMemoryRegistry::new();

        // Register 3 agents
        let mut handles = Vec::new();
        for i in 0..3 {
            let agent_id = AgentId::new();
            let handle = KernelHandle::new(agent_id, CapabilitySet::default());
            handles.push(handle.handle_id.clone());
            let def = AgentDef::new(format!("agent-{}", i));
            let caps = CapabilitySet::default();

            registry
                .register(agent_id, handle, def, caps)
                .await
                .unwrap();
        }

        let active = registry.list_active().await;
        assert_eq!(active.len(), 3);

        // Revoke one
        registry.revoke(handles[0].clone()).await.unwrap();

        let active = registry.list_active().await;
        assert_eq!(active.len(), 2);
    }

    #[tokio::test]
    async fn test_get_capabilities() {
        let registry = InMemoryRegistry::new();

        let agent_id = AgentId::new();
        let handle = KernelHandle::new(agent_id, CapabilitySet::default());
        let def = AgentDef::new("test-agent");
        let caps = CapabilitySet::default().with_capability(agent_protocol::Capability::ToolRead {
            tool_id: "calc".to_string(),
        });

        registry
            .register(agent_id, handle, def, caps.clone())
            .await
            .unwrap();

        let retrieved_caps = registry.get_capabilities(agent_id).await.unwrap();
        assert_eq!(retrieved_caps, caps);
    }

    #[tokio::test]
    async fn test_capacity_limit() {
        let registry = InMemoryRegistry::with_capacity(2);

        // Register 2 agents (at capacity)
        for i in 0..2 {
            let agent_id = AgentId::new();
            let handle = KernelHandle::new(agent_id, CapabilitySet::default());
            let def = AgentDef::new(format!("agent-{}", i));
            let caps = CapabilitySet::default();

            registry
                .register(agent_id, handle, def, caps)
                .await
                .unwrap();
        }

        // Third agent should fail
        let agent_id = AgentId::new();
        let handle = KernelHandle::new(agent_id, CapabilitySet::default());
        let def = AgentDef::new("agent-3");
        let caps = CapabilitySet::default();

        let result = registry.register(agent_id, handle, def, caps).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RegistryError::CapacityExceeded
        ));
    }

    #[tokio::test]
    async fn test_stats_tracking() {
        let registry = InMemoryRegistry::new();

        // Register 2 agents
        let agent_id1 = AgentId::new();
        let handle1 = KernelHandle::new(agent_id1, CapabilitySet::default());
        let handle1_id = handle1.handle_id.clone();
        registry
            .register(
                agent_id1,
                handle1,
                AgentDef::new("agent-1"),
                CapabilitySet::default(),
            )
            .await
            .unwrap();

        let agent_id2 = AgentId::new();
        let handle2 = KernelHandle::new(agent_id2, CapabilitySet::default());
        registry
            .register(
                agent_id2,
                handle2,
                AgentDef::new("agent-2"),
                CapabilitySet::default(),
            )
            .await
            .unwrap();

        let stats = registry.stats().await;
        assert_eq!(stats.total_registered, 2);
        assert_eq!(stats.active_count, 2);

        // Revoke one
        registry.revoke(handle1_id).await.unwrap();

        let stats = registry.stats().await;
        assert_eq!(stats.active_count, 1);
        assert_eq!(stats.revoked_count, 1);
    }
}
