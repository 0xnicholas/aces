//! Kernel configuration
//!
//! Defines configuration for the kernel and its subsystems.

/// Configuration for the kernel.
#[derive(Debug, Clone)]
pub struct KernelConfig {
    /// Enable audit logging
    pub enable_audit: bool,
    /// Enable permission checking
    pub enable_permissions: bool,
    /// Enable scheduling
    pub enable_scheduler: bool,
    /// Max concurrent agents
    pub max_concurrent_agents: usize,
}

impl Default for KernelConfig {
    fn default() -> Self {
        Self {
            enable_audit: true,
            enable_permissions: true,
            enable_scheduler: true,
            max_concurrent_agents: 1000,
        }
    }
}

impl KernelConfig {
    /// Create a new config with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Disable audit logging (for testing).
    pub fn without_audit(mut self) -> Self {
        self.enable_audit = false;
        self
    }

    /// Disable permission checking (for testing).
    pub fn without_permissions(mut self) -> Self {
        self.enable_permissions = false;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = KernelConfig::default();
        assert!(config.enable_audit);
        assert!(config.enable_permissions);
        assert!(config.enable_scheduler);
        assert_eq!(config.max_concurrent_agents, 1000);
    }

    #[test]
    fn test_config_builder() {
        let config = KernelConfig::default()
            .without_audit()
            .without_permissions();

        assert!(!config.enable_audit);
        assert!(!config.enable_permissions);
    }
}
