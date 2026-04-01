//! Kernel Core - Critical dispatch path and subsystem integration
//!
//! This crate implements the kernel's dispatch path, which is the single
//! critical path for all agent operations.
//!
//! # Dispatch Path
//!
//! ```text
//! invoke(handle, action)
//!   │
//!   ├─ 1. identity check     ──► agent-registry::is_valid()
//!   ├─ 2. permission check   ──► permission-engine::evaluate()
//!   ├─ 3. scheduler          ──► scheduler::schedule()
//!   ├─ 4. sandbox            ──► (optional)
//!   ├─ 5. audit write        ──► audit-log::append()
//!   └─ 6. return result
//! ```
//!
//! # Usage
//!
//! ```rust,no_run
//! use kernel_core::{Kernel, KernelConfig};
//! use kernel_api::AgentSyscall;
//! use agent_protocol::{AgentDef, CapabilitySet, Action};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let kernel = Kernel::new(KernelConfig::default()).await?;
//!
//! // Use via AgentSyscall trait
//! let def = AgentDef::new("test-agent");
//! let caps = CapabilitySet::default();
//! let handle = kernel.spawn(def, caps).await?;
//!
//! let action = Action::ToolCall {
//!     tool_id: "test".to_string(),
//!     params: serde_json::json!({}),
//! };
//! let result = kernel.invoke(&handle, action).await?;
//! # Ok(())
//! # }
//! ```

pub mod config;
pub mod dispatch;
pub mod error;

pub use config::KernelConfig;
pub use dispatch::Kernel;
pub use error::KernelError;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crate_exports() {
        // Just verify types are exported
        let _ = KernelConfig::default();
    }
}
