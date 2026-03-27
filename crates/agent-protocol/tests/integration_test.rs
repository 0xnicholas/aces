//! Integration tests for agent-protocol
//!
//! These tests verify the core functionality of the agent-protocol crate.

use agent_protocol::*;

// ========== ADR-002: Capability Non-Amplification Tests ==========

#[test]
fn capability_intersection_basic() {
    // Test ADR-002: Capability Non-Amplification
    // delegated_caps = caller_caps ∩ target_caps

    let common_tool = Capability::ToolRead {
        tool_id: "tool_a".to_string(),
    };
    let caller_exclusive = Capability::ToolRead {
        tool_id: "tool_b".to_string(),
    };
    let target_exclusive = Capability::ToolWrite {
        tool_id: "tool_c".to_string(),
    };

    let caller_caps = CapabilitySet::new()
        .with_capability(common_tool.clone())
        .with_capability(caller_exclusive.clone());

    let target_caps = CapabilitySet::new()
        .with_capability(common_tool.clone())
        .with_capability(target_exclusive.clone());

    let delegated = caller_caps.intersection(&target_caps);

    // Should have common capability
    assert!(
        delegated.contains(&common_tool),
        "Intersection should contain common capability"
    );

    // Should NOT have caller's exclusive capability (non-amplification)
    assert!(
        !delegated.contains(&caller_exclusive),
        "Intersection should NOT contain caller's exclusive capability (ADR-002)"
    );

    // Should NOT have target's exclusive capability
    assert!(
        !delegated.contains(&target_exclusive),
        "Intersection should NOT contain target's exclusive capability"
    );
}

#[test]
fn capability_intersection_empty() {
    // Test when caller and target have no common capabilities
    let caller_tool = Capability::ToolRead {
        tool_id: "tool_a".to_string(),
    };
    let target_tool = Capability::ToolRead {
        tool_id: "tool_b".to_string(),
    };

    let caller_caps = CapabilitySet::new().with_capability(caller_tool);
    let target_caps = CapabilitySet::new().with_capability(target_tool);

    let delegated = caller_caps.intersection(&target_caps);
    assert!(
        delegated.capabilities.is_empty(),
        "Intersection of disjoint sets should be empty"
    );
}

#[test]
fn capability_intersection_same_set() {
    // Test intersection of identical sets
    let cap1 = Capability::ToolRead {
        tool_id: "tool1".to_string(),
    };
    let cap2 = Capability::ContextAccess {
        scope: "session".to_string(),
    };

    let set1 = CapabilitySet::new()
        .with_capability(cap1.clone())
        .with_capability(cap2.clone());

    let set2 = CapabilitySet::new()
        .with_capability(cap1.clone())
        .with_capability(cap2.clone());

    let intersection = set1.intersection(&set2);

    assert!(intersection.contains(&cap1));
    assert!(intersection.contains(&cap2));
    assert_eq!(intersection.capabilities.len(), 2);
}

// ========== Error Priority Tests ==========

#[test]
fn error_priority_invalid_handle_highest() {
    use agent_protocol::errors::error_priority;

    let err = ProtocolError::InvalidHandle {
        reason: HandleInvalidReason::Expired,
    };
    assert_eq!(
        error_priority(&err),
        4,
        "InvalidHandle should have highest priority (4)"
    );
}

#[test]
fn error_priority_policy_violation_second() {
    use agent_protocol::errors::error_priority;

    let err = ProtocolError::PolicyViolation {
        action: Action::ContextRead {
            key: "test".to_string(),
        },
        missing_cap: Capability::ContextAccess {
            scope: "test".to_string(),
        },
        agent_id: AgentId::new(),
    };
    assert_eq!(
        error_priority(&err),
        3,
        "PolicyViolation should have second priority (3)"
    );
}

#[test]
fn error_priority_resource_exhausted_third() {
    use agent_protocol::errors::error_priority;

    let err = ProtocolError::ResourceExhausted {
        resource: ResourceKind::ComputeQuota,
        retry_after: None,
    };
    assert_eq!(
        error_priority(&err),
        2,
        "ResourceExhausted should have third priority (2)"
    );
}

#[test]
fn error_priority_others_lowest() {
    use agent_protocol::errors::error_priority;

    let test_cases = vec![
        ProtocolError::Interrupted {
            token: InterruptToken("test".to_string()),
            rejected: false,
        },
        ProtocolError::Timeout {
            action: Action::ContextRead {
                key: "test".to_string(),
            },
            limit: std::time::Duration::from_secs(30),
        },
        ProtocolError::ContextOverflow {
            current: 100,
            limit: 50,
        },
        ProtocolError::Cancelled {
            run_id: RunId::new(),
        },
        ProtocolError::AuditIntegrityError {
            seq: 1,
            expected: vec![1, 2, 3],
            actual: vec![4, 5, 6],
        },
        ProtocolError::ProtocolViolation {
            detail: "test".to_string(),
        },
    ];

    for err in test_cases {
        assert_eq!(
            error_priority(&err),
            1,
            "All other errors should have lowest priority (1)"
        );
    }
}

#[test]
fn compare_by_priority_sorts_descending() {
    use agent_protocol::errors::compare_by_priority;

    let invalid = ProtocolError::InvalidHandle {
        reason: HandleInvalidReason::Revoked,
    };
    let policy = ProtocolError::PolicyViolation {
        action: Action::ContextRead {
            key: "test".to_string(),
        },
        missing_cap: Capability::ContextAccess {
            scope: "test".to_string(),
        },
        agent_id: AgentId::new(),
    };
    let resource = ProtocolError::ResourceExhausted {
        resource: ResourceKind::LlmConcurrency,
        retry_after: None,
    };
    let cancelled = ProtocolError::Cancelled {
        run_id: RunId::new(),
    };

    let mut errors = vec![&cancelled, &invalid, &policy, &resource];
    errors.sort_by(|a, b| compare_by_priority(a, b));

    // After sorting by priority descending: InvalidHandle, PolicyViolation, ResourceExhausted, Cancelled
    assert!(matches!(errors[0], ProtocolError::InvalidHandle { .. }));
    assert!(matches!(errors[1], ProtocolError::PolicyViolation { .. }));
    assert!(matches!(errors[2], ProtocolError::ResourceExhausted { .. }));
    assert!(matches!(errors[3], ProtocolError::Cancelled { .. }));
}

// ========== Serde Roundtrip Tests ==========

#[test]
fn serde_agent_id_roundtrip() {
    let original = AgentId::new();
    let json = serde_json::to_string(&original).expect("Failed to serialize AgentId");
    let decoded: AgentId = serde_json::from_str(&json).expect("Failed to deserialize AgentId");
    assert_eq!(original, decoded, "AgentId should roundtrip correctly");
}

#[test]
fn serde_run_id_roundtrip() {
    let original = RunId::new();
    let json = serde_json::to_string(&original).expect("Failed to serialize RunId");
    let decoded: RunId = serde_json::from_str(&json).expect("Failed to deserialize RunId");
    assert_eq!(original, decoded, "RunId should roundtrip correctly");
}

#[test]
fn serde_span_id_roundtrip() {
    let original = SpanId::new();
    let json = serde_json::to_string(&original).expect("Failed to serialize SpanId");
    let decoded: SpanId = serde_json::from_str(&json).expect("Failed to deserialize SpanId");
    assert_eq!(original, decoded, "SpanId should roundtrip correctly");
}

#[test]
fn serde_handle_id_roundtrip() {
    let original = HandleId::new();
    let json = serde_json::to_string(&original).expect("Failed to serialize HandleId");
    let decoded: HandleId = serde_json::from_str(&json).expect("Failed to deserialize HandleId");
    assert_eq!(original, decoded, "HandleId should roundtrip correctly");
}

#[test]
fn serde_capability_set_roundtrip() {
    let original = CapabilitySet::new()
        .with_capability(Capability::ToolRead {
            tool_id: "test".to_string(),
        })
        .with_capability(Capability::ContextAccess {
            scope: "session".to_string(),
        });

    let json = serde_json::to_string(&original).expect("Failed to serialize CapabilitySet");
    let decoded: CapabilitySet =
        serde_json::from_str(&json).expect("Failed to deserialize CapabilitySet");
    assert_eq!(
        original, decoded,
        "CapabilitySet should roundtrip correctly"
    );
}

#[test]
fn serde_protocol_error_roundtrip() {
    let original = ProtocolError::Cancelled {
        run_id: RunId::new(),
    };
    let json = serde_json::to_string(&original).expect("Failed to serialize ProtocolError");
    let decoded: ProtocolError =
        serde_json::from_str(&json).expect("Failed to deserialize ProtocolError");
    assert_eq!(
        original, decoded,
        "ProtocolError should roundtrip correctly"
    );
}

#[test]
fn serde_action_roundtrip() {
    let original = Action::ToolCall {
        tool_id: "calculator".to_string(),
        params: serde_json::json!({"expr": "1 + 1"}),
    };

    let json = serde_json::to_string(&original).expect("Failed to serialize Action");
    let decoded: Action = serde_json::from_str(&json).expect("Failed to deserialize Action");
    assert_eq!(original, decoded, "Action should roundtrip correctly");
}

#[test]
fn serde_kernel_handle_roundtrip() {
    let agent_id = AgentId::new();
    let caps = CapabilitySet::new().with_capability(Capability::ToolRead {
        tool_id: "test".to_string(),
    });

    let original = KernelHandle::new(agent_id, caps);
    let json = serde_json::to_string(&original).expect("Failed to serialize KernelHandle");
    let decoded: KernelHandle =
        serde_json::from_str(&json).expect("Failed to deserialize KernelHandle");

    assert_eq!(original.agent_id, decoded.agent_id);
    assert_eq!(original.capabilities, decoded.capabilities);
}

// ========== Full Workflow Tests ==========

#[test]
fn full_workflow_agent_creation() {
    // Simulate a complete workflow

    // 1. Create Agent definition
    let agent_def = AgentDef::new("calculator-agent").with_config(serde_json::json!({
        "model": "gpt-4",
        "max_tokens": 1000
    }));

    assert_eq!(agent_def.name, "calculator-agent");
    assert_eq!(agent_def.config["model"], "gpt-4");

    // 2. Create capabilities
    let caps = CapabilitySet::new()
        .with_capability(Capability::ToolRead {
            tool_id: "calculator".to_string(),
        })
        .with_capability(Capability::ToolWrite {
            tool_id: "calculator".to_string(),
        })
        .with_capability(Capability::ContextAccess {
            scope: "session".to_string(),
        });

    // 3. Create kernel handle
    let agent_id = AgentId::new();
    let handle = KernelHandle::new(agent_id, caps);

    assert_eq!(handle.agent_id, agent_id);
    assert!(handle.capabilities.contains(&Capability::ToolRead {
        tool_id: "calculator".to_string()
    }));

    // 4. Create and verify action
    let action = Action::ToolCall {
        tool_id: "calculator".to_string(),
        params: serde_json::json!({"expr": "2 + 2"}),
    };

    match action {
        Action::ToolCall { tool_id, params } => {
            assert_eq!(tool_id, "calculator");
            assert_eq!(params["expr"], "2 + 2");
        }
        _ => panic!("Expected ToolCall action"),
    }
}

#[test]
fn error_handling_flow() {
    // Test error creation and formatting

    // Create various error types
    let errors = vec![
        ProtocolError::InvalidHandle {
            reason: HandleInvalidReason::Expired,
        },
        ProtocolError::PolicyViolation {
            action: Action::ContextRead {
                key: "test".to_string(),
            },
            missing_cap: Capability::ContextAccess {
                scope: "test".to_string(),
            },
            agent_id: AgentId::new(),
        },
        ProtocolError::ResourceExhausted {
            resource: ResourceKind::LlmConcurrency,
            retry_after: Some(std::time::Duration::from_secs(60)),
        },
        ProtocolError::ContextOverflow {
            current: 1000,
            limit: 100,
        },
        ProtocolError::Cancelled {
            run_id: RunId::new(),
        },
        ProtocolError::ProtocolViolation {
            detail: "Invalid format".to_string(),
        },
    ];

    // Verify all errors can be formatted (Display trait)
    for err in errors {
        let message = format!("{}", err);
        assert!(!message.is_empty(), "Error should have a message");
    }
}

// ========== Edge Cases ==========

#[test]
fn capability_set_empty_operations() {
    let empty = CapabilitySet::new();
    assert!(empty.capabilities.is_empty());

    let other = CapabilitySet::new().with_capability(Capability::ToolRead {
        tool_id: "test".to_string(),
    });

    let intersection = empty.intersection(&other);
    assert!(intersection.capabilities.is_empty());
}

#[test]
fn agent_id_display_format() {
    let id = AgentId::new();
    let display = format!("{}", id);

    // UUID format: xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx (36 chars)
    assert_eq!(
        display.len(),
        36,
        "AgentId display should be 36 characters (UUID format)"
    );
    assert!(display.contains('-'), "UUID should contain hyphens");
}

#[test]
fn all_error_variants_display_correctly() {
    // Ensure all ProtocolError variants implement Display properly
    let error_variants = vec![
        (
            "InvalidHandle",
            ProtocolError::InvalidHandle {
                reason: HandleInvalidReason::Expired,
            },
        ),
        (
            "PolicyViolation",
            ProtocolError::PolicyViolation {
                action: Action::ContextRead {
                    key: "test".to_string(),
                },
                missing_cap: Capability::ContextAccess {
                    scope: "test".to_string(),
                },
                agent_id: AgentId::new(),
            },
        ),
        (
            "ResourceExhausted",
            ProtocolError::ResourceExhausted {
                resource: ResourceKind::LlmConcurrency,
                retry_after: None,
            },
        ),
        (
            "Interrupted",
            ProtocolError::Interrupted {
                token: InterruptToken("token123".to_string()),
                rejected: false,
            },
        ),
        (
            "Timeout",
            ProtocolError::Timeout {
                action: Action::ContextRead {
                    key: "test".to_string(),
                },
                limit: std::time::Duration::from_secs(30),
            },
        ),
        (
            "ContextOverflow",
            ProtocolError::ContextOverflow {
                current: 100,
                limit: 50,
            },
        ),
        (
            "Cancelled",
            ProtocolError::Cancelled {
                run_id: RunId::new(),
            },
        ),
        (
            "AuditIntegrityError",
            ProtocolError::AuditIntegrityError {
                seq: 1,
                expected: vec![1, 2, 3],
                actual: vec![4, 5, 6],
            },
        ),
        (
            "ProtocolViolation",
            ProtocolError::ProtocolViolation {
                detail: "Test error".to_string(),
            },
        ),
    ];

    for (name, err) in error_variants {
        let msg = format!("{}", err);
        assert!(
            msg.contains(name) || !msg.is_empty(),
            "{} error should have a meaningful message",
            name
        );
    }
}
