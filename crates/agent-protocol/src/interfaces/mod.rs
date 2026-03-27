//! Interface families for the Agent Protocol
//!
//! The Protocol defines five interface families:
//! - AgentLifecycle: manage Agent existence (fork/exec/kill analogy)
//! - Invocation: execute actions (read/write analogy)
//! - ContextIO: memory access (mmap/lseek analogy)
//! - SignalEvent: events and HITL (signal/wait analogy)
//! - ObservabilityHook: mandatory tracing (ptrace analogy)

pub mod context_io;
pub mod invocation;
pub mod lifecycle;
pub mod observability;
pub mod signal_event;

pub use context_io::ContextIO;
pub use invocation::Invocation;
pub use lifecycle::AgentLifecycle;
pub use observability::ObservabilityHook;
pub use signal_event::SignalEvent;
