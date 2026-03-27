//! Core types for the Agent Protocol

use crate::errors::ProtocolError;
use std::fmt;
use uuid::Uuid;

/// Unique identifier for an Agent
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

/// A set of capabilities granted to an Agent
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CapabilitySet {
    pub capabilities: Vec<Capability>,
}

impl CapabilitySet {
    pub fn new() -> Self {
        Self {
            capabilities: Vec::new(),
        }
    }

    pub fn empty() -> Self {
        Self::new()
    }

    pub fn with_capability(mut self, cap: Capability) -> Self {
        self.capabilities.push(cap);
        self
    }
}

/// Individual capability
#[derive(Debug, Clone, PartialEq, Eq)]
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
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
pub enum ActionResult {
    /// Success with value
    Success(serde_json::Value),
    /// Error occurred
    Error(ProtocolError),
}

/// Opaque handle for Agent operations
#[derive(Debug, Clone)]
pub struct KernelHandle {
    pub agent_id: AgentId,
    pub capabilities: CapabilitySet,
}

impl KernelHandle {
    pub fn new(agent_id: AgentId, capabilities: CapabilitySet) -> Self {
        Self {
            agent_id,
            capabilities,
        }
    }
}

/// Agent definition for spawning
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone, Default)]
pub struct AuditFilter {
    pub from_timestamp: Option<u64>,
    pub to_timestamp: Option<u64>,
    pub agent_id: Option<AgentId>,
    pub run_id: Option<RunId>,
}

/// Log entry in the audit trail
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
pub struct RunSummary {
    pub run_id: RunId,
    pub actions_executed: u64,
    pub final_status: String,
}
