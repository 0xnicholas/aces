# Architecture Decision Records (ADRs)

This directory contains Architecture Decision Records for the Agent Kernel project.

## What is an ADR?

An Architecture Decision Record (ADR) captures an important architectural decision made along with its context and consequences. ADRs are immutable once accepted (new decisions supersede old ones rather than modifying them).

## ADR Status

- **Proposed** - Under review, not yet approved
- **Accepted** - Approved and in effect
- **Deprecated** - No longer relevant, superseded by newer ADR
- **Superseded** - Replaced by newer ADR (link to replacement)

## Current ADRs

| ADR | Title | Status | Date | Summary |
|-----|-------|--------|------|---------|
| [ADR-001](ADR-001-single-interception-point.md) | Single Interception Point | Accepted | 2024-03-27 | All actions flow through `invoke` method |
| [ADR-002](ADR-002-capability-intersection.md) | Capability Intersection Rule | Accepted | 2024-03-27 | Delegated caps = caller ∩ target |
| [ADR-003](ADR-003-synchronous-audit-write.md) | Synchronous Audit Write | Accepted | 2024-03-27 | WAL entry written before result return |
| [ADR-004](ADR-004-protocol-kernel-separation.md) | Protocol-Kernel Separation | Accepted | 2024-03-27 | Dual licensing: MIT (Protocol) + Apache 2.0 (Kernel) |
| [ADR-005](ADR-005-sandbox-isolation-strategy.md) | Sandbox Isolation Strategy | Accepted | 2024-03-27 | Linux seccomp-bpf + namespaces |
| [ADR-006](ADR-006-scheduling-policy.md) | Scheduling Policy | Accepted | 2024-03-27 | Token bucket + priority queue |

## ADR Template

When creating a new ADR, use this template:

```markdown
# ADR-XXX: Title

## Status

- Proposed / Accepted / Deprecated / Superseded by ADR-YYY

## Context

What is the issue that we're seeing that is motivating this decision or change?

## Decision

What is the change that we're proposing or have agreed to implement?

## Consequences

What becomes easier or more difficult to do and any risks introduced by the change that will need to be mitigated.

### Positive
- Benefit 1
- Benefit 2

### Negative
- Drawback 1
- Drawback 2

### Risks
- Risk and mitigation

## Alternatives Considered

### Alternative 1: [Name]
- Why it was rejected

### Alternative 2: [Name]
- Why it was rejected

## Related

- Links to related ADRs
- Links to relevant documentation
```

## Creating a New ADR

1. Copy this template to a new file: `ADR-XXX-short-title.md`
2. Replace XXX with the next available number
3. Fill in all sections
4. Set status to "Proposed"
5. Submit for review
6. Once approved, update status to "Accepted" and add to the table above

## Review Process

- ADRs are reviewed via PR
- Require one maintainer approval for minor decisions
- Require two maintainer approvals for major architectural changes
- Superseding an existing ADR requires linking to the new ADR in the old one
