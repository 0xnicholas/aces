//! Mock Kernel - Testing utilities for the Kernel API
//!
//! This module provides mock implementations of [`AgentSyscall`] for testing
//! purposes. The mock allows you to:
//!
//! - Set expected return values for each method
//! - Verify that methods were called with expected arguments
//! - Test error handling paths
//! - Simulate various scenarios without a real kernel
//!
//! # Example
//!
//! ```rust
//! use kernel_api::{AgentSyscall, MockKernel, MockCall, MockResult};
//! use agent_protocol::{AgentDef, CapabilitySet, KernelHandle, AgentId};
//!
//! # async fn test() {
//! let mock = MockKernel::new()
//!     .expect_spawn(Ok(KernelHandle::new(AgentId::new(), CapabilitySet::default())))
//!     .expect_invoke(Ok(agent_protocol::ActionResult::Success(serde_json::json!(null))));
//!
//! let def = AgentDef::new("test-agent");
//! let handle = mock.spawn(def, CapabilitySet::default()).await.unwrap();
//!
//! // Verify expectations were met
//! mock.verify().unwrap();
//! # }
//! ```

use crate::AgentSyscall;
use agent_protocol::{
    Action, ActionResult, AgentDef, AuditFilter, CapabilitySet, KernelHandle, LogEntry,
    ProtocolError, RunSummary,
};
use async_trait::async_trait;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use thiserror::Error;

/// Errors that can occur when using the mock kernel.
#[derive(Debug, Error)]
pub enum MockError {
    /// Expected method was not called
    #[error("expected method {method} to be called, but it was not")]
    ExpectedCallNotMade { method: String },

    /// Method was called but no expectation was set
    #[error("unexpected call to {method}")]
    UnexpectedCall { method: String },

    /// Wrong number of arguments
    #[error("wrong number of arguments for {method}: expected {expected}, got {actual}")]
    WrongArgumentCount {
        method: String,
        expected: usize,
        actual: usize,
    },

    /// Verification failed
    #[error("verification failed: {0}")]
    VerificationFailed(String),
}

/// Tracks a call made to the mock kernel.
#[derive(Debug, Clone)]
pub struct MockCall {
    /// Name of the method called
    pub method: String,
    /// Arguments passed to the method (serialized as JSON for inspection)
    pub arguments: Vec<serde_json::Value>,
}

/// The result to return from a mock method call.
pub type MockResult<T> = Result<T, ProtocolError>;

/// A mock implementation of [`AgentSyscall`] for testing.
///
/// The mock allows you to set expectations for method calls and their
/// return values. It tracks all calls made and can verify that all
/// expectations were met.
///
/// # Usage
///
/// 1. Create a new mock with [`MockKernel::new`]
/// 2. Set expectations using `expect_*` methods
/// 3. Use the mock in your tests
/// 4. Call [`MockKernel::verify`] to ensure all expectations were met
///
/// # Example
///
/// ```rust
/// use kernel_api::{AgentSyscall, MockKernel};
/// use agent_protocol::{AgentDef, CapabilitySet, KernelHandle, AgentId};
///
/// # async fn test() {
/// let mock = MockKernel::new()
///     .expect_spawn(Ok(KernelHandle::new(AgentId::new(), CapabilitySet::default())));
///
/// let def = AgentDef::new("test-agent");
/// let caps = CapabilitySet::default();
/// let handle = mock.spawn(def, caps).await.unwrap();
///
/// // Verify all expectations were met
/// mock.verify().unwrap();
/// # }
/// ```
#[derive(Debug)]
pub struct MockKernel {
    state: Arc<Mutex<MockState>>,
}

#[derive(Debug)]
struct MockState {
    spawn_expectations: VecDeque<MockResult<KernelHandle>>,
    invoke_expectations: VecDeque<MockResult<ActionResult>>,
    revoke_expectations: VecDeque<MockResult<RunSummary>>,
    query_audit_expectations: VecDeque<MockResult<Vec<LogEntry>>>,
    calls: Vec<MockCall>,
}

impl Default for MockKernel {
    fn default() -> Self {
        Self::new()
    }
}

impl MockKernel {
    /// Create a new mock kernel with no expectations set.
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(MockState {
                spawn_expectations: VecDeque::new(),
                invoke_expectations: VecDeque::new(),
                revoke_expectations: VecDeque::new(),
                query_audit_expectations: VecDeque::new(),
                calls: Vec::new(),
            })),
        }
    }

    /// Set the expected result for the next `spawn` call.
    ///
    /// # Example
    ///
    /// ```rust
    /// use kernel_api::MockKernel;
    /// use agent_protocol::{KernelHandle, AgentId, CapabilitySet};
    ///
    /// let mock = MockKernel::new()
    ///     .expect_spawn(Ok(KernelHandle::new(AgentId::new(), CapabilitySet::default())));
    /// ```
    pub fn expect_spawn(mut self, result: MockResult<KernelHandle>) -> Self {
        let state = Arc::get_mut(&mut self.state)
            .expect("MockKernel has been cloned, cannot set expectations");
        state
            .get_mut()
            .unwrap()
            .spawn_expectations
            .push_back(result);
        self
    }

    /// Set the expected result for the next `invoke` call.
    ///
    /// # Example
    ///
    /// ```rust
    /// use kernel_api::MockKernel;
    /// use agent_protocol::ActionResult;
    ///
    /// let mock = MockKernel::new()
    ///     .expect_invoke(Ok(ActionResult::Success(serde_json::json!("result"))));
    /// ```
    pub fn expect_invoke(mut self, result: MockResult<ActionResult>) -> Self {
        let state = Arc::get_mut(&mut self.state)
            .expect("MockKernel has been cloned, cannot set expectations");
        state
            .get_mut()
            .unwrap()
            .invoke_expectations
            .push_back(result);
        self
    }

    /// Set the expected result for the next `revoke` call.
    ///
    /// # Example
    ///
    /// ```rust
    /// use kernel_api::MockKernel;
    /// use agent_protocol::{RunSummary, RunId};
    ///
    /// let mock = MockKernel::new()
    ///     .expect_revoke(Ok(RunSummary {
    ///         run_id: RunId::new(),
    ///         actions_executed: 5,
    ///         final_status: "completed".to_string(),
    ///     }));
    /// ```
    pub fn expect_revoke(mut self, result: MockResult<RunSummary>) -> Self {
        let state = Arc::get_mut(&mut self.state)
            .expect("MockKernel has been cloned, cannot set expectations");
        state
            .get_mut()
            .unwrap()
            .revoke_expectations
            .push_back(result);
        self
    }

    /// Set the expected result for the next `query_audit` call.
    ///
    /// # Example
    ///
    /// ```rust
    /// use kernel_api::MockKernel;
    ///
    /// let mock = MockKernel::new()
    ///     .expect_query_audit(Ok(vec![]));
    /// ```
    pub fn expect_query_audit(mut self, result: MockResult<Vec<LogEntry>>) -> Self {
        let state = Arc::get_mut(&mut self.state)
            .expect("MockKernel has been cloned, cannot set expectations");
        state
            .get_mut()
            .unwrap()
            .query_audit_expectations
            .push_back(result);
        self
    }

    /// Verify that all expected calls were made.
    ///
    /// Returns an error if:
    /// - Any expected calls were not made
    /// - Unexpected calls were made (if strict mode)
    ///
    /// # Example
    ///
    /// ```rust
    /// use kernel_api::MockKernel;
    ///
    /// let mock = MockKernel::new();
    /// // ... use mock ...
    /// mock.verify().unwrap();
    /// ```
    pub fn verify(&self) -> Result<(), MockError> {
        let state = self.state.lock().unwrap();

        // Check for unmet spawn expectations
        if !state.spawn_expectations.is_empty() {
            return Err(MockError::ExpectedCallNotMade {
                method: "spawn".to_string(),
            });
        }

        // Check for unmet invoke expectations
        if !state.invoke_expectations.is_empty() {
            return Err(MockError::ExpectedCallNotMade {
                method: "invoke".to_string(),
            });
        }

        // Check for unmet revoke expectations
        if !state.revoke_expectations.is_empty() {
            return Err(MockError::ExpectedCallNotMade {
                method: "revoke".to_string(),
            });
        }

        // Check for unmet query_audit expectations
        if !state.query_audit_expectations.is_empty() {
            return Err(MockError::ExpectedCallNotMade {
                method: "query_audit".to_string(),
            });
        }

        Ok(())
    }

    /// Get all calls made to this mock.
    ///
    /// This is useful for inspecting the exact arguments passed to methods.
    ///
    /// # Example
    ///
    /// ```rust
    /// use kernel_api::MockKernel;
    ///
    /// let mock = MockKernel::new();
    /// // ... use mock ...
    /// let calls = mock.calls();
    /// for call in calls {
    ///     println!("Called: {} with {:?}", call.method, call.arguments);
    /// }
    /// ```
    pub fn calls(&self) -> Vec<MockCall> {
        self.state.lock().unwrap().calls.clone()
    }

    /// Clear all expectations and recorded calls.
    ///
    /// This is useful for reusing a mock in multiple test cases.
    pub fn reset(&mut self) {
        let state =
            Arc::get_mut(&mut self.state).expect("MockKernel has been cloned, cannot reset");
        let state = state.get_mut().unwrap();
        state.spawn_expectations.clear();
        state.invoke_expectations.clear();
        state.revoke_expectations.clear();
        state.query_audit_expectations.clear();
        state.calls.clear();
    }

    fn record_call(&self, method: &str, arguments: Vec<serde_json::Value>) {
        let mut state = self.state.lock().unwrap();
        state.calls.push(MockCall {
            method: method.to_string(),
            arguments,
        });
    }
}

#[async_trait]
impl AgentSyscall for MockKernel {
    async fn spawn(
        &self,
        def: AgentDef,
        caps: CapabilitySet,
    ) -> Result<KernelHandle, ProtocolError> {
        self.record_call(
            "spawn",
            vec![
                serde_json::to_value(&def).unwrap_or(serde_json::Value::Null),
                serde_json::to_value(&caps).unwrap_or(serde_json::Value::Null),
            ],
        );

        let mut state = self.state.lock().unwrap();
        state
            .spawn_expectations
            .pop_front()
            .ok_or(ProtocolError::InvalidHandle {
                reason: agent_protocol::HandleInvalidReason::Expired,
            })?
    }

    async fn invoke(
        &self,
        handle: &KernelHandle,
        action: Action,
    ) -> Result<ActionResult, ProtocolError> {
        self.record_call(
            "invoke",
            vec![
                serde_json::to_value(handle).unwrap_or(serde_json::Value::Null),
                serde_json::to_value(&action).unwrap_or(serde_json::Value::Null),
            ],
        );

        let mut state = self.state.lock().unwrap();
        state
            .invoke_expectations
            .pop_front()
            .ok_or(ProtocolError::InvalidHandle {
                reason: agent_protocol::HandleInvalidReason::Expired,
            })?
    }

    async fn revoke(&self, handle: KernelHandle) -> Result<RunSummary, ProtocolError> {
        self.record_call(
            "revoke",
            vec![serde_json::to_value(&handle).unwrap_or(serde_json::Value::Null)],
        );

        let mut state = self.state.lock().unwrap();
        state
            .revoke_expectations
            .pop_front()
            .ok_or(ProtocolError::InvalidHandle {
                reason: agent_protocol::HandleInvalidReason::Expired,
            })?
    }

    async fn query_audit(&self, filter: AuditFilter) -> Result<Vec<LogEntry>, ProtocolError> {
        self.record_call(
            "query_audit",
            vec![serde_json::to_value(&filter).unwrap_or(serde_json::Value::Null)],
        );

        let mut state = self.state.lock().unwrap();
        state
            .query_audit_expectations
            .pop_front()
            .ok_or(ProtocolError::InvalidHandle {
                reason: agent_protocol::HandleInvalidReason::Expired,
            })?
    }
}

impl Clone for MockKernel {
    fn clone(&self) -> Self {
        Self {
            state: Arc::clone(&self.state),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_protocol::{Action, AgentId, RunId};

    #[test]
    fn test_mock_creation() {
        let mock = MockKernel::new();
        assert!(mock.calls().is_empty());
    }

    #[test]
    fn test_mock_clone() {
        let mock = MockKernel::new();
        let cloned = mock.clone();
        assert!(cloned.calls().is_empty());
    }

    #[tokio::test]
    async fn test_mock_spawn() {
        let agent_id = AgentId::new();
        let caps = CapabilitySet::default();
        let expected_handle = KernelHandle::new(agent_id, caps.clone());

        let mock = MockKernel::new().expect_spawn(Ok(expected_handle.clone()));

        let def = AgentDef::new("test-agent");
        let handle = mock.spawn(def, caps).await.unwrap();

        assert_eq!(handle.agent_id, expected_handle.agent_id);
        mock.verify().unwrap();
    }

    #[tokio::test]
    async fn test_mock_invoke() {
        let expected_result = ActionResult::Success(serde_json::json!("test result"));
        let mock = MockKernel::new().expect_invoke(Ok(expected_result.clone()));

        let handle = KernelHandle::new(AgentId::new(), CapabilitySet::default());
        let action = Action::ToolCall {
            tool_id: "test-tool".to_string(),
            params: serde_json::json!({}),
        };
        let result = mock.invoke(&handle, action).await.unwrap();

        // Compare by pattern matching since ActionResult doesn't implement PartialEq
        match (result, expected_result) {
            (ActionResult::Success(got), ActionResult::Success(expected)) => {
                assert_eq!(got, expected);
            }
            _ => panic!("Results don't match"),
        }
        mock.verify().unwrap();
    }

    #[tokio::test]
    async fn test_mock_revoke() {
        let expected_summary = RunSummary {
            run_id: RunId::new(),
            actions_executed: 5,
            final_status: "completed".to_string(),
        };
        let mock = MockKernel::new().expect_revoke(Ok(expected_summary.clone()));

        let handle = KernelHandle::new(AgentId::new(), CapabilitySet::default());
        let summary = mock.revoke(handle).await.unwrap();

        assert_eq!(summary.actions_executed, expected_summary.actions_executed);
        mock.verify().unwrap();
    }

    #[tokio::test]
    async fn test_mock_query_audit() {
        let mock = MockKernel::new().expect_query_audit(Ok(vec![]));

        let filter = AuditFilter::default();
        let entries = mock.query_audit(filter).await.unwrap();

        assert!(entries.is_empty());
        mock.verify().unwrap();
    }

    #[tokio::test]
    async fn test_mock_records_calls() {
        let mock = MockKernel::new().expect_spawn(Ok(KernelHandle::new(
            AgentId::new(),
            CapabilitySet::default(),
        )));

        let def = AgentDef::new("test-agent");
        let caps = CapabilitySet::default();
        let _ = mock.spawn(def.clone(), caps.clone()).await.unwrap();

        let calls = mock.calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].method, "spawn");
    }

    #[tokio::test]
    async fn test_mock_spawn_error() {
        let mock = MockKernel::new().expect_spawn(Err(ProtocolError::ResourceExhausted {
            resource: agent_protocol::ResourceKind::ComputeQuota,
            retry_after: Some(std::time::Duration::from_secs(60)),
        }));

        let def = AgentDef::new("test-agent");
        let caps = CapabilitySet::default();
        let result = mock.spawn(def, caps).await;

        assert!(result.is_err());
        mock.verify().unwrap();
    }

    #[test]
    fn test_mock_verify_fails_when_expectations_not_met() {
        let mock = MockKernel::new()
            .expect_spawn(Ok(KernelHandle::new(
                AgentId::new(),
                CapabilitySet::default(),
            )))
            .expect_invoke(Ok(ActionResult::Success(serde_json::json!(null))));

        // Don't call any methods
        let result = mock.verify();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MockError::ExpectedCallNotMade { method } if method == "spawn"
        ));
    }

    #[test]
    fn test_mock_reset() {
        let mut mock = MockKernel::new().expect_spawn(Ok(KernelHandle::new(
            AgentId::new(),
            CapabilitySet::default(),
        )));

        mock.reset();

        // After reset, verify should pass because expectations were cleared
        mock.verify().unwrap();
    }

    #[tokio::test]
    async fn test_mock_multiple_expectations() {
        let mock = MockKernel::new()
            .expect_spawn(Ok(KernelHandle::new(
                AgentId::new(),
                CapabilitySet::default(),
            )))
            .expect_spawn(Ok(KernelHandle::new(
                AgentId::new(),
                CapabilitySet::default(),
            )));

        let def1 = AgentDef::new("agent-1");
        let def2 = AgentDef::new("agent-2");
        let caps = CapabilitySet::default();

        let _ = mock.spawn(def1, caps.clone()).await.unwrap();
        let _ = mock.spawn(def2, caps).await.unwrap();

        mock.verify().unwrap();
    }

    #[test]
    fn test_mock_error_display() {
        let err = MockError::ExpectedCallNotMade {
            method: "spawn".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "expected method spawn to be called, but it was not"
        );

        let err = MockError::UnexpectedCall {
            method: "invoke".to_string(),
        };
        assert_eq!(err.to_string(), "unexpected call to invoke");

        let err = MockError::VerificationFailed("test".to_string());
        assert_eq!(err.to_string(), "verification failed: test");
    }
}
