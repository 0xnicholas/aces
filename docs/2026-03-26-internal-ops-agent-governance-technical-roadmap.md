# Internal Ops Agent Governance Technical Roadmap

## Purpose

This roadmap translates the Internal Ops Agent Governance product definition
into an implementation-oriented technical path.

It is designed to answer two questions at the same time:

- what phases the team should move through
- which technical modules matter in each phase

The roadmap is intentionally staged. It is not a full task plan. It exists to
help the team sequence implementation work without prematurely expanding scope.

## Guiding Principles

- Prove the controlled path before broadening the platform.
- Keep the first implementation tightly coupled to the internal ops MVP.
- Make permissions, approval, and audit first-class from the start.
- Avoid building orchestration or platform features before governance is proven.
- Treat "demo credibility" as a real engineering requirement, not only a
  presentation concern.

## Phase 0: Design Baseline And Minimal Skeleton

### Goal

Establish the minimum technical shape required to implement the MVP without
yet attempting a production-ready system.

### Primary modules

- `agent-protocol`
- `kernel-api`
- minimal `kernel-core`
- architecture and product documents

### Scope

This phase creates the implementation contract, not the full implementation.

The team should decide:

- the minimum data model required for the MVP
- the exact action types required for the ops demo
- the minimum audit schema needed for replay
- the approval interrupt shape
- the boundary between `kernel-api` and internal orchestration

### Key deliverables

- a narrowed MVP protocol surface for internal ops
- a first-pass object model:
  - `Agent`
  - `CapabilitySet`
  - `Action`
  - `KernelHandle`
  - `ActionResult`
  - `AuditEntry`
  - `ApprovalInterrupt`
- a concrete `invoke` path definition for MVP actions
- a minimal repository implementation layout decision

### What not to do

- do not build a generalized policy language
- do not build multiple integration surfaces
- do not build a broad SDK story
- do not optimize performance before the path exists

### Exit criteria

- the MVP object model is stable enough to start implementation
- the `invoke` path is clearly specified
- the dangerous-action approval behavior is specified
- child capability reduction is specified

## Phase 1: Controlled Action Path MVP

### Goal

Implement the smallest end-to-end Kernel path that can accept actions, enforce
capabilities, and return structured results.

### Primary modules

- `agent-protocol`
- `kernel-api`
- `kernel-core`
- `permission-engine`

### Scope

This phase should produce a minimal but working flow for:

- agent registration
- capability binding
- action submission
- permission denial
- structured results

### Module focus

#### `agent-protocol`

Implement the MVP subset needed for:

- `invoke`
- `CallAgent`
- structured errors
- run and span identifiers

#### `kernel-api`

Implement the smallest usable public surface:

- `spawn`
- `invoke`
- `revoke`
- `query_audit`

#### `kernel-core`

Implement the dispatch path skeleton:

- handle validation
- permission check
- action dispatch
- result shaping

#### `permission-engine`

Implement explicit capability checks for the initial tools:

- `read_logs`
- `read_metrics`
- `restart_service`
- `open_incident`

### Key deliverables

- a working `invoke` pipeline
- permission denials for unauthorized actions
- structured `ActionResult` values
- minimal support for `ops-investigator`

### What not to do

- do not implement complete audit-chain integrity yet if it blocks MVP flow
- do not attempt broad sandboxing yet
- do not add every protocol interface family up front

### Exit criteria

- an agent can be spawned with explicit capabilities
- a read action succeeds through `invoke`
- an unauthorized restart is denied
- the denial is visible in structured output

## Phase 2: Audit, Approval, And Sub-Agent Control

### Goal

Turn the minimal action path into a credible governed system for the ops demo.

### Primary modules

- `kernel-core`
- `audit-log`
- `permission-engine`
- approval / interrupt handling
- agent registry

### Scope

This phase is where the MVP becomes believable.

It should implement:

- audit record creation on every controlled action
- child invocation support
- capability intersection for sub-agents
- human approval for dangerous actions
- audit query for replay

### Module focus

#### `audit-log`

Implement a structured audit layer capable of recording:

- run id
- agent id
- parent-child linkage
- action attempted
- action result
- timestamps

The first version may be simple, but it must be queryable.

#### approval handling

Implement an `Interrupted` or equivalent result state for dangerous actions.

MVP dangerous action set:

- `restart_service`

#### agent registry

Implement target lookup for `CallAgent`.

This registry only needs enough functionality to support:

- `ops-investigator`
- `ops-executor`

#### permission-engine

Add child-capability computation:

`delegated_caps = caller_caps ∩ target_caps`

### Key deliverables

- successful read-only investigation flow
- denied direct restart from `ops-investigator`
- delegated `ops-executor` invocation
- approval-required restart flow
- audit query that reconstructs the chain

### What not to do

- do not add many tool types beyond the MVP four
- do not build a general approval workflow engine
- do not build complex UI for audit exploration

### Exit criteria

- the complete MVP user flow works end to end
- the restart path requires approval
- the audit view can reconstruct the run and child call chain
- the sub-agent path proves non-amplifying delegation

## Phase 3: Demo Hardening And Internal Usability

### Goal

Make the MVP technically and operationally clear enough for internal demos,
design reviews, and early stakeholder validation.

### Primary modules

- audit query layer
- demo fixture layer
- example tools
- documentation and demo scripts

### Scope

This phase is not about feature breadth. It is about reliability,
repeatability, and explainability.

### Module focus

#### demo fixtures

Create stable demo scenarios for:

- healthy investigation
- denied action
- delegated child execution
- approval and completion

#### audit query usability

Make the audit output easy to inspect in a way that supports the product story.

Minimum needs:

- group by run
- show parent-child sequence
- show final status of each step

#### example tool adapters

Provide lightweight adapters or mocks for:

- logs
- metrics
- incident creation
- restart action

These do not need production integration depth. They need demo reliability.

### Key deliverables

- stable internal demo environment
- repeatable output for product walkthroughs
- documentation that explains:
  - what happened
  - why an action was denied or interrupted
  - how the audit reconstruction works

### What not to do

- do not confuse demo hardening with scale hardening
- do not broaden scope into many additional internal systems

### Exit criteria

- the team can run the MVP demo repeatably
- product and architecture stakeholders can understand the governance story
- the output is good enough for customer discovery conversations

## Phase 4: Post-MVP Technical Expansion

### Goal

Extend the governed core without losing the narrow thesis.

### Expansion themes

- richer policy model
- stronger audit semantics
- additional operational tool classes
- stronger execution isolation
- better approval workflows
- packaging for wider platform use

### Candidate module expansions

#### policy expansion

- finer-grained capability scopes
- environment-based constraints
- action-class-based approval policy

#### audit expansion

- integrity chaining
- stronger query filters
- incident-linked views
- export formats for internal review

#### execution expansion

- more executor classes
- stronger sandbox boundaries
- timeout and cancellation mechanics

#### adoption expansion

- CLI and service packaging
- reference integrations
- operational dashboards

### Guardrail

Do not expand into a general multi-agent platform until the internal ops
governance use case is clearly proven.

## Recommended Module Order

If the team needs a concrete implementation order inside the roadmap, use this:

1. `agent-protocol` MVP subset
2. `kernel-api`
3. `kernel-core` dispatch skeleton
4. `permission-engine`
5. `audit-log`
6. approval interrupt handling
7. agent registry and `CallAgent`
8. demo fixtures and example tools
9. audit query usability
10. post-MVP hardening modules

## Dependency Notes

- `kernel-api` depends on a stable enough MVP object model
- `kernel-core` depends on `kernel-api` and protocol types
- `permission-engine` must be integrated early because the product story
  collapses without explicit enforcement
- `audit-log` must land before the MVP is considered credible
- approval handling depends on action classification and result modeling
- child-agent delegation depends on both agent registry and permission logic

## Technical Risks

### Risk 1: Too much protocol breadth too early

If the team tries to implement the full protocol before proving the MVP, the
project may stall in infrastructure work that is not required for the first
demo.

### Risk 2: Audit treated as a secondary concern

If audit lands late or remains shallow, the product story loses much of its
enterprise value.

### Risk 3: Approval logic treated as UI-only

The interrupt-and-approve model is part of the Kernel behavior, not merely a
presentation-layer feature.

### Risk 4: Overbuilding execution isolation too early

Strong isolation matters, but the MVP should first prove governed control flow.
Isolation can deepen in later phases.

## Recommended Near-Term Sequence

For the immediate next implementation cycle:

1. finalize MVP protocol/data model
2. define the minimal `kernel-api` surface
3. implement `invoke` with permission enforcement
4. implement audit recording
5. implement dangerous-action interrupt handling
6. implement `CallAgent` with capability intersection
7. package the demo flow and audit replay

## Roadmap Summary

The roadmap should be understood as:

- Phase 0: define the minimum shape
- Phase 1: build the controlled path
- Phase 2: make it governable
- Phase 3: make it demo-credible
- Phase 4: deepen the platform without losing the thesis

The central discipline is to keep governance features ahead of platform
breadth. The first win is not "more agent capability." The first win is
"bounded, auditable, approval-aware internal ops execution."
