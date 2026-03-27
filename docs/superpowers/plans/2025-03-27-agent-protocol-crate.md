# agent-protocol Crate Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the `agent-protocol` crate containing core types (AgentId, RunId, Capability, Action, etc.), error taxonomy (ProtocolError), and the five interface families (AgentLifecycle, Invocation, ContextIO, SignalEvent, ObservabilityHook).

**Architecture:** Single crate with clear module separation: `types.rs` for data structures, `errors.rs` for error taxonomy, `interfaces/` directory for the five interface families, and `lib.rs` as the public API surface. Uses `serde` for serialization, `uuid` for identifiers, `thiserror` for error definitions.

**Tech Stack:** Rust, serde, uuid v4, thiserror, async-trait (for interface traits)

---

## File Structure

```
crates/agent-protocol/
├── Cargo.toml                    # Crate manifest with dependencies
├── src/
│   ├── lib.rs                    # Public exports and crate documentation
│   ├── types.rs                  # Core data structures (AgentId, RunId, etc.)
│   ├── errors.rs                 # ProtocolError enum and variants
│   └── interfaces/
│       ├── mod.rs                # Interface module exports
│       ├── lifecycle.rs          # AgentLifecycle trait (spawn, suspend, resume, terminate)
│       ├── invocation.rs         # Invocation trait (invoke, invoke_stream, cancel)
│       ├── context_io.rs         # ContextIO trait (read, write, search, snapshot, restore)
│       ├── signal_event.rs       # SignalEvent trait (emit, subscribe, interrupt, confirm)
│       └── observability.rs      # ObservabilityHook trait (on_invoke_begin, on_invoke_end, on_error)
└── tests/
    └── types_test.rs             # Unit tests for core types
```

---

## Prerequisites

Before starting this plan, ensure:
1. Workspace `Cargo.toml` exists at repository root
2. `crates/` directory exists
3. You have Rust 1.75+ installed

---

## Task 1: Create Crate Structure and Manifest

**Files:**
- Create: `crates/agent-protocol/Cargo.toml`
- Create: `crates/agent-protocol/src/lib.rs`

- [ ] **Step 1: Create the crate directory structure**

```bash
mkdir -p crates/agent-protocol/src/interfaces
mkdir -p crates/agent-protocol/tests
touch crates/agent-protocol/src/lib.rs
touch crates/agent-protocol/src/types.rs
touch crates/agent-protocol/src/errors.rs
touch crates/agent-protocol/src/interfaces/mod.rs
touch crates/agent-protocol/src/interfaces/lifecycle.rs
touch crates/agent-protocol/src/interfaces/invocation.rs
touch crates/agent-protocol/src/interfaces/context_io.rs
touch crates/agent-protocol/src/interfaces/signal_event.rs
touch crates/agent-protocol/src/interfaces/observability.rs
```

- [ ] **Step 2: Write Cargo.toml with dependencies**

Create `crates/agent-protocol/Cargo.toml`:

```toml
[package]
name = "agent-protocol"
version = "0.1.0"
edition = "2021"
license = "MIT"
description = "Agent Protocol - Open specification for Agent communication"
repository = "https://github.com/0xnicholas/aces"
readme = "../../README.md"

[dependencies]
serde = { version = "1.0", features = ["derive"] }
uuid = { version = "1.6", features = ["v4", "serde"] }
thiserror = "1.0"
async-trait = "0.1"

[dev-dependencies]
tokio = { version = "1.35", features = ["rt-multi-thread", "macros"] }
serde_json = "1.0"
```

- [ ] **Step 3: Create initial lib.rs with crate documentation**

Create `crates/agent-protocol/src/lib.rs`:

```rust
//! Agent Protocol - Open specification for Agent communication
//!
//! This crate defines the core types, interfaces, and error taxonomy for the Agent Protocol.
//! It serves as the contract between Agent Runtimes and governance implementations.
//!
//! # License
//! MIT - See LICENSE-MIT file for details

pub mod types;
pub mod errors;
pub mod interfaces;

// Re-export commonly used types
pub use types::*;
pub use errors::ProtocolError;
pub use interfaces::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crate_compiles() {
        // This test ensures the crate compiles
        assert!(true);
    }
}
```

- [ ] **Step 4: Verify crate compiles**

```bash
cd crates/agent-protocol
cargo check
```

Expected: SUCCESS (no errors, just warnings about unused imports)

- [ ] **Step 5: Commit crate structure**

```bash
cd /Users/nicholasl/Documents/build-whatever/aces
git add crates/agent-protocol/Cargo.toml crates/agent-protocol/src/lib.rs
git commit -m "chore(agent-protocol): create crate structure with manifest and lib.rs

- Add Cargo.toml with serde, uuid, thiserror, async-trait dependencies
- Create initial lib.rs with module structure
- Set up interfaces/ subdirectory for five interface families
- MIT licensed as per Protocol specification"
```

---

## Task 2: Implement Core Types (types.rs)

**Files:**
- Create: `crates/agent-protocol/src/types.rs`
- Test: `crates/agent-protocol/tests/types_test.rs`

- [ ] **Step 1: Write test for AgentId**

Create `crates/agent-protocol/tests/types_test.rs`:

```rust
use agent_protocol::*;
use uuid::Uuid;

#[test]
fn test_agent_id_new_creates_unique_id() {
    let id1 = AgentId::new();
    let id2 = AgentId::new();
    assert_ne!(id1, id2);
}

#[test]
fn test_agent_id_from_uuid() {
    let uuid = Uuid::new_v4();
    let agent_id = AgentId::from(uuid);
    assert_eq!(agent_id.as_uuid(), &uuid);
}

#[test]
fn test_agent_id_display() {
    let uuid = Uuid::new_v4();
    let agent_id = AgentId::from(uuid);
    let display = format!("{}", agent_id);
    assert!(display.contains(&uuid.to_string()));
}

#[test]
fn test_run_id_propagation() {
    let run_id = RunId::new();
    let span_id = SpanId::new();
    
    // RunId and SpanId should be different types
    let _ = format!("{}", run_id);
    let _ = format!("{}", span_id);
}

#[test]
fn test_capability_tool_read() {
    let cap = Capability::ToolRead { tool_id: "test_tool".to_string() };
    match cap {
        Capability::ToolRead { tool_id } => assert_eq!(tool_id, "test_tool"),
        _ => panic!("Wrong capability variant"),
    }
}

#[test]
fn test_capability_set_empty() {
    let caps = CapabilitySet::empty();
    assert!(caps.is_empty());
    assert_eq!(caps.len(), 0);
}

#[test]
fn test_capability_set_add_and_contains() {
    let mut caps = CapabilitySet::empty();
    let cap = Capability::ToolRead { tool_id: "test".to_string() };
    caps.add(cap.clone());
    
    assert!(caps.contains(&cap));
    assert_eq!(caps.len(), 1);
}

#[test]
fn test_action_tool_invoke() {
    let action = Action::InvokeTool {
        tool_id: "calculator".to_string(),
        params: vec![1, 2, 3],
        idempotency_key: Some("key123".to_string()),
    };
    
    match action {
        Action::InvokeTool { tool_id, .. } => assert_eq!(tool_id, "calculator"),
        _ => panic!("Wrong action variant"),
    }
}

#[test]
fn test_kernel_handle_creation() {
    use std::time::{SystemTime, Duration};
    
    let agent_id = AgentId::new();
    let handle = KernelHandle::new(agent_id, None);
    
    assert_eq!(handle.agent_id(), agent_id);
    assert!(handle.expires_at().is_none());
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd crates/agent-protocol
cargo test --test types_test
```

Expected: FAIL with "error[E0433]: failed to resolve: use of undeclared crate or module `agent_protocol`" and various "not found" errors for types

- [ ] **Step 3: Implement AgentId, RunId, SpanId**

Create/modify `crates/agent-protocol/src/types.rs`:

```rust
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Unique identifier for an Agent definition (the blueprint)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(Uuid);

impl AgentId {
    /// Generate a new unique AgentId
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Access the underlying UUID
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for AgentId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for AgentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for AgentId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

/// Unique identifier for a top-level Agent invocation
/// Propagated through the entire call tree, including sub-Agent calls
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RunId(Uuid);

impl RunId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for RunId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for RunId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for RunId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

/// Unique identifier for one step within a run
/// Forms a tree with parent_span_id linking child to parent
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SpanId(Uuid);

impl SpanId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for SpanId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for SpanId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for SpanId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

/// Opaque, unforgeable token inside a KernelHandle
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HandleId(Vec<u8>);

impl HandleId {
    pub fn new_random() -> Self {
        use uuid::Uuid;
        // Use UUID as base, then add additional random bytes for 256-bit security
        let uuid_bytes = Uuid::new_v4().as_bytes().to_vec();
        let additional = Uuid::new_v4().as_bytes().to_vec();
        let mut combined = uuid_bytes;
        combined.extend_from_slice(&additional);
        Self(combined)
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl Default for HandleId {
    fn default() -> Self {
        Self::new_random()
    }
}
```

- [ ] **Step 4: Implement Capability enum and CapabilitySet**

Add to `crates/agent-protocol/src/types.rs`:

```rust
/// Context scope for memory operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ContextScope {
    /// Current run only, this Agent only
    Session,
    /// Across runs, this Agent only
    Agent,
    /// Across runs, multiple Agents
    Shared,
}

/// An explicit, unforgeable token granting an Agent the right to perform a specific class of action
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Capability {
    /// Read tool definition/metadata
    ToolRead { tool_id: String },
    /// Write/modify tool configuration
    ToolWrite { tool_id: String },
    /// Execute tool
    ToolExec { tool_id: String },
    /// Read from context/memory
    MemoryRead { scope: ContextScope },
    /// Write to context/memory
    MemoryWrite { scope: ContextScope },
    /// Call LLM models
    LlmCall { model_family: Option<String> },
    /// Call other Agents
    AgentCall { target_agent_id: Option<AgentId> },
    /// Make HTTP requests
    HttpFetch { domain_allowlist: Vec<String> },
    /// Emit notifications
    Notify { channel: String },
}

/// The complete set of capabilities bound to an Agent instance at spawn time
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CapabilitySet {
    capabilities: Vec<Capability>,
}

impl CapabilitySet {
    /// Create an empty capability set
    pub fn empty() -> Self {
        Self {
            capabilities: Vec::new(),
        }
    }

    /// Add a capability to the set
    pub fn add(&mut self, cap: Capability) {
        self.capabilities.push(cap);
    }

    /// Check if the set contains a specific capability
    pub fn contains(&self, cap: &Capability) -> bool {
        self.capabilities.contains(cap)
    }

    /// Returns true if the set contains no capabilities
    pub fn is_empty(&self) -> bool {
        self.capabilities.is_empty()
    }

    /// Returns the number of capabilities in the set
    pub fn len(&self) -> usize {
        self.capabilities.len()
    }

    /// Get an iterator over capabilities
    pub fn iter(&self) -> impl Iterator<Item = &Capability> {
        self.capabilities.iter()
    }

    /// Compute intersection with another CapabilitySet
    pub fn intersection(&self, other: &CapabilitySet) -> CapabilitySet {
        let mut result = CapabilitySet::empty();
        for cap in &self.capabilities {
            if other.contains(cap) {
                result.add(cap.clone());
            }
        }
        result
    }
}

impl From<Vec<Capability>> for CapabilitySet {
    fn from(caps: Vec<Capability>) -> Self {
        Self { capabilities: caps }
    }
}
```

- [ ] **Step 5: Implement Action enum and related types**

Add to `crates/agent-protocol/src/types.rs`:

```rust
/// A typed request produced by a Runtime and submitted via invoke
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Action {
    /// Invoke a tool
    InvokeTool {
        tool_id: String,
        params: Vec<u8>,
        idempotency_key: Option<String>,
    },
    /// LLM inference request
    LlmInfer {
        model: String,
        messages: Vec<Message>,
        params: InferParams,
    },
    /// Read from context
    MemoryRead {
        key: ContextKey,
        scope: ContextScope,
    },
    /// Write to context
    MemoryWrite {
        key: ContextKey,
        value: Vec<u8>,
        scope: ContextScope,
    },
    /// Search context
    MemorySearch {
        query: SemanticQuery,
        scope: ContextScope,
    },
    /// Call another Agent
    CallAgent {
        target_id: AgentId,
        payload: Vec<u8>,
        caps_hint: Option<CapabilitySet>,
    },
    /// Emit notification
    Notify {
        channel: String,
        body: Vec<u8>,
    },
}

/// Message for LLM inference
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

/// Role in a conversation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

/// Parameters for LLM inference
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct InferParams {
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub top_p: Option<f32>,
}

/// Key for context operations
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContextKey(String);

impl ContextKey {
    pub fn new(key: impl Into<String>) -> Self {
        Self(key.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for ContextKey {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for ContextKey {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Semantic query for context search
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SemanticQuery {
    pub text: String,
    pub embedding: Option<Vec<f32>>,
}

impl SemanticQuery {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            embedding: None,
        }
    }
}
```

- [ ] **Step 6: Implement KernelHandle**

Add to `crates/agent-protocol/src/types.rs`:

```rust
use std::time::{SystemTime, Duration};

/// Opaque reference to a spawned Agent instance
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KernelHandle {
    agent_id: AgentId,
    handle_id: HandleId,
    issued_at: SystemTime,
    expires_at: Option<SystemTime>,
}

impl KernelHandle {
    /// Create a new KernelHandle
    pub fn new(agent_id: AgentId, expires_at: Option<SystemTime>) -> Self {
        Self {
            agent_id,
            handle_id: HandleId::new_random(),
            issued_at: SystemTime::now(),
            expires_at,
        }
    }

    /// Get the AgentId associated with this handle
    pub fn agent_id(&self) -> AgentId {
        self.agent_id
    }

    /// Check if the handle is expired
    pub fn is_expired(&self) -> bool {
        match self.expires_at {
            Some(expiry) => SystemTime::now() > expiry,
            None => false,
        }
    }

    /// Get the expiration time, if any
    pub fn expires_at(&self) -> Option<SystemTime> {
        self.expires_at
    }

    /// Get the handle ID (internal use only)
    pub fn handle_id(&self) -> &HandleId {
        &self.handle_id
    }
}

/// Static definition of an Agent
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentDef {
    pub agent_id: AgentId,
    pub name: String,
    pub description: Option<String>,
    pub model: Option<String>,
    pub max_steps: Option<u32>,
    pub timeout: Option<Duration>,
}

impl AgentDef {
    pub fn new(agent_id: AgentId, name: impl Into<String>) -> Self {
        Self {
            agent_id,
            name: name.into(),
            description: None,
            model: None,
            max_steps: None,
            timeout: None,
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }
}

/// Result of an action execution
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActionResult {
    pub run_id: RunId,
    pub span_id: SpanId,
    pub status: ActionStatus,
    pub payload: Option<Vec<u8>>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionStatus {
    Ok,
    Err,
    Pending,
}
```

- [ ] **Step 7: Update lib.rs to export types module**

Ensure `crates/agent-protocol/src/lib.rs` contains:

```rust
pub mod types;
pub use types::*;
```

- [ ] **Step 8: Run tests to verify they pass**

```bash
cd crates/agent-protocol
cargo test --test types_test
```

Expected: PASS - All 10 tests should pass

- [ ] **Step 9: Commit types implementation**

```bash
cd /Users/nicholasl/Documents/build-whatever/aces
git add crates/agent-protocol/src/types.rs crates/agent-protocol/tests/types_test.rs
git commit -m "feat(agent-protocol): implement core types

- Add identity types: AgentId, RunId, SpanId, HandleId
- Add Capability enum with 8 variants (Tool*, Memory*, LlmCall, AgentCall, HttpFetch, Notify)
- Add CapabilitySet with intersection operation (ADR-002)
- Add Action enum with 7 variants (InvokeTool, LlmInfer, Memory*, CallAgent, Notify)
- Add supporting types: Message, ContextKey, SemanticQuery, AgentDef
- Add KernelHandle with expiration support
- Add ActionResult and ActionStatus
- Comprehensive unit tests for all types"
```

---

## Task 3: Implement Error Taxonomy (errors.rs)

**Files:**
- Create: `crates/agent-protocol/src/errors.rs`
- Test: `crates/agent-protocol/tests/error_test.rs`

- [ ] **Step 1: Write test for ProtocolError variants**

Create `crates/agent-protocol/tests/error_test.rs`:

```rust
use agent_protocol::ProtocolError;
use agent_protocol::{Action, Capability, AgentId, RunId};

#[test]
fn test_policy_violation_error() {
    let action = Action::InvokeTool {
        tool_id: "test".to_string(),
        params: vec![],
        idempotency_key: None,
    };
    let missing_cap = Capability::ToolExec { tool_id: "test".to_string() };
    let agent_id = AgentId::new();
    
    let err = ProtocolError::PolicyViolation {
        action,
        missing_cap,
        agent_id,
    };
    
    assert!(err.to_string().contains("PolicyViolation"));
}

#[test]
fn test_resource_exhausted_error() {
    use std::time::Duration;
    
    let err = ProtocolError::ResourceExhausted {
        resource: agent_protocol::ResourceKind::LlmConcurrency,
        retry_after: Some(Duration::from_secs(60)),
    };
    
    match err {
        ProtocolError::ResourceExhausted { resource, retry_after } => {
            assert!(matches!(resource, agent_protocol::ResourceKind::LlmConcurrency));
            assert_eq!(retry_after, Some(Duration::from_secs(60)));
        }
        _ => panic!("Wrong error variant"),
    }
}

#[test]
fn test_invalid_handle_error() {
    let err = ProtocolError::InvalidHandle {
        reason: agent_protocol::HandleInvalidReason::Expired,
    };
    
    assert!(err.to_string().contains("InvalidHandle"));
}

#[test]
fn test_error_ordering_priority() {
    // InvalidHandle should be highest priority (checked first)
    // PolicyViolation second
    // ResourceExhausted third
    use agent_protocol::error_priority;
    
    assert!(error_priority(&ProtocolError::InvalidHandle { 
        reason: agent_protocol::HandleInvalidReason::Revoked 
    }) > error_priority(&ProtocolError::PolicyViolation { 
        action: Action::InvokeTool { tool_id: "x".to_string(), params: vec![], idempotency_key: None },
        missing_cap: Capability::ToolExec { tool_id: "x".to_string() },
        agent_id: AgentId::new(),
    }));
}

#[test]
fn test_cancelled_error() {
    let run_id = RunId::new();
    let err = ProtocolError::Cancelled { run_id };
    
    match err {
        ProtocolError::Cancelled { run_id: r } => assert_eq!(r, run_id),
        _ => panic!("Wrong error variant"),
    }
}

#[test]
fn test_timeout_error() {
    use std::time::Duration;
    
    let action = Action::InvokeTool {
        tool_id: "slow_tool".to_string(),
        params: vec![],
        idempotency_key: None,
    };
    
    let err = ProtocolError::Timeout {
        action,
        limit: Duration::from_secs(30),
    };
    
    assert!(err.to_string().contains("Timeout"));
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd crates/agent-protocol
cargo test --test error_test
```

Expected: FAIL - ProtocolError not defined, ResourceKind not defined, etc.

- [ ] **Step 3: Implement ResourceKind and HandleInvalidReason enums**

Create/modify `crates/agent-protocol/src/errors.rs`:

```rust
use crate::types::{Action, Capability, AgentId, RunId, SpanId};
use thiserror::Error;
use std::time::Duration;

/// Types of resources that can be exhausted
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceKind {
    LlmConcurrency,
    ToolCallRate,
    ContextBudget,
    ComputeQuota,
}

/// Reasons a handle might be invalid
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HandleInvalidReason {
    Expired,
    Revoked,
    Unrecognised,
}

/// Human decision for interrupt confirmation
#[derive(Debug, Clone, PartialEq)]
pub enum HumanDecision {
    Approve,
    ApproveWithModification { modified_action: Action },
    Reject { reason: String },
}

/// Opaque token returned by interrupt
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InterruptToken(String);

impl InterruptToken {
    pub fn new() -> Self {
        use uuid::Uuid;
        Self(Uuid::new_v4().to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for InterruptToken {
    fn default() -> Self {
        Self::new()
    }
}
```

- [ ] **Step 4: Implement ProtocolError enum with all 8 variants**

Add to `crates/agent-protocol/src/errors.rs`:

```rust
/// All errors returned across the Protocol boundary
#[derive(Error, Debug, Clone, PartialEq)]
pub enum ProtocolError {
    /// Capability check failed; action not permitted
    #[error("PolicyViolation: action not permitted (missing: {missing_cap:?})")]
    PolicyViolation {
        action: Action,
        missing_cap: Capability,
        agent_id: AgentId,
    },

    /// Scheduler budget exhausted; retry after backoff
    #[error("ResourceExhausted: {resource:?}, retry after: {retry_after:?}")]
    ResourceExhausted {
        resource: ResourceKind,
        retry_after: Option<Duration>,
    },

    /// HITL confirmation pending; call confirm() to resume
    #[error("Interrupted: confirmation required (token: {token:?}, rejected: {rejected})")]
    Interrupted {
        token: InterruptToken,
        rejected: bool,
    },

    /// Action exceeded the configured time limit
    #[error("Timeout: action exceeded limit of {limit:?}")]
    Timeout {
        action: Action,
        limit: Duration,
    },

    /// Context window budget exceeded
    #[error("ContextOverflow: {current} tokens exceeds limit of {limit}")]
    ContextOverflow {
        current: u64,
        limit: u64,
    },

    /// Cancelled by an upstream cancel() call
    #[error("Cancelled: run {run_id} was cancelled")]
    Cancelled {
        run_id: RunId,
    },

    /// Audit chain verification failed
    #[error("AuditIntegrityError: chain broken at seq {seq}")]
    AuditIntegrityError {
        seq: u64,
        expected: Vec<u8>,
        actual: Vec<u8>,
    },

    /// KernelHandle is expired or revoked
    #[error("InvalidHandle: {reason:?}")]
    InvalidHandle {
        reason: HandleInvalidReason,
    },

    /// Protocol violation (malformed request)
    #[error("ProtocolViolation: {detail}")]
    ProtocolViolation {
        detail: String,
    },
}

/// Priority ordering for errors when multiple could apply
/// Higher number = check first
pub fn error_priority(err: &ProtocolError) -> u8 {
    match err {
        ProtocolError::InvalidHandle { .. } => 4,      // Check first
        ProtocolError::PolicyViolation { .. } => 3,    // Check second
        ProtocolError::ResourceExhausted { .. } => 2,  // Check third
        _ => 1,                                         // All others last
    }
}

/// Compare two errors by priority (for sorting)
pub fn compare_by_priority(a: &ProtocolError, b: &ProtocolError) -> std::cmp::Ordering {
    error_priority(b).cmp(&error_priority(a)) // Reverse for descending order
}
```

- [ ] **Step 5: Update lib.rs to export errors module**

Ensure `crates/agent-protocol/src/lib.rs` contains:

```rust
pub mod errors;
pub use errors::*;

// Also add these specific exports for convenience
pub use errors::{
    ProtocolError, ResourceKind, HandleInvalidReason,
    HumanDecision, InterruptToken, error_priority, compare_by_priority
};
```

- [ ] **Step 6: Run tests to verify they pass**

```bash
cd crates/agent-protocol
cargo test --test error_test
```

Expected: PASS - All 6 tests should pass

- [ ] **Step 7: Commit error taxonomy**

```bash
cd /Users/nicholasl/Documents/build-whatever/aces
git add crates/agent-protocol/src/errors.rs crates/agent-protocol/tests/error_test.rs
git commit -m "feat(agent-protocol): implement error taxonomy (ProtocolError)

- Add ResourceKind enum (LlmConcurrency, ToolCallRate, ContextBudget, ComputeQuota)
- Add HandleInvalidReason enum (Expired, Revoked, Unrecognised)
- Add HumanDecision and InterruptToken for HITL flow
- Implement ProtocolError with all 8 variants:
  PolicyViolation, ResourceExhausted, Interrupted, Timeout,
  ContextOverflow, Cancelled, AuditIntegrityError, InvalidHandle
- Add error_priority() function for error ordering (ADR-001 constraint)
- Use thiserror for automatic Display impl
- Comprehensive tests for all error variants"
```

---

## Task 4: Implement Interface Traits

**Files:**
- Create: `crates/agent-protocol/src/interfaces/mod.rs`
- Create: `crates/agent-protocol/src/interfaces/lifecycle.rs`
- Create: `crates/agent-protocol/src/interfaces/invocation.rs`
- Create: `crates/agent-protocol/src/interfaces/context_io.rs`
- Create: `crates/agent-protocol/src/interfaces/signal_event.rs`
- Create: `crates/agent-protocol/src/interfaces/observability.rs`
- Test: `crates/agent-protocol/tests/interface_test.rs`

- [ ] **Step 1: Create interfaces module exports**

Create `crates/agent-protocol/src/interfaces/mod.rs`:

```rust
//! Interface families for the Agent Protocol
//!
//! The Protocol defines five interface families:
//! - AgentLifecycle: manage Agent existence (fork/exec/kill analogy)
//! - Invocation: execute actions (read/write analogy)
//! - ContextIO: memory access (mmap/lseek analogy)
//! - SignalEvent: events and HITL (signal/wait analogy)
//! - ObservabilityHook: mandatory tracing (ptrace analogy)

pub mod lifecycle;
pub mod invocation;
pub mod context_io;
pub mod signal_event;
pub mod observability;

pub use lifecycle::AgentLifecycle;
pub use invocation::Invocation;
pub use context_io::ContextIO;
pub use signal_event::SignalEvent;
pub use observability::ObservabilityHook;
```

- [ ] **Step 2: Implement AgentLifecycle trait**

Create `crates/agent-protocol/src/interfaces/lifecycle.rs`:

```rust
use crate::types::{AgentDef, CapabilitySet, KernelHandle, RunSummary};
use crate::errors::ProtocolError;
use async_trait::async_trait;

/// Agent Lifecycle interface - POSIX analogy: fork/exec/kill
#[async_trait]
pub trait AgentLifecycle {
    /// Spawn a new Agent instance bound to the given capabilities
    /// 
    /// # Arguments
    /// * `def` - Agent definition (blueprint)
    /// * `caps` - CapabilitySet bound at spawn time (immutable thereafter)
    ///
    /// # Returns
    /// * `KernelHandle` - Opaque handle for subsequent operations
    async fn spawn(
        &self,
        def: AgentDef,
        caps: CapabilitySet,
    ) -> Result<KernelHandle, ProtocolError>;

    /// Suspend execution and return a Checkpoint
    /// 
    /// In-flight actions are allowed to complete before suspension.
    /// Audit log is flushed before returning.
    async fn suspend(
        &self,
        handle: &KernelHandle,
    ) -> Result<Checkpoint, ProtocolError>;

    /// Restore an Agent from a Checkpoint
    /// 
    /// The resumed instance has the same AgentId and CapabilitySet.
    /// A new HandleId is issued.
    async fn resume(
        &self,
        checkpoint: Checkpoint,
    ) -> Result<KernelHandle, ProtocolError>;

    /// Permanently revoke the handle
    /// 
    /// In-flight actions receive Cancelled.
    /// A Revocation entry is written to the audit log.
    async fn terminate(
        &self,
        handle: KernelHandle,
        reason: TerminationReason,
    ) -> Result<RunSummary, ProtocolError>;
}

/// Checkpoint for suspend/resume operations
#[derive(Debug, Clone)]
pub struct Checkpoint {
    pub state: Vec<u8>,
    pub agent_id: crate::types::AgentId,
    pub caps: CapabilitySet,
}

/// Reasons for termination
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminationReason {
    UserRequested,
    Timeout,
    PolicyViolation,
    Error,
}

/// Summary of a completed run
#[derive(Debug, Clone)]
pub struct RunSummary {
    pub agent_id: crate::types::AgentId,
    pub run_id: crate::types::RunId,
    pub actions_taken: u64,
    pub started_at: std::time::SystemTime,
    pub ended_at: std::time::SystemTime,
    pub termination: TerminationReason,
}
```

- [ ] **Step 3: Implement Invocation trait**

Create `crates/agent-protocol/src/interfaces/invocation.rs`:

```rust
use crate::types::{KernelHandle, Action, ActionResult, RunId, SpanId};
use crate::errors::ProtocolError;
use async_trait::async_trait;
use futures::stream::Stream;

/// Invocation interface - POSIX analogy: read/write
/// 
/// This is THE single interception point for all Agent actions.
#[async_trait]
pub trait Invocation {
    /// Execute an action on behalf of the Agent identified by handle
    /// 
    /// This is the single critical path:
    /// check → schedule → sandbox → audit → return result
    async fn invoke(
        &self,
        handle: &KernelHandle,
        action: Action,
        run_id: RunId,
        span_id: SpanId,
        parent_span_id: Option<SpanId>,
    ) -> Result<ActionResult, ProtocolError>;

    /// Execute action with streaming output (primarily for LLM inference)
    /// 
    /// Audit entry written when stream opens (status: Pending).
    /// Completion entry written when stream closes.
    async fn invoke_stream(
        &self,
        handle: &KernelHandle,
        action: Action,
        run_id: RunId,
        span_id: SpanId,
        parent_span_id: Option<SpanId>,
    ) -> Result<Box<dyn Stream<Item = Result<Chunk, ProtocolError>> + Send>, ProtocolError>;

    /// Cancel all in-flight operations for a run
    /// 
    /// Cancellation MUST propagate to the full call tree.
    /// Each cancelled operation receives Cancelled error.
    async fn cancel(&self, run_id: RunId) -> Result<(), ProtocolError>;
}

/// Chunk for streaming responses
#[derive(Debug, Clone)]
pub struct Chunk {
    pub data: Vec<u8>,
    pub is_final: bool,
}
```

- [ ] **Step 4: Implement ContextIO trait**

Create `crates/agent-protocol/src/interfaces/context_io.rs`:

```rust
use crate::types::{KernelHandle, ContextKey, ContextScope, SemanticQuery};
use crate::errors::ProtocolError;
use async_trait::async_trait;

/// ContextIO interface - POSIX analogy: mmap/lseek
/// 
/// Standardized read and write access to Agent memory.
#[async_trait]
pub trait ContextIO {
    /// Read value from context
    async fn context_read(
        &self,
        handle: &KernelHandle,
        key: ContextKey,
        scope: ContextScope,
    ) -> Result<ContextValue, ProtocolError>;

    /// Write value to context
    async fn context_write(
        &self,
        handle: &KernelHandle,
        key: ContextKey,
        value: ContextValue,
        scope: ContextScope,
    ) -> Result<(), ProtocolError>;

    /// Search context semantically
    async fn context_search(
        &self,
        handle: &KernelHandle,
        query: SemanticQuery,
        scope: ContextScope,
        limit: u32,
    ) -> Result<Vec<ContextValue>, ProtocolError>;

    /// Create a snapshot of current context state
    async fn snapshot(
        &self,
        handle: &KernelHandle,
    ) -> Result<ContextSnapshot, ProtocolError>;

    /// Restore context from snapshot
    async fn restore(
        &self,
        handle: &KernelHandle,
        snapshot: ContextSnapshot,
    ) -> Result<(), ProtocolError>;
}

/// Value stored in context
#[derive(Debug, Clone, PartialEq)]
pub struct ContextValue {
    pub data: Vec<u8>,
    pub content_type: Option<String>,
}

/// Snapshot of context state
#[derive(Debug, Clone)]
pub struct ContextSnapshot {
    pub state: Vec<u8>,
    pub timestamp: std::time::SystemTime,
}
```

- [ ] **Step 5: Implement SignalEvent trait**

Create `crates/agent-protocol/src/interfaces/signal_event.rs`:

```rust
use crate::types::KernelHandle;
use crate::errors::{ProtocolError, InterruptToken, HumanDecision};
use async_trait::async_trait;
use futures::stream::Stream;

/// SignalEvent interface - POSIX analogy: signal/wait
/// 
/// Event emission, subscription, and human-in-the-loop interruption.
#[async_trait]
pub trait SignalEvent {
    /// Emit an event
    async fn emit(
        &self,
        handle: &KernelHandle,
        event: AgentEvent,
    ) -> Result<(), ProtocolError>;

    /// Subscribe to events matching filter
    async fn subscribe(
        &self,
        handle: &KernelHandle,
        filter: EventFilter,
    ) -> Result<Box<dyn Stream<Item = AgentEvent> + Send>, ProtocolError>;

    /// Interrupt Agent for human confirmation
    /// 
    /// Places Agent in Interrupted state. Returns token for confirm/reject.
    async fn interrupt(
        &self,
        handle: &KernelHandle,
        signal: AgentSignal,
        reason: String,
    ) -> Result<InterruptToken, ProtocolError>;

    /// Confirm or reject an interrupted action
    async fn confirm(
        &self,
        token: InterruptToken,
        decision: HumanDecision,
    ) -> Result<(), ProtocolError>;
}

/// Agent event
#[derive(Debug, Clone, PartialEq)]
pub struct AgentEvent {
    pub event_type: String,
    pub payload: Vec<u8>,
    pub timestamp: std::time::SystemTime,
}

/// Filter for event subscription
#[derive(Debug, Clone, Default)]
pub struct EventFilter {
    pub event_types: Vec<String>,
    pub sources: Vec<String>,
}

/// Signal for interrupt
#[derive(Debug, Clone, PartialEq)]
pub enum AgentSignal {
    HumanConfirmationRequired { risk_level: RiskLevel },
    ExternalEvent { source: String, payload: Vec<u8> },
    PolicyAlert { policy_id: String, detail: String },
}

/// Risk level for human confirmation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}
```

- [ ] **Step 6: Implement ObservabilityHook trait**

Create `crates/agent-protocol/src/interfaces/observability.rs`:

```rust
use crate::types::{KernelHandle, Action, ActionResult, CapabilitySet, RunId, SpanId};

/// ObservabilityHook interface - POSIX analogy: ptrace
/// 
/// Mandatory trace emission on every invocation.
/// This interface is NOT optional - hooks cannot be disabled.
pub trait ObservabilityHook {
    /// Called before any capability check
    fn on_invoke_begin(
        &self,
        run_id: RunId,
        span_id: SpanId,
        parent_span_id: Option<SpanId>,
        action: &Action,
        caps_scope: &CapabilitySet,
    ) -> SpanContext;

    /// Called after audit entry is written, before result returned
    fn on_invoke_end(
        &self,
        ctx: SpanContext,
        result: &ActionResult,
    );

    /// Called for lifecycle state changes
    fn on_state_change(
        &self,
        handle: &KernelHandle,
        event: LifecycleEvent,
    );

    /// Called for every error
    fn on_error(
        &self,
        run_id: RunId,
        span_id: SpanId,
        error: &crate::errors::ProtocolError,
    );
}

/// Context passed through span lifecycle
#[derive(Debug, Clone)]
pub struct SpanContext {
    pub run_id: RunId,
    pub span_id: SpanId,
    pub start_time: std::time::Instant,
}

/// Lifecycle events
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleEvent {
    Spawned,
    Suspended,
    Resumed,
    Terminated,
    Revoked,
}
```

- [ ] **Step 7: Write interface tests**

Create `crates/agent-protocol/tests/interface_test.rs`:

```rust
use agent_protocol::interfaces::*;
use agent_protocol::*;
use async_trait::async_trait;

// Mock implementations to verify traits can be implemented

struct MockLifecycle;

#[async_trait]
impl AgentLifecycle for MockLifecycle {
    async fn spawn(
        &self,
        _def: AgentDef,
        _caps: CapabilitySet,
    ) -> Result<KernelHandle, ProtocolError> {
        unimplemented!()
    }

    async fn suspend(&self, _handle: &KernelHandle) -> Result<Checkpoint, ProtocolError> {
        unimplemented!()
    }

    async fn resume(&self, _checkpoint: Checkpoint) -> Result<KernelHandle, ProtocolError> {
        unimplemented!()
    }

    async fn terminate(
        &self,
        _handle: KernelHandle,
        _reason: TerminationReason,
    ) -> Result<RunSummary, ProtocolError> {
        unimplemented!()
    }
}

#[test]
fn test_lifecycle_trait_is_object_safe() {
    let _: Box<dyn AgentLifecycle> = Box::new(MockLifecycle);
}

#[test]
fn test_observability_hook_trait_is_object_safe() {
    struct MockHook;
    impl ObservabilityHook for MockHook {
        fn on_invoke_begin(
            &self,
            _run_id: RunId,
            _span_id: SpanId,
            _parent_span_id: Option<SpanId>,
            _action: &Action,
            _caps_scope: &CapabilitySet,
        ) -> SpanContext {
            unimplemented!()
        }

        fn on_invoke_end(&self, _ctx: SpanContext, _result: &ActionResult) {}

        fn on_state_change(&self, _handle: &KernelHandle, _event: LifecycleEvent) {}

        fn on_error(&self, _run_id: RunId, _span_id: SpanId, _error: &ProtocolError) {}
    }

    let _: Box<dyn ObservabilityHook> = Box::new(MockHook);
}

#[test]
fn test_risk_level_enum() {
    assert_ne!(RiskLevel::Low, RiskLevel::Critical);
}

#[test]
fn test_lifecycle_event_enum() {
    use LifecycleEvent::*;
    assert!(matches!(Spawned, Spawned));
}
```

- [ ] **Step 8: Update Cargo.toml to add futures dependency**

Add to `crates/agent-protocol/Cargo.toml` dependencies:

```toml
[dependencies]
# ... existing deps ...
futures = "0.3"
```

- [ ] **Step 9: Run all tests**

```bash
cd crates/agent-protocol
cargo test
```

Expected: PASS - All tests should pass (types, errors, interfaces)

- [ ] **Step 10: Commit interface implementations**

```bash
cd /Users/nicholasl/Documents/build-whatever/aces
git add crates/agent-protocol/src/interfaces/ crates/agent-protocol/tests/interface_test.rs
git commit -m "feat(agent-protocol): implement five interface families

- AgentLifecycle: spawn, suspend, resume, terminate
- Invocation: invoke, invoke_stream, cancel (THE single interception point)
- ContextIO: context_read, context_write, context_search, snapshot, restore
- SignalEvent: emit, subscribe, interrupt, confirm (HITL support)
- ObservabilityHook: on_invoke_begin, on_invoke_end, on_state_change, on_error

All traits use async-trait for async support.
Comprehensive supporting types: Checkpoint, RunSummary, ContextSnapshot,
AgentEvent, EventFilter, AgentSignal, SpanContext, etc.

Tests verify trait object safety and basic functionality."
```

---

## Task 5: Final Integration and Documentation

**Files:**
- Modify: `crates/agent-protocol/src/lib.rs`

- [ ] **Step 1: Add comprehensive crate documentation**

Update `crates/agent-protocol/src/lib.rs`:

```rust
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
//! let mut caps = CapabilitySet::empty();
//! caps.add(Capability::ToolRead { tool_id: "calculator".to_string() });
//!
//! // Create an action
//! let action = Action::InvokeTool {
//!     tool_id: "calculator".to_string(),
//!     params: vec![1, 2, 3],
//!     idempotency_key: None,
//! };
//! ```

pub mod types;
pub mod errors;
pub mod interfaces;

// Re-export commonly used types
pub use types::*;

// Re-export error types
pub use errors::{
    ProtocolError, ResourceKind, HandleInvalidReason,
    HumanDecision, InterruptToken, error_priority, compare_by_priority
};

// Re-export interfaces
pub use interfaces::{
    AgentLifecycle, Invocation, ContextIO, SignalEvent, ObservabilityHook,
    Checkpoint, RunSummary, TerminationReason,
    Chunk,
    ContextValue, ContextSnapshot,
    AgentEvent, EventFilter, AgentSignal, RiskLevel,
    SpanContext, LifecycleEvent,
};

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
```

- [ ] **Step 2: Run final test suite**

```bash
cd crates/agent-protocol
cargo test --lib
cargo test --tests
cargo doc --no-deps
```

Expected: All tests pass, docs build without warnings

- [ ] **Step 3: Verify crate can be built in release mode**

```bash
cd crates/agent-protocol
cargo build --release
```

Expected: SUCCESS

- [ ] **Step 4: Add workspace dependency reference**

Ensure root `Cargo.toml` includes agent-protocol in workspace members:

```toml
[workspace]
members = [
    "crates/agent-protocol",
    # Future crates will be added here
]
```

- [ ] **Step 5: Commit final integration**

```bash
cd /Users/nicholasl/Documents/build-whatever/aces
git add crates/agent-protocol/src/lib.rs Cargo.toml
git commit -m "docs(agent-protocol): add comprehensive crate documentation

- Complete rustdoc for lib.rs with overview and examples
- Document all five interface families
- Explain six semantic constraints
- Show architecture diagram in text
- Add example code in documentation
- Export all public types through lib.rs
- Ensure crate compiles in release mode"
```

---

## Summary

This plan implements a complete `agent-protocol` crate with:

✅ **Core Types**: AgentId, RunId, SpanId, HandleId, Capability, CapabilitySet, Action, KernelHandle  
✅ **Error Taxonomy**: ProtocolError with 8 variants, ResourceKind, HandleInvalidReason  
✅ **Five Interface Families**: Lifecycle, Invocation, ContextIO, SignalEvent, ObservabilityHook  
✅ **Comprehensive Tests**: Unit tests for all major components  
✅ **Full Documentation**: Rustdoc with examples and architecture overview

**Dependencies**: serde, uuid, thiserror, async-trait, futures

**License**: MIT (as per Protocol specification)

---

## Next Steps After This Plan

1. **Review this plan** - Check for completeness and accuracy
2. **Execute the plan** - Use subagent-driven-development or executing-plans skill
3. **Create next plan** - kernel-api crate (public API surface with 4 methods)
4. **Continue chain** - kernel-core, permission-engine, audit-log, scheduler, sandbox
