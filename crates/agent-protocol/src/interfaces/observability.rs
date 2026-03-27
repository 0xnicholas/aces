//! ObservabilityHook interface - mandatory tracing (ptrace analogy)

use crate::errors::ProtocolError;
use crate::types::{Action, ActionResult, RunId, SpanId};

/// Interface for observability and tracing hooks
///
/// Analogous to POSIX ptrace operations
///
/// These hooks cannot be disabled or mocked in production builds.
pub trait ObservabilityHook {
    /// Called when an invoke begins
    fn on_invoke_begin(
        &self,
        run_id: RunId,
        span_id: SpanId,
        action: &Action,
    ) -> impl std::future::Future<Output = Result<(), ProtocolError>> + Send;

    /// Called when an invoke completes
    fn on_invoke_end(
        &self,
        run_id: RunId,
        span_id: SpanId,
        result: &ActionResult,
    ) -> impl std::future::Future<Output = Result<(), ProtocolError>> + Send;

    /// Called when an error occurs
    fn on_error(
        &self,
        run_id: RunId,
        span_id: SpanId,
        error: &ProtocolError,
    ) -> impl std::future::Future<Output = Result<(), ProtocolError>> + Send;
}

/// A no-op observability hook for testing
pub struct NoopObservabilityHook;

impl ObservabilityHook for NoopObservabilityHook {
    async fn on_invoke_begin(
        &self,
        _run_id: RunId,
        _span_id: SpanId,
        _action: &Action,
    ) -> Result<(), ProtocolError> {
        Ok(())
    }

    async fn on_invoke_end(
        &self,
        _run_id: RunId,
        _span_id: SpanId,
        _result: &ActionResult,
    ) -> Result<(), ProtocolError> {
        Ok(())
    }

    async fn on_error(
        &self,
        _run_id: RunId,
        _span_id: SpanId,
        _error: &ProtocolError,
    ) -> Result<(), ProtocolError> {
        Ok(())
    }
}
