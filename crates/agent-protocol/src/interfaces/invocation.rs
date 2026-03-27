//! Invocation interface - execute actions (read/write analogy)

use crate::errors::ProtocolError;
use crate::types::{Action, ActionResult, KernelHandle, RunId};

/// Interface for invoking actions on Agents
///
/// Analogous to POSIX read/write operations
pub trait Invocation {
    /// Execute an action on behalf of the Agent identified by handle.
    /// This is the single critical path: check -> schedule -> sandbox -> audit.
    fn invoke(
        &self,
        handle: &KernelHandle,
        action: Action,
    ) -> impl std::future::Future<Output = Result<ActionResult, ProtocolError>> + Send;

    /// Invoke an action with streaming results.
    fn invoke_stream(
        &self,
        handle: &KernelHandle,
        action: Action,
    ) -> impl std::future::Future<
        Output = Result<futures::stream::BoxStream<'_, ActionResult>, ProtocolError>,
    > + Send;

    /// Cancel an in-flight invocation.
    fn cancel(
        &self,
        run_id: RunId,
    ) -> impl std::future::Future<Output = Result<(), ProtocolError>> + Send;
}
