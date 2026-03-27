# ADR-002: Capability Intersection Rule

## Status

Accepted

## Context

Agent-to-Agent (A2A) calling introduces a delegation problem: when Agent A calls Agent B, what capabilities should B have? There are several possibilities:

1. **Full inheritance** - B gets A's full capabilities (risk: privilege escalation)
2. **Target's capabilities** - B gets its own capabilities (risk: caller assumes target has permissions it doesn't)
3. **Caller-specified** - A specifies what to delegate (risk: A might delegate more than it has)
4. **Intersection** - B gets the intersection of A and B's capabilities

The fundamental security property we need: **delegation cannot amplify capabilities**.

## Decision

When Agent A calls Agent B, the delegated capability set is computed as:

```
delegated_caps = caller_caps ∩ target_caps
```

Where `∩` is defined as:
- Both sets must contain a matching `Capability` variant
- For parameterized capabilities (e.g., `ToolRead { tool_id }`), parameters must match exactly
- `AgentCall { target_agent_id: None }` ∩ `AgentCall { target_agent_id: Some(X) }` = `AgentCall { target_agent_id: Some(X) }` (specific wins)
- `LlmCall { model_family: None }` ∩ `LlmCall { model_family: Some("claude") }` = `LlmCall { model_family: Some("claude") }` (specific wins)

Agent B executes **only** with `delegated_caps`. It cannot access:
- Capabilities A has but B doesn't
- Capabilities B has but A doesn't

## Consequences

### Positive

- **Security guarantee** - Structurally impossible to delegate authority you don't have
- **Explicit trust** - Caller and target must both agree on capabilities
- **Defense in depth** - Even if one Agent is compromised, delegation limits the blast radius
- **Simple to understand** - Mathematical property (subset) is clear

### Negative

- **Restrictive** - Two Agents with complementary capabilities cannot combine them through delegation
- **Surprising behavior** - Developers might expect B to use its own capabilities
- **Capability discovery** - Caller must know target's capabilities to predict what will be available

### Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| Unexpected capability loss | Clear documentation; capability introspection API |
| Performance of intersection calculation | Cache intersection results; pre-compute at spawn time |
| Deep call chain complexity | Document that effective caps shrink with depth |

## Alternatives Considered

### Alternative 1: Full Inheritance
delegated_caps = caller_caps

**Rejected because:**
- Privilege escalation: low-privilege A calls high-privilege B, B gains A's low privs + its own high privs
- Violates principle of least privilege
- Defeats capability-based security model

### Alternative 2: Target's Capabilities
delegated_caps = target_caps

**Rejected because:**
- Caller might assume target can perform action based on caller's capabilities
- Caller might accidentally expose sensitive operations to untrusted target
- No way for caller to limit what target can do

### Alternative 3: Caller-Specified Delegation
delegated_caps = caller_specified (subject to caller_caps as upper bound)

**Rejected because:**
- Complex to implement correctly (must validate subset relationship)
- Error-prone: easy to accidentally specify wrong capabilities
- Requires caller to understand target's needs
- Still allows caller to delegate all its capabilities (same risk as full inheritance)

### Alternative 4: Union
delegated_caps = caller_caps ∪ target_caps

**Rejected because:**
- Worst of both worlds: privilege escalation AND caller/target mismatch
- Completely violates capability security model

## Related

- [protocol-spec/overview.md](../../protocol-spec/overview.md) §8 Agent-to-Agent Calling Convention
- [protocol-spec/overview.md](../../protocol-spec/overview.md) §9.3 Delegation Intersection Rule
- [ADR-001](ADR-001-single-interception-point.md) - A2A calls flow through invoke
