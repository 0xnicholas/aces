# Internal Ops Agent Governance Technical Design Plan

## Purpose

This document defines the design work that should be completed before moving
into full implementation of the Internal Ops Agent Governance MVP.

It is not a task-by-task engineering execution plan. It is a technical design
plan covering the full MVP through demo-readiness. The goal is to ensure the
team stabilizes the right interfaces, object models, control flow, and
operational assumptions before implementation expands.

## Relationship To Other Documents

- [2026-03-26-internal-ops-agent-governance-mvp.md](2026-03-26-internal-ops-agent-governance-mvp.md)
  defines the MVP product scope
- [2026-03-26-internal-ops-agent-governance-prd.md](2026-03-26-internal-ops-agent-governance-prd.md)
  defines the internal product requirements
- [2026-03-26-internal-ops-agent-governance-technical-roadmap.md](2026-03-26-internal-ops-agent-governance-technical-roadmap.md)
  defines the staged implementation path

This document answers a different question:

`What technical design artifacts must exist before implementation is likely to stay coherent?`

## Design Goal

Produce a coherent technical design for an MVP that demonstrates:

- controlled invocation through a single path
- capability enforcement
- bounded sub-agent delegation
- approval-gated dangerous actions
- structured audit reconstruction
- a repeatable internal ops demo

## Design Scope

This plan covers the full MVP up to demo-readiness, including:

- MVP protocol subset
- `kernel-api` surface design
- `kernel-core` dispatch design
- capability model design
- audit model design
- approval interrupt design
- agent registry and `CallAgent` behavior
- demo fixture and example tool assumptions

This plan does not cover:

- generalized multi-tenant platform architecture
- broad SDK design
- full production hardening
- complete UI design
- large-scale performance design

## Design Principles

- Design the narrowest coherent system that can prove the product thesis.
- Keep governance semantics explicit in every design artifact.
- Prefer a small number of stable core objects over many provisional ones.
- Keep product demo needs visible during design; do not treat them as a later
  packaging concern.
- Do not design for generalized platform breadth before the internal ops use
  case is coherent.

## Design Theme 1: MVP System Boundary

### Objective

Define exactly what belongs inside the MVP Kernel-governed system and what is
assumed external.

### Required design outputs

- a system context diagram for the MVP
- a boundary table covering:
  - Runtime
  - Kernel API
  - Protocol subset
  - agent registry
  - tools
  - approval actor
  - audit query consumer
- a list of external dependencies assumed for the demo

### Key design questions

- What is considered inside the Kernel versus adjacent demo infrastructure?
- Is approval modeled as a Kernel-visible control step or a purely external
  callback?
- Which parts of the tool environment are mocked versus real?

### Design completion criteria

- the team can explain the MVP boundary in one diagram and one page of prose
- there is no ambiguity about which subsystem owns approval, audit, and
  delegation logic

## Design Theme 2: Core Object Model

### Objective

Define the smallest stable object model required for the MVP.

### Required design outputs

- object definitions for:
  - `Agent`
  - `CapabilitySet`
  - `KernelHandle`
  - `Action`
  - `ActionResult`
  - `AuditEntry`
  - `ApprovalInterrupt`
  - `RunId`
  - `SpanId`
- field-level rationale for each type
- invariants for each object where relevant

### Key design questions

- What fields are truly required for the MVP versus merely nice to have?
- Where should approval state live?
- What must be stable across the parent/child call chain?
- Which objects are public-facing versus internal?

### Design completion criteria

- the MVP object set is small and stable enough to support API and dispatch
  design
- every field has a clear reason to exist

## Design Theme 3: MVP Protocol Subset

### Objective

Define the protocol subset necessary for the internal ops use case.

### Required design outputs

- a narrowed list of protocol interfaces used in the MVP
- the minimal action taxonomy required for:
  - `read_logs`
  - `read_metrics`
  - `open_incident`
  - `restart_service`
  - `CallAgent`
- a result/error taxonomy for MVP execution states:
  - success
  - policy denial
  - interrupt pending approval
  - cancelled or aborted action path if needed

### Key design questions

- Which full protocol concepts can be deferred without damaging the MVP?
- What is the minimum error vocabulary that still tells the truth?
- How should `CallAgent` be represented in the MVP subset?

### Design completion criteria

- the team can explain the MVP protocol without referring to the full future
  protocol surface
- the protocol subset directly supports the demo scenario and no obvious
  unrelated use cases

## Design Theme 4: `kernel-api` Surface

### Objective

Define the MVP-facing Kernel API contract.

### Required design outputs

- API contract for:
  - `spawn`
  - `invoke`
  - `revoke`
  - `query_audit`
- ownership of handle validation
- mapping from public API calls to internal dispatch behavior

### Key design questions

- What must callers provide on `spawn`?
- What must `invoke` guarantee on denial, interruption, and success?
- What should `query_audit` return for the MVP demo?
- What should remain opaque to callers?

### Design completion criteria

- the public API is small enough to support the MVP without exposing internals
- `invoke` semantics are precise enough to anchor the rest of the design

## Design Theme 5: Dispatch And Control Flow

### Objective

Define the controlled execution path and the sequencing guarantees around it.

### Required design outputs

- a dispatch sequence for normal execution
- a dispatch sequence for denial
- a dispatch sequence for approval interrupt
- a dispatch sequence for child-agent invocation
- a statement of ordering guarantees for:
  - permission check
  - approval interrupt
  - audit write
  - result visibility

### Key design questions

- At what point is a dangerous action interrupted?
- At what point is an audit record created?
- What is the exact difference between denial and interrupt?
- What is the run/span behavior across child invocation?

### Design completion criteria

- the full MVP flow can be explained as a deterministic control path
- there is no ambiguity about where permissions, audit, and approval occur

## Design Theme 6: Capability Model

### Objective

Define the MVP capability model and how it maps to the internal ops tools.

### Required design outputs

- capability definitions for the four MVP tools
- a capability assignment model for:
  - `ops-investigator`
  - `ops-executor`
- a delegation rule specification:

`delegated_caps = caller_caps ∩ target_caps`

- examples of allowed and denied calls

### Key design questions

- How fine-grained must capabilities be in the MVP?
- Are tool actions enough, or do some actions need risk classes?
- What must be visible in audit about active capability scope?

### Design completion criteria

- the capability model is simple enough to explain live
- the delegation rule is explicit and testable

## Design Theme 7: Approval Model

### Objective

Define how dangerous actions are interrupted, approved, rejected, and resumed.

### Required design outputs

- a dangerous-action classification for the MVP
- interrupt state model
- approval state transitions
- resume/reject semantics
- audit representation of interrupt and approval outcomes

### Key design questions

- Is `restart_service` the only dangerous action in the MVP?
- What exact data is needed to resume safely after approval?
- What happens to stale or rejected approval requests?

### Design completion criteria

- the approval model is specific enough to implement without ad hoc decisions
- interrupt and approval states are visible in both API and audit design

## Design Theme 8: Audit Model And Query Shape

### Objective

Define the minimum audit system needed for reconstruction and demo credibility.

### Required design outputs

- `AuditEntry` schema for the MVP
- required linkage fields:
  - `run_id`
  - `agent_id`
  - `parent_span_id`
  - `span_id`
  - action summary
  - result summary
- query shape for run reconstruction
- sample audit output for the demo scenario

### Key design questions

- What is the smallest audit schema that still proves the thesis?
- What must be queryable versus merely stored?
- How should interrupts and denials appear in the same model?

### Design completion criteria

- a single run can be reconstructed from the designed audit schema
- the audit design supports the planned demo script

## Design Theme 9: Agent Registry And Child Invocation

### Objective

Define how target agents are discovered and how child execution is created.

### Required design outputs

- a minimal agent registry model
- target lookup semantics
- child invocation lifecycle
- run/span propagation rules
- delegated capability application rules

### Key design questions

- Is the registry static for the MVP or runtime-managed?
- What is the minimal metadata needed for a child agent?
- What exactly is inherited versus regenerated in child execution?

### Design completion criteria

- child invocation behavior is deterministic
- run/span and capability rules are explicit and auditable

## Design Theme 10: Demo Fixtures And Example Tool Contracts

### Objective

Define the non-production but credible tool environment needed for the MVP demo.

### Required design outputs

- contract definitions for:
  - `read_logs`
  - `read_metrics`
  - `open_incident`
  - `restart_service`
- mock or fixture behavior for demo scenarios
- expected outputs for:
  - normal investigation
  - denied restart
  - approval-gated restart

### Key design questions

- Which tools should be mocked versus lightly integrated?
- How deterministic should the demo fixture data be?
- What is the minimum realism needed to be credible to platform teams?

### Design completion criteria

- the team can run a repeatable internal demo without depending on fragile live
  infrastructure

## Cross-Cutting Decisions

These decisions affect multiple themes and should be settled explicitly:

- where public API types stop and internal orchestration types begin
- whether audit records are written before or after action execution states
  resolve
- how approval interrupts are represented to callers
- what level of isolation is modeled versus deferred
- whether the MVP uses in-memory persistence only or defines persistence
  abstraction boundaries early

## Recommended Design Sequence

The design work should proceed in this order:

1. MVP system boundary
2. core object model
3. MVP protocol subset
4. `kernel-api` surface
5. dispatch and control flow
6. capability model
7. approval model
8. audit model and query shape
9. agent registry and child invocation
10. demo fixtures and tool contracts

This sequence is recommended because each later theme depends on stable
definitions from the earlier ones.

## Design Dependencies

- the public API cannot stabilize before the object model stabilizes
- dispatch sequencing cannot stabilize before result and interrupt semantics
  are defined
- audit design depends on run/span and child invocation design
- approval design depends on dangerous action classification and result modeling
- demo fixture design depends on the final action/tool contract

## Design Review Checklist

Before implementation starts, the team should be able to answer yes to each of
the following:

- Can we explain the MVP boundary in one diagram?
- Can we explain every public object in one sentence?
- Do we know exactly where permission checks happen?
- Do we know exactly when audit records are created?
- Do we know exactly how approval interrupts behave?
- Do we know exactly how child capabilities are derived?
- Can we show a sample run and sample audit chain on paper?
- Can we explain the demo without hand-waving missing control-path behavior?

## Definition Of Design Completion

The technical design phase is complete when:

- the MVP object model is stable
- the MVP protocol subset is documented
- the `kernel-api` surface is documented
- dispatch, approval, audit, and child invocation flows are documented
- the capability model for the demo agents is explicit
- the demo tool contracts are explicit
- the team can produce sample action and audit traces without improvisation

At that point, implementation can proceed with much lower risk of architectural
drift.

## Summary

This design plan exists to prevent the team from jumping straight from product
thesis to implementation. The MVP will only be credible if the Kernel behavior
is designed as a coherent governed system, not assembled as a set of loosely
related features.

The design work should therefore stabilize:

- what is being governed
- how it is invoked
- how authority is bounded
- how dangerous actions are interrupted
- how actions are recorded
- how the demo makes all of that visible
