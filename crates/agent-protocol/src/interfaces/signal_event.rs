//! SignalEvent interface - events and HITL (signal/wait analogy)

use crate::errors::ProtocolError;
use crate::types::{AgentId, RunId};
use serde_json::Value;

/// Interface for signals, events, and Human-in-the-Loop operations
///
/// Analogous to POSIX signal/wait operations
pub trait SignalEvent {
    /// Emit a signal/event to an Agent.
    fn emit(
        &self,
        agent_id: AgentId,
        signal: Signal,
    ) -> impl std::future::Future<Output = Result<(), ProtocolError>> + Send;

    /// Subscribe to signals/events from an Agent.
    fn subscribe(
        &self,
        agent_id: AgentId,
        event_type: String,
    ) -> impl std::future::Future<
        Output = Result<futures::stream::BoxStream<'_, Signal>, ProtocolError>,
    > + Send;

    /// Interrupt an Agent for HITL confirmation.
    fn interrupt(
        &self,
        run_id: RunId,
        reason: String,
    ) -> impl std::future::Future<Output = Result<ConfirmationRequest, ProtocolError>> + Send;

    /// Confirm/reject a HITL interrupt.
    fn confirm(
        &self,
        confirmation_id: String,
        approved: bool,
    ) -> impl std::future::Future<Output = Result<(), ProtocolError>> + Send;
}

/// Signal/Event types
#[derive(Debug, Clone)]
pub enum Signal {
    /// Standard signal with payload
    Standard { event_type: String, payload: Value },
    /// Error signal
    Error { code: u32, message: String },
    /// Status update
    Status { run_id: RunId, state: String },
}

/// HITL confirmation request
#[derive(Debug, Clone)]
pub struct ConfirmationRequest {
    pub confirmation_id: String,
    pub run_id: RunId,
    pub reason: String,
    pub timeout_ms: u64,
}
