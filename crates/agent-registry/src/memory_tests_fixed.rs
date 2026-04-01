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

        registry.register(agent_id, handle, def, caps).await.unwrap();

        let entry = registry.get(agent_id).await.unwrap();
        assert_eq!(entry.agent_id, agent_id);
        assert!(entry.is_active());

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

        registry.register(agent_id, handle1, def.clone(), caps.clone()).await.unwrap();

        let result = registry.register(agent_id, handle2, def, caps).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RegistryError::DuplicateAgent(..)));
    }

    #[tokio::test]
    async fn test_is_valid() {
        let registry = InMemoryRegistry::new();

        let agent_id = AgentId::new();
        let handle = KernelHandle::new(agent_id, CapabilitySet::default());
        let handle_id = handle.handle_id.clone();
        let def = AgentDef::new("test-agent");
        let caps = CapabilitySet::default();

        registry.register(agent_id, handle, def, caps).await.unwrap();

        assert!(registry.is_valid(handle_id.clone()).await);
        registry.revoke(handle_id.clone()).await.unwrap();
        assert!(!registry.is_valid(handle_id).await);
    }

    #[tokio::test]
    async fn test_update_status() {
        let registry = InMemoryRegistry::new();

        let agent_id = AgentId::new();
        let handle = KernelHandle::new(agent_id, CapabilitySet::default());
        let def = AgentDef::new("test-agent");
        let caps = CapabilitySet::default();

        registry.register(agent_id, handle, def, caps).await.unwrap();

        registry.update_status(agent_id, AgentStatus::Suspended).await.unwrap();
        let entry = registry.get(agent_id).await.unwrap();
        assert_eq!(entry.status, AgentStatus::Suspended);

        registry.update_status(agent_id, AgentStatus::Active).await.unwrap();
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

        registry.register(agent_id, handle, def, caps).await.unwrap();
        registry.revoke(handle_id).await.unwrap();

        let result = registry.update_status(agent_id, AgentStatus::Active).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RegistryError::InvalidStatusTransition { .. }));
    }

    #[tokio::test]
    async fn test_list_active() {
        let registry = InMemoryRegistry::new();

        let mut handles = Vec::new();
        for i in 0..3 {
            let agent_id = AgentId::new();
            let handle = KernelHandle::new(agent_id, CapabilitySet::default());
            handles.push(handle.handle_id.clone());
            let def = AgentDef::new(format!("agent-{}", i));
            let caps = CapabilitySet::default();
            registry.register(agent_id, handle, def, caps).await.unwrap();
        }

        let active = registry.list_active().await;
        assert_eq!(active.len(), 3);

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
        let caps = CapabilitySet::default()
            .with_capability(agent_protocol::Capability::ToolRead {
                tool_id: "calc".to_string(),
            });

        registry.register(agent_id, handle, def, caps.clone()).await.unwrap();

        let retrieved_caps = registry.get_capabilities(agent_id).await.unwrap();
        assert_eq!(retrieved_caps, caps);
    }

    #[tokio::test]
    async fn test_capacity_limit() {
        let registry = InMemoryRegistry::with_capacity(2);

        for i in 0..2 {
            let agent_id = AgentId::new();
            let handle = KernelHandle::new(agent_id, CapabilitySet::default());
            let def = AgentDef::new(format!("agent-{}", i));
            let caps = CapabilitySet::default();
            registry.register(agent_id, handle, def, caps).await.unwrap();
        }

        let agent_id = AgentId::new();
        let handle = KernelHandle::new(agent_id, CapabilitySet::default());
        let def = AgentDef::new("agent-3");
        let caps = CapabilitySet::default();

        let result = registry.register(agent_id, handle, def, caps).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RegistryError::CapacityExceeded));
    }

    #[tokio::test]
    async fn test_stats_tracking() {
        let registry = InMemoryRegistry::new();

        let agent_id1 = AgentId::new();
        let handle1 = KernelHandle::new(agent_id1, CapabilitySet::default());
        let handle1_id = handle1.handle_id.clone();
        registry.register(agent_id1, handle1, AgentDef::new("agent-1"), CapabilitySet::default()).await.unwrap();

        let agent_id2 = AgentId::new();
        let handle2 = KernelHandle::new(agent_id2, CapabilitySet::default());
        registry.register(agent_id2, handle2, AgentDef::new("agent-2"), CapabilitySet::default()).await.unwrap();

        let stats = registry.stats().await;
        assert_eq!(stats.total_registered, 2);
        assert_eq!(stats.active_count, 2);

        registry.revoke(handle1_id).await.unwrap();

        let stats = registry.stats().await;
        assert_eq!(stats.active_count, 1);
        assert_eq!(stats.revoked_count, 1);
    }
}
