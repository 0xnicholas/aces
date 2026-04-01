//! Agent Kernel Public API
//!
//! This crate provides the public surface exposed to Runtime callers.
//! It defines exactly four methods as the contract between Runtimes and the Kernel:
//!
//! 1. [`AgentSyscall::spawn`] - Create a new Agent with given capabilities
//! 2. [`AgentSyscall::invoke`] - Execute an action on behalf of an Agent
//! 3. [`AgentSyscall::revoke`] - Revoke an Agent and return summary
//! 4. [`AgentSyscall::query_audit`] - Query the audit log
//!
//! # Usage
//!
//! ```rust,no_run
//! use kernel_api::{AgentSyscall, KernelBuilder, KernelConfig};
//! use agent_protocol::{AgentDef, CapabilitySet};
//! use std::default::Default;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Build a kernel instance
//! let kernel: MockKernel = KernelBuilder::new()
//!     .with_config(KernelConfig::default())
//!     .build()?;
//!
//! // Spawn an agent
//! let def = AgentDef::new("my-agent");
//! let caps = CapabilitySet::default();
//! let handle = kernel.spawn(def, caps).await?;
//! # Ok(())
//! # }
//! # use kernel_api::MockKernel;
//! ```
//!
//! # Testing
//!
//! For testing, use [`MockKernel`] to mock the kernel behavior:
//!
//! ```rust
//! use kernel_api::{AgentSyscall, MockKernel};
//! use agent_protocol::{AgentDef, CapabilitySet, KernelHandle, AgentId};
//!
//! # async fn test() {
//! let mock = MockKernel::new()
//!     .expect_spawn(Ok(KernelHandle::new(AgentId::new(), CapabilitySet::default())));
//!
//! let def = AgentDef::new("test-agent");
//! let handle = mock.spawn(def, CapabilitySet::default()).await.unwrap();
//! # }
//! ```
//!
//! # License
//!
//! Apache 2.0 - See LICENSE-APACHE file for details.

use std::sync::Arc;

mod builder;
mod mock;
mod syscall;

pub use builder::{KernelBuilder, KernelConfig, KernelError};
pub use mock::{MockCall, MockError, MockKernel, MockResult};
pub use syscall::AgentSyscall;

/// A type-erased kernel handle that implements [`AgentSyscall`].
///
/// This allows storing kernels with different implementations in the same
/// container or passing them through APIs without generics.
pub type DynKernel = Arc<dyn AgentSyscall>;

/// Create a new dynamic kernel from any type implementing [`AgentSyscall`].
pub fn into_dyn<K>(kernel: K) -> DynKernel
where
    K: AgentSyscall + 'static,
{
    Arc::new(kernel)
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_protocol::{AgentId, Capability, CapabilitySet};

    #[tokio::test]
    async fn test_kernel_exports_all_types() {
        // This test ensures all key types are exported
        let _agent_id = AgentId::new();
        let _caps = CapabilitySet::empty();
        let _cap = Capability::ToolRead {
            tool_id: "test".to_string(),
        };
    }

    #[test]
    fn test_builder_exists() {
        let _builder = KernelBuilder::new();
    }

    #[test]
    fn test_mock_exists() {
        let _mock = MockKernel::new();
    }
}
