# Architecture Docs

This directory is the working area for the repository's internal architecture
materials. It contains the approved architecture note for the current refresh,
the implementation plan that drove the documentation update, and the
source-controlled diagram files.

## Quick Navigation

### For Contributors
Start here if you're working on the Kernel implementation:
1. Read the [Protocol Spec](../../protocol-spec/overview.md) for the interface contract
2. Read [ARCHITECTURE.md](../../ARCHITECTURE.md) for system context
3. Read [AGENTS.md](../../AGENTS.md) for development rules
4. Browse [Architecture Diagrams](#diagrams) below
5. Review [Architecture Decisions (ADRs)](#architecture-decisions) for design rationale

### For Integrators
Start here if you're building a Runtime or Framework:
1. Read the [Protocol Spec](../../protocol-spec/overview.md) for integration contract
2. Read [ARCHITECTURE.md](../../ARCHITECTURE.md) for system model and integration paths

---

## Diagrams

Mermaid source files in [diagrams/](diagrams/) are the source of truth for
architecture visualizations.

### Core Diagrams

| Diagram | File | Description |
|---------|------|-------------|
| **Main Architecture** | [kernel-protocol-main.mmd](diagrams/kernel-protocol-main.mmd) | Layered system view (L1-L5) showing Kernel internals |
| **A2A Governance** | [kernel-protocol-a2a.mmd](diagrams/kernel-protocol-a2a.mmd) | Agent-to-Agent calling flow with capability delegation |
| **Invocation Sequence** | [kernel-protocol-invoke-sequence.mmd](diagrams/kernel-protocol-invoke-sequence.mmd) | Critical path: validation → permission → schedule → sandbox → audit |

### Supporting Diagrams

| Diagram | File | Description |
|---------|------|-------------|
| **Error Handling** | [kernel-protocol-error-flow.mmd](diagrams/kernel-protocol-error-flow.mmd) | All 8 ProtocolError types and their handling paths |
| **Context Scopes** | [kernel-protocol-context-scopes.mmd](diagrams/kernel-protocol-context-scopes.mmd) | Session/Agent/Shared scope lifecycles and capabilities |

### Updating Diagrams

To modify a diagram, edit the corresponding `.mmd` file. View rendered diagrams:
- GitHub natively renders Mermaid diagrams
- VS Code with Mermaid extension
- [Mermaid Live Editor](https://mermaid.live/)

Legacy image assets should be treated as temporary until rendered outputs from
these sources replace them.

---

## Architecture Decisions (ADRs)

Architecture Decision Records capture significant design choices and their rationale.

Location: [decisions/](decisions/)

| ADR | Title | Status | Summary |
|-----|-------|--------|---------|
| [ADR-001](decisions/ADR-001-single-interception-point.md) | Single Interception Point | Accepted | All actions flow through `invoke` |
| [ADR-002](decisions/ADR-002-capability-intersection.md) | Capability Intersection Rule | Accepted | Delegated caps = caller ∩ target |
| [ADR-003](decisions/ADR-003-synchronous-audit-write.md) | Synchronous Audit Write | Accepted | WAL entry before result return |
| [ADR-004](decisions/ADR-004-protocol-kernel-separation.md) | Protocol-Kernel Separation | Accepted | MIT (Protocol) + Apache 2.0 (Kernel) |
| [ADR-005](decisions/ADR-005-sandbox-isolation-strategy.md) | Sandbox Isolation Strategy | Accepted | Linux seccomp-bpf + namespaces |
| [ADR-006](decisions/ADR-006-scheduling-policy.md) | Scheduling Policy | Accepted | Token bucket + priority queue |

See [decisions/README.md](decisions/README.md) for the ADR template and process.

---

## Implementation Guide

[IMPLEMENTATION-GUIDE.md](IMPLEMENTATION-GUIDE.md) - Practical guide for Kernel developers covering:
- Build environment and dependencies
- Crates organization and interfaces
- Testing strategy and performance budgets
- Release process

---

## Contents

### Architecture Notes
- [2026-03-25-kernel-protocol-architecture.md](2026-03-25-kernel-protocol-architecture.md)
  Approved internal architecture note for the current Kernel + Protocol refresh.
- [2026-03-25-kernel-protocol-implementation-plan.md](2026-03-25-kernel-protocol-implementation-plan.md)
  Working implementation plan used to drive the documentation update.

### Design Decisions
- [decisions/](decisions/) - Architecture Decision Records

### Developer Resources
- [IMPLEMENTATION-GUIDE.md](IMPLEMENTATION-GUIDE.md) - Developer implementation guide

---

## Maintenance

Add new architecture notes to this directory. Keep long-lived entry points
stable, and prefer linking to this directory-level index from top-level docs
instead of linking directly to date-stamped notes unless a specific note is the
point of reference.

When adding new diagrams:
1. Create the `.mmd` file in [diagrams/](diagrams/)
2. Add it to the [Diagrams](#diagrams) table above
3. Link to it from relevant documents

When adding new ADRs:
1. Use the template in [decisions/README.md](decisions/README.md)
2. Assign the next available number
3. Update the ADR table above
