//! Agent Protocol - Open specification for Agent communication
//!
//! This crate defines the core types, interfaces, and error taxonomy for the Agent Protocol,
//! which serves as the contract between Agent Runtimes and governance implementations.
//!
//! # Overview
//!
//! The Agent Protocol specifies:
//! - **Five interface families**: Lifecycle, Invocation, ContextIO, SignalEvent, Observability
//! - **Six semantic constraints**: Idempotency, non-amplification, observability, structured errors,
//!   cancellation propagation, and WAL audit
//! - **Eight error types**: All errors use structured ProtocolError enum
//! - **Capability-based security**: Authority is explicit, non-ambient, and non-amplifiable
//!
//! # License
//!
//! MIT - See LICENSE-MIT file for details. This is an open specification that can be
//! independently implemented by third parties.
//!
//! # Architecture
//!
//! ```text
//! Agent Runtime
//!       │
//!       ▼
//! Agent Protocol (this crate) ← MIT License
//!       │
//!       ▼
//! Agent Kernel (implementation) ← Apache 2.0
//!       │
//!       ▼
//! Execution Substrate
//! ```
//!
//! # Key Types
//!
//! - [`AgentId`], [`RunId`], [`SpanId`] - Identity types
//! - [`Capability`], [`CapabilitySet`] - Authority model
//! - [`Action`] - All possible actions
//! - [`KernelHandle`] - Opaque handle to spawned Agent
//! - [`ProtocolError`] - Structured error taxonomy
//!
//! # Interface Families
//!
//! - [`AgentLifecycle`] - Manage Agent existence (spawn, suspend, resume, terminate)
//! - [`Invocation`] - Execute actions (invoke, invoke_stream, cancel)
//! - [`ContextIO`] - Memory access (read, write, search, snapshot, restore)
//! - [`SignalEvent`] - Events and HITL (emit, subscribe, interrupt, confirm)
//! - [`ObservabilityHook`] - Mandatory tracing (on_invoke_begin, on_invoke_end, on_error)
//!
//! # Example
//!
//! ```rust
//! use agent_protocol::*;
//!
//! // Create identities
//! let agent_id = AgentId::new();
//! let run_id = RunId::new();
//!
//! // Define capabilities
//! let caps = CapabilitySet::default()
//!     .with_capability(Capability::ToolRead { tool_id: "calculator".to_string() });
//!
//! // Create an action
//! let action = Action::ToolCall {
//!     tool_id: "calculator".to_string(),
//!     params: serde_json::json!({"expression": "1 + 2 + 3"}),
//! };
//! ```

pub mod errors;
pub mod interfaces;
pub mod types;

// Re-export commonly used types
pub use types::*;

// Re-export error types
pub use errors::{
    compare_by_priority, error_priority, HandleInvalidReason, HumanDecision, InterruptToken,
    ProtocolError, ResourceKind,
};

// Re-export interfaces
pub use interfaces::{AgentLifecycle, ContextIO, Invocation, ObservabilityHook, SignalEvent};

// Re-export SignalEvent types
pub use interfaces::signal_event::{ConfirmationRequest, Signal};

// Re-export ObservabilityHook types
pub use interfaces::observability::NoopObservabilityHook;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crate_exports_all_key_types() {
        // This test ensures key types are exported
        let _agent_id = AgentId::new();
        let _run_id = RunId::new();
        let _span_id = SpanId::new();
        let _caps = CapabilitySet::empty();
    }
}
