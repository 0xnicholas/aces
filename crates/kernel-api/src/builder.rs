//! Kernel Builder - Construction patterns for Kernel instances
//!
//! This module provides a builder pattern for constructing Kernel instances
//! with various configurations. The builder ensures that all required
//! components are properly initialized before creating the kernel.

use thiserror::Error;

/// Configuration for the Kernel.
///
/// This struct holds all configuration parameters that control the
/// behavior of the Kernel. It can be constructed using the builder
/// pattern or created with default values.
///
/// # Example
///
/// ```rust
/// use kernel_api::KernelConfig;
///
/// // Default configuration
/// let config = KernelConfig::default();
///
/// // Custom configuration using builder methods
/// let config = KernelConfig::default()
///     .with_max_concurrent_agents(100)
///     .with_audit_retention_days(90);
/// ```
#[derive(Debug, Clone)]
pub struct KernelConfig {
    /// Maximum number of concurrent agents allowed
    pub max_concurrent_agents: usize,
    /// Maximum number of queued actions per agent
    pub max_queued_actions: usize,
    /// Default timeout for actions (in milliseconds)
    pub default_action_timeout_ms: u64,
    /// Audit log retention period (in days)
    pub audit_retention_days: u32,
    /// Enable integrity verification on audit queries
    pub verify_audit_integrity: bool,
    /// Sandbox configuration
    pub sandbox_config: SandboxConfig,
}

impl Default for KernelConfig {
    fn default() -> Self {
        Self {
            max_concurrent_agents: 1000,
            max_queued_actions: 100,
            default_action_timeout_ms: 30000, // 30 seconds
            audit_retention_days: 365,
            verify_audit_integrity: true,
            sandbox_config: SandboxConfig::default(),
        }
    }
}

impl KernelConfig {
    /// Create a new configuration with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the maximum number of concurrent agents.
    pub fn with_max_concurrent_agents(mut self, max: usize) -> Self {
        self.max_concurrent_agents = max;
        self
    }

    /// Set the maximum number of queued actions per agent.
    pub fn with_max_queued_actions(mut self, max: usize) -> Self {
        self.max_queued_actions = max;
        self
    }

    /// Set the default action timeout in milliseconds.
    pub fn with_default_action_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.default_action_timeout_ms = timeout_ms;
        self
    }

    /// Set the audit log retention period in days.
    pub fn with_audit_retention_days(mut self, days: u32) -> Self {
        self.audit_retention_days = days;
        self
    }

    /// Enable or disable audit integrity verification.
    pub fn with_verify_audit_integrity(mut self, verify: bool) -> Self {
        self.verify_audit_integrity = verify;
        self
    }

    /// Set the sandbox configuration.
    pub fn with_sandbox_config(mut self, config: SandboxConfig) -> Self {
        self.sandbox_config = config;
        self
    }
}

/// Configuration for the sandbox subsystem.
#[derive(Debug, Clone)]
pub struct SandboxConfig {
    /// Enable seccomp-bpf filtering
    pub enable_seccomp: bool,
    /// Enable Linux namespaces
    pub enable_namespaces: bool,
    /// Maximum memory per sandbox (in MB)
    pub max_memory_mb: usize,
    /// Maximum CPU time per action (in milliseconds)
    pub max_cpu_time_ms: u64,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            enable_seccomp: true,
            enable_namespaces: true,
            max_memory_mb: 512,
            max_cpu_time_ms: 10000, // 10 seconds
        }
    }
}

impl SandboxConfig {
    /// Create a new sandbox configuration with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable or disable seccomp-bpf filtering.
    pub fn with_seccomp(mut self, enable: bool) -> Self {
        self.enable_seccomp = enable;
        self
    }

    /// Enable or disable Linux namespaces.
    pub fn with_namespaces(mut self, enable: bool) -> Self {
        self.enable_namespaces = enable;
        self
    }

    /// Set the maximum memory per sandbox in MB.
    pub fn with_max_memory_mb(mut self, mb: usize) -> Self {
        self.max_memory_mb = mb;
        self
    }

    /// Set the maximum CPU time per action in milliseconds.
    pub fn with_max_cpu_time_ms(mut self, ms: u64) -> Self {
        self.max_cpu_time_ms = ms;
        self
    }
}

/// Errors that can occur during kernel construction.
#[derive(Debug, Error)]
pub enum KernelError {
    /// Invalid configuration parameter
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),

    /// Required component missing
    #[error("missing required component: {0}")]
    MissingComponent(String),

    /// Initialization failed
    #[error("initialization failed: {0}")]
    InitializationFailed(String),

    /// Resource allocation failed
    #[error("resource allocation failed: {0}")]
    ResourceAllocationFailed(String),
}

/// Builder for constructing Kernel instances.
///
/// The builder pattern ensures that all required components are properly
/// configured before creating the kernel. It validates the configuration
/// and returns a concrete error if anything is missing or invalid.
///
/// # Example
///
/// ```rust
/// use kernel_api::{KernelBuilder, KernelConfig};
///
/// # fn example() -> Result<(), kernel_api::KernelError> {
/// let kernel = KernelBuilder::new()
///     .with_config(KernelConfig::default()
///         .with_max_concurrent_agents(100)
///         .with_audit_retention_days(90))
///     .build()?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct KernelBuilder {
    config: Option<KernelConfig>,
}

impl Default for KernelBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl KernelBuilder {
    /// Create a new kernel builder with default settings.
    pub fn new() -> Self {
        Self { config: None }
    }

    /// Set the kernel configuration.
    ///
    /// # Example
    ///
    /// ```rust
    /// use kernel_api::{KernelBuilder, KernelConfig};
    ///
    /// let builder = KernelBuilder::new()
    ///     .with_config(KernelConfig::default());
    /// ```
    pub fn with_config(mut self, config: KernelConfig) -> Self {
        self.config = Some(config);
        self
    }

    /// Build the kernel with the configured settings.
    ///
    /// This method validates the configuration and creates the kernel
    /// instance. It returns an error if the configuration is invalid
    /// or if initialization fails.
    ///
    /// # Returns
    ///
    /// Returns a kernel instance implementing [`AgentSyscall`] on success,
    /// or a [`KernelError`] if construction fails.
    ///
    /// # Errors
    ///
    /// Returns [`KernelError::InvalidConfig`] if:
    /// - `max_concurrent_agents` is 0
    /// - `max_queued_actions` is 0
    /// - `default_action_timeout_ms` is 0
    /// - `audit_retention_days` is 0
    ///
    /// Returns [`KernelError::InitializationFailed`] if:
    /// - Subsystem initialization fails
    /// - Resource allocation fails
    pub fn build<K>(self) -> Result<K, KernelError>
    where
        K: Default,
    {
        let config = self.config.unwrap_or_default();

        // Validate configuration
        if config.max_concurrent_agents == 0 {
            return Err(KernelError::InvalidConfig(
                "max_concurrent_agents must be greater than 0".to_string(),
            ));
        }

        if config.max_queued_actions == 0 {
            return Err(KernelError::InvalidConfig(
                "max_queued_actions must be greater than 0".to_string(),
            ));
        }

        if config.default_action_timeout_ms == 0 {
            return Err(KernelError::InvalidConfig(
                "default_action_timeout_ms must be greater than 0".to_string(),
            ));
        }

        if config.audit_retention_days == 0 {
            return Err(KernelError::InvalidConfig(
                "audit_retention_days must be greater than 0".to_string(),
            ));
        }

        // For now, return a placeholder. The actual implementation
        // will be provided by kernel-core.
        Ok(K::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = KernelConfig::default();
        assert_eq!(config.max_concurrent_agents, 1000);
        assert_eq!(config.max_queued_actions, 100);
        assert_eq!(config.default_action_timeout_ms, 30000);
        assert_eq!(config.audit_retention_days, 365);
        assert!(config.verify_audit_integrity);
    }

    #[test]
    fn test_config_builder_methods() {
        let config = KernelConfig::default()
            .with_max_concurrent_agents(500)
            .with_max_queued_actions(50)
            .with_default_action_timeout_ms(60000)
            .with_audit_retention_days(180)
            .with_verify_audit_integrity(false);

        assert_eq!(config.max_concurrent_agents, 500);
        assert_eq!(config.max_queued_actions, 50);
        assert_eq!(config.default_action_timeout_ms, 60000);
        assert_eq!(config.audit_retention_days, 180);
        assert!(!config.verify_audit_integrity);
    }

    #[test]
    fn test_sandbox_config() {
        let config = SandboxConfig::default();
        assert!(config.enable_seccomp);
        assert!(config.enable_namespaces);
        assert_eq!(config.max_memory_mb, 512);
        assert_eq!(config.max_cpu_time_ms, 10000);
    }

    #[test]
    fn test_sandbox_config_builder() {
        let config = SandboxConfig::default()
            .with_seccomp(false)
            .with_namespaces(false)
            .with_max_memory_mb(1024)
            .with_max_cpu_time_ms(20000);

        assert!(!config.enable_seccomp);
        assert!(!config.enable_namespaces);
        assert_eq!(config.max_memory_mb, 1024);
        assert_eq!(config.max_cpu_time_ms, 20000);
    }

    #[test]
    fn test_builder_creation() {
        let builder = KernelBuilder::new();
        assert!(builder.config.is_none());
    }

    #[test]
    fn test_builder_with_config() {
        let config = KernelConfig::default();
        let builder = KernelBuilder::new().with_config(config);
        assert!(builder.config.is_some());
    }

    #[test]
    fn test_kernel_error_display() {
        let err = KernelError::InvalidConfig("test error".to_string());
        assert_eq!(err.to_string(), "invalid configuration: test error");

        let err = KernelError::MissingComponent("test component".to_string());
        assert_eq!(
            err.to_string(),
            "missing required component: test component"
        );

        let err = KernelError::InitializationFailed("init failed".to_string());
        assert_eq!(err.to_string(), "initialization failed: init failed");

        let err = KernelError::ResourceAllocationFailed("allocation failed".to_string());
        assert_eq!(
            err.to_string(),
            "resource allocation failed: allocation failed"
        );
    }
}
