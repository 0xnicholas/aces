//! Agent Registry trait definition.
//!
//! Defines the core interface for agent registry implementations.

use crate::{
    entry::{AgentEntry, AgentStats, AgentStatus},
    error::RegistryError,
};
use agent_protocol::{AgentDef, AgentId, CapabilitySet, HandleId, KernelHandle};
use async_trait::async_trait;

/// Core trait for agent registry implementations.
///
/// This trait defines the interface for managing agent lifecycle,
/// handle validation, and agent lookup.
///
/// # Example
///
/// ```rust
/// use agent_registry::AgentRegistry;
/// use agent_protocol::{AgentId, AgentDef, CapabilitySet, KernelHandle};
///
/// # async fn example<R: AgentRegistry>(registry: &R) -> Result<(), Box<dyn std::error::Error>> {
/// // Register an agent
/// let agent_id = AgentId::new();
/// let handle = KernelHandle::new(agent_id, CapabilitySet::default());
/// let handle_id = handle.handle_id.clone();
/// let def = AgentDef::new("my-agent");
/// let caps = CapabilitySet::default();
///
/// registry.register(agent_id, handle, def, caps).await?;
///
/// // Validate handle (used in dispatch path)
/// let is_valid = registry.is_valid(handle_id).await;
/// assert!(is_valid);
///
/// // Get agent entry
/// let entry = registry.get(agent_id).await?;
/// # Ok(())
/// # }
/// ```
#[async_trait]
pub trait AgentRegistry: Send + Sync {
    /// Register a new agent in the registry.
    ///
    /// # Arguments
    ///
    /// * `agent_id` - Unique agent identifier
    /// * `handle` - Kernel handle for this agent instance
    /// * `def` - Agent definition
    /// * `caps` - Capability set for this agent
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or `RegistryError` if:
    /// - Agent already exists
    /// - Handle already exists
    /// - Registry capacity exceeded
    ///
    /// # Example
    ///
    /// ```rust
    /// use agent_registry::AgentRegistry;
    /// use agent_protocol::{AgentId, AgentDef, CapabilitySet, KernelHandle};
    ///
    /// # async fn example<R: AgentRegistry>(registry: &R) -> Result<(), Box<dyn std::error::Error>> {
    /// let agent_id = AgentId::new();
    /// let handle = KernelHandle::new(agent_id, CapabilitySet::default());
    /// let def = AgentDef::new("my-agent");
    /// let caps = CapabilitySet::default();
    ///
    /// registry.register(agent_id, handle, def, caps).await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn register(
        &self,
        agent_id: AgentId,
        handle: KernelHandle,
        def: AgentDef,
        caps: CapabilitySet,
    ) -> Result<(), RegistryError>;

    /// Get an agent entry by AgentId.
    ///
    /// # Arguments
    ///
    /// * `agent_id` - The agent ID to look up
    ///
    /// # Returns
    ///
    /// Returns the `AgentEntry` on success, or `RegistryError::AgentNotFound`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use agent_registry::AgentRegistry;
    /// use agent_protocol::AgentId;
    ///
    /// # async fn example<R: AgentRegistry>(registry: &R, agent_id: AgentId) -> Result<(), Box<dyn std::error::Error>> {
    /// match registry.get(agent_id).await {
    ///     Ok(entry) => println!("Found agent: {:?}", entry.def),
    ///     Err(e) => println!("Agent not found: {}", e),
    /// }
    /// # Ok(())
    /// # }
    /// ```
    async fn get(&self, agent_id: AgentId) -> Result<AgentEntry, RegistryError>;

    /// Get an agent entry by HandleId.
    ///
    /// Used for handle validation in the dispatch path.
    ///
    /// # Arguments
    ///
    /// * `handle_id` - The handle ID to look up
    ///
    /// # Returns
    ///
    /// Returns the `AgentEntry` on success, or `RegistryError::HandleNotFound`.
    async fn get_by_handle(&self, handle_id: HandleId) -> Result<AgentEntry, RegistryError>;

    /// Check if a handle is valid (active and not revoked/expired).
    ///
    /// This is the primary method used in the dispatch path for identity verification.
    ///
    /// # Arguments
    ///
    /// * `handle_id` - The handle ID to validate
    ///
    /// # Returns
    ///
    /// Returns `true` if the handle is valid and can be used for operations.
    ///
    /// # Performance
    ///
    /// This operation should complete in O(1) time and take less than 0.1ms.
    ///
    /// # Example
    ///
    /// ```rust
    /// use agent_registry::AgentRegistry;
    /// use agent_protocol::HandleId;
    ///
    /// # async fn example<R: AgentRegistry>(registry: &R, handle_id: HandleId) {
    /// if registry.is_valid(handle_id).await {
    ///     println!("Handle is valid");
    /// } else {
    ///     println!("Handle is invalid or revoked");
    /// }
    /// # }
    /// ```
    async fn is_valid(&self, handle_id: HandleId) -> bool;

    /// Update an agent's status.
    ///
    /// # Arguments
    ///
    /// * `agent_id` - The agent to update
    /// * `status` - The new status
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or `RegistryError` if:
    /// - Agent not found
    /// - Invalid status transition
    ///
    /// # Example
    ///
    /// ```rust
    /// use agent_registry::{AgentRegistry, AgentStatus};
    /// use agent_protocol::AgentId;
    ///
    /// # async fn example<R: AgentRegistry>(registry: &R, agent_id: AgentId) -> Result<(), Box<dyn std::error::Error>> {
    /// // Suspend an agent
    /// registry.update_status(agent_id, AgentStatus::Suspended).await?;
    ///
    /// // Resume an agent
    /// registry.update_status(agent_id, AgentStatus::Active).await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn update_status(
        &self,
        agent_id: AgentId,
        status: AgentStatus,
    ) -> Result<(), RegistryError>;

    /// Revoke a handle, terminating the agent.
    ///
    /// # Arguments
    ///
    /// * `handle_id` - The handle to revoke
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or `RegistryError::HandleNotFound`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use agent_registry::AgentRegistry;
    /// use agent_protocol::HandleId;
    ///
    /// # async fn example<R: AgentRegistry>(registry: &R, handle_id: HandleId) -> Result<(), Box<dyn std::error::Error>> {
    /// registry.revoke(handle_id).await?;
    /// println!("Handle revoked");
    /// # Ok(())
    /// # }
    /// ```
    async fn revoke(&self, handle_id: HandleId) -> Result<(), RegistryError>;

    /// List all active agents.
    ///
    /// # Returns
    ///
    /// Returns a vector of all agents with `Active` status.
    ///
    /// # Example
    ///
    /// ```rust
    /// use agent_registry::AgentRegistry;
    ///
    /// # async fn example<R: AgentRegistry>(registry: &R) {
    /// let active_agents = registry.list_active().await;
    /// println!("Active agents: {}", active_agents.len());
    /// # }
    /// ```
    async fn list_active(&self) -> Vec<AgentEntry>;

    /// Get the capability set for an agent.
    ///
    /// Used by permission-engine for capability delegation.
    ///
    /// # Arguments
    ///
    /// * `agent_id` - The agent ID
    ///
    /// # Returns
    ///
    /// Returns the `CapabilitySet` on success, or `RegistryError::AgentNotFound`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use agent_registry::AgentRegistry;
    /// use agent_protocol::AgentId;
    ///
    /// # async fn example<R: AgentRegistry>(registry: &R, agent_id: AgentId) -> Result<(), Box<dyn std::error::Error>> {
    /// let caps = registry.get_capabilities(agent_id).await?;
    /// println!("Got agent capabilities: {:?}", caps);
    /// # Ok(())
    /// # }
    /// ```
    async fn get_capabilities(&self, agent_id: AgentId) -> Result<CapabilitySet, RegistryError>;

    /// Get registry statistics.
    ///
    /// # Returns
    ///
    /// Returns current statistics about the registry state.
    ///
    /// # Example
    ///
    /// ```rust
    /// use agent_registry::AgentRegistry;
    ///
    /// # async fn example<R: AgentRegistry>(registry: &R) {
    /// let stats = registry.stats().await;
    /// println!("Total agents: {}", stats.total_agents());
    /// println!("Active: {}", stats.active_count);
    /// # }
    /// ```
    async fn stats(&self) -> AgentStats;
}
