//! Core types for the Agent Protocol

use crate::errors::ProtocolError;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt;
use uuid::Uuid;

/// Unique identifier for an Agent
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(pub Uuid);

impl AgentId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
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

/// Unique identifier for a specific run/invocation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RunId(pub Uuid);

impl RunId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
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

/// Unique identifier for a span in a trace
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SpanId(pub Uuid);

impl SpanId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
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

/// Opaque, unforgeable token inside a KernelHandle
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HandleId(pub Uuid);

impl HandleId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for HandleId {
    fn default() -> Self {
        Self::new()
    }
}

/// A set of capabilities granted to an Agent
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilitySet {
    pub capabilities: HashSet<Capability>,
}

impl CapabilitySet {
    pub fn new() -> Self {
        Self {
            capabilities: HashSet::new(),
        }
    }

    pub fn empty() -> Self {
        Self::new()
    }

    pub fn with_capability(mut self, cap: Capability) -> Self {
        self.capabilities.insert(cap);
        self
    }

    pub fn contains(&self, cap: &Capability) -> bool {
        self.capabilities.contains(cap)
    }

    pub fn intersection(&self, other: &CapabilitySet) -> CapabilitySet {
        CapabilitySet {
            capabilities: self
                .capabilities
                .intersection(&other.capabilities)
                .cloned()
                .collect(),
        }
    }
}

/// Individual capability
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Capability {
    /// Read from a specific tool
    ToolRead { tool_id: String },
    /// Write to a specific tool
    ToolWrite { tool_id: String },
    /// Invoke another agent
    AgentInvoke { agent_id: AgentId },
    /// Access context storage
    ContextAccess { scope: String },
}

/// Action to be executed by the Kernel
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Action {
    /// Call a tool
    ToolCall {
        tool_id: String,
        params: serde_json::Value,
    },
    /// Call another agent
    CallAgent {
        target_id: AgentId,
        action: Box<Action>,
    },
    /// Read from context
    ContextRead { key: String },
    /// Write to context
    ContextWrite {
        key: String,
        value: serde_json::Value,
    },
}

/// Result of executing an action
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionResult {
    /// Success with value
    Success(serde_json::Value),
    /// Error occurred
    Error(ProtocolError),
}

/// Opaque handle for Agent operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelHandle {
    pub agent_id: AgentId,
    pub handle_id: HandleId,
    pub capabilities: CapabilitySet,
}

impl KernelHandle {
    pub fn new(agent_id: AgentId, capabilities: CapabilitySet) -> Self {
        Self {
            agent_id,
            handle_id: HandleId::new(),
            capabilities,
        }
    }
}

/// Agent definition for spawning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDef {
    pub name: String,
    pub config: serde_json::Value,
}

impl AgentDef {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            config: serde_json::json!({}),
        }
    }

    pub fn with_config(mut self, config: serde_json::Value) -> Self {
        self.config = config;
        self
    }
}

/// Audit filter for querying logs
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuditFilter {
    pub from_timestamp: Option<u64>,
    pub to_timestamp: Option<u64>,
    pub agent_id: Option<AgentId>,
    pub run_id: Option<RunId>,
}

/// Log entry in the audit trail
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub sequence: u64,
    pub timestamp: u64,
    pub run_id: RunId,
    pub span_id: SpanId,
    pub parent_span_id: Option<SpanId>,
    pub action: Action,
    pub result: Option<ActionResult>,
    pub integrity: [u8; 32],
}

/// Summary returned when an Agent is revoked
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunSummary {
    pub run_id: RunId,
    pub actions_executed: u64,
    pub final_status: String,
}

/// Checkpoint for suspend/resume operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub agent_id: AgentId,
    pub state: Vec<u8>,
}

/// Context value for memory operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextValue {
    pub data: Vec<u8>,
}

/// Context snapshot for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSnapshot {
    pub data: Vec<u8>,
    pub timestamp: u64,
}

/// Chunk for streaming responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub data: Vec<u8>,
    pub is_final: bool,
}

/// Agent event for signal/event interface
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEvent {
    pub event_type: String,
    pub payload: serde_json::Value,
}

/// Event filter for subscriptions
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EventFilter {
    pub event_types: Vec<String>,
}

/// Agent signal for interrupt
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentSignal {
    /// Human in the loop required
    HumanInTheLoop,
    /// Cancel current operation
    Cancel,
}

/// Risk level for operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// Span context for observability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanContext {
    pub span_id: SpanId,
    pub parent_id: Option<SpanId>,
}

/// Lifecycle event types
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LifecycleEvent {
    /// Agent spawned
    Spawned,
    /// Agent suspended
    Suspended,
    /// Agent resumed
    Resumed,
    /// Agent terminated
    Terminated,
}

/// Context key for memory operations
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContextKey(pub String);

/// Semantic query for context search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticQuery {
    pub query: String,
    pub embedding: Option<Vec<f32>>,
}

/// Message for LLM inference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

/// Inference parameters
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InferParams {
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========== Identity Types Tests ==========

    #[test]
    fn agent_id_creation_and_uniqueness() {
        // RED: Write test expecting unique IDs
        // GREEN: Implementation uses Uuid::new_v4()
        let id1 = AgentId::new();
        let id2 = AgentId::new();
        assert_ne!(id1, id2, "Each AgentId should be unique");
    }

    #[test]
    fn agent_id_default_creates_new() {
        let id1: AgentId = Default::default();
        let id2: AgentId = Default::default();
        assert_ne!(id1, id2, "Default should create unique IDs");
    }

    #[test]
    fn agent_id_display_format() {
        let id = AgentId::new();
        let display = format!("{}", id);
        // Should display as UUID string
        assert!(!display.is_empty());
        assert_eq!(display.len(), 36, "UUID should be 36 chars");
    }

    #[test]
    fn run_id_creation_and_uniqueness() {
        let id1 = RunId::new();
        let id2 = RunId::new();
        assert_ne!(id1, id2, "Each RunId should be unique");
    }

    #[test]
    fn span_id_creation_and_uniqueness() {
        let id1 = SpanId::new();
        let id2 = SpanId::new();
        assert_ne!(id1, id2, "Each SpanId should be unique");
    }

    #[test]
    fn handle_id_creation_and_uniqueness() {
        let id1 = HandleId::new();
        let id2 = HandleId::new();
        assert_ne!(id1, id2, "Each HandleId should be unique");
    }

    // ========== Serde Tests ==========

    #[test]
    fn agent_id_serde_roundtrip() {
        let id = AgentId::new();
        let json = serde_json::to_string(&id).unwrap();
        let decoded: AgentId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, decoded);
    }

    #[test]
    fn run_id_serde_roundtrip() {
        let id = RunId::new();
        let json = serde_json::to_string(&id).unwrap();
        let decoded: RunId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, decoded);
    }

    #[test]
    fn capability_set_serde_roundtrip() {
        let set = CapabilitySet::new().with_capability(Capability::ToolRead {
            tool_id: "test".to_string(),
        });
        let json = serde_json::to_string(&set).unwrap();
        let decoded: CapabilitySet = serde_json::from_str(&json).unwrap();
        assert_eq!(set, decoded);
    }

    // ========== CapabilitySet Tests ==========

    #[test]
    fn capability_set_new_is_empty() {
        let set = CapabilitySet::new();
        assert!(set.capabilities.is_empty());
    }

    #[test]
    fn capability_set_with_capability_adds() {
        let cap = Capability::ToolRead {
            tool_id: "test".to_string(),
        };
        let set = CapabilitySet::new().with_capability(cap.clone());
        assert!(set.capabilities.contains(&cap));
    }

    #[test]
    fn capability_set_contains_check() {
        let cap = Capability::ToolRead {
            tool_id: "test".to_string(),
        };
        let set = CapabilitySet::new().with_capability(cap.clone());
        assert!(set.contains(&cap));

        let other_cap = Capability::ToolRead {
            tool_id: "other".to_string(),
        };
        assert!(!set.contains(&other_cap));
    }

    #[test]
    fn capability_set_intersection_basic() {
        // ADR-002: Capability Non-Amplification
        let cap1 = Capability::ToolRead {
            tool_id: "tool1".to_string(),
        };
        let cap2 = Capability::ToolRead {
            tool_id: "tool2".to_string(),
        };

        let set_a = CapabilitySet::new()
            .with_capability(cap1.clone())
            .with_capability(cap2.clone());

        let set_b = CapabilitySet::new().with_capability(cap1.clone());

        let intersection = set_a.intersection(&set_b);

        assert!(intersection.contains(&cap1));
        assert!(!intersection.contains(&cap2));
    }

    #[test]
    fn capability_set_intersection_empty() {
        let cap1 = Capability::ToolRead {
            tool_id: "tool1".to_string(),
        };
        let cap2 = Capability::ToolRead {
            tool_id: "tool2".to_string(),
        };

        let set_a = CapabilitySet::new().with_capability(cap1);
        let set_b = CapabilitySet::new().with_capability(cap2);

        let intersection = set_a.intersection(&set_b);
        assert!(intersection.capabilities.is_empty());
    }

    #[test]
    fn capability_set_duplicate_capabilities_ignored() {
        // HashSet behavior: duplicates are ignored
        let cap = Capability::ToolRead {
            tool_id: "test".to_string(),
        };
        let set = CapabilitySet::new()
            .with_capability(cap.clone())
            .with_capability(cap.clone());

        assert_eq!(set.capabilities.len(), 1);
    }

    // ========== Action Tests ==========

    #[test]
    fn action_tool_call_creation() {
        let action = Action::ToolCall {
            tool_id: "calculator".to_string(),
            params: serde_json::json!({"expr": "1+1"}),
        };

        match action {
            Action::ToolCall { tool_id, .. } => {
                assert_eq!(tool_id, "calculator");
            }
            _ => panic!("Expected ToolCall variant"),
        }
    }

    #[test]
    fn agent_def_builder_pattern() {
        let def = AgentDef::new("test-agent").with_config(serde_json::json!({"key": "value"}));

        assert_eq!(def.name, "test-agent");
        assert_eq!(def.config["key"], "value");
    }

    #[test]
    fn kernel_handle_creation() {
        let agent_id = AgentId::new();
        let caps = CapabilitySet::new();
        let handle = KernelHandle::new(agent_id, caps);

        assert_eq!(handle.agent_id, agent_id);
    }
}
