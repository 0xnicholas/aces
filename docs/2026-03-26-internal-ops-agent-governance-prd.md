# Internal Ops Agent Governance PRD

## Document Purpose

This PRD defines the first productized version of the Internal Ops Agent
Governance concept. It is an internal product document for aligning product,
architecture, and implementation decisions around a narrowly scoped MVP.

This document is intentionally focused on:

- the first high-value target use case
- the minimum functional surface needed to validate the thesis
- the boundaries that keep the MVP tractable
- the next-stage expansion path after the MVP proves value

## Product Summary

Internal Ops Agent Governance is a governance layer for internal operations
agents that need controlled access to production-facing tools.

The product does not aim to improve agent reasoning quality directly. It aims
to make high-privilege agent behavior governable through:

- explicit capability control
- a single controlled invocation path
- structured auditability
- bounded sub-agent delegation
- human approval on dangerous actions

## Problem Statement

Teams increasingly want to use agents for internal operations work such as:

- reading logs
- inspecting metrics
- opening incidents
- executing remediation actions

The current blocker is not only model quality. The real blocker is operational
trust.

Current failure modes in typical internal-agent deployments:

- permissions are enforced in application code rather than by an external
  control layer
- tool access is often too broad once an agent is connected to internal systems
- sub-agent delegation makes capability boundaries difficult to reason about
- dangerous actions do not have a consistent interrupt-and-approve mechanism
- post-incident investigation depends on fragmented logs rather than structured
  action records

The result is that teams can build impressive demos but hesitate to allow those
agents to interact with real production systems.

## Product Thesis

The key blocker for production internal agents is not only intelligence. It is
governability.

If we provide a governance layer that can reliably enforce permissions, control
tool access, interrupt dangerous actions, and reconstruct the full execution
chain, then operations teams can allow agents to perform bounded production
work with meaningfully lower risk.

## Goals

### Primary goal

Demonstrate that an internal operations agent can perform useful production-
adjacent work through a controlled path without receiving unchecked system
authority.

### Secondary goals

- Prove that bounded sub-agent delegation is understandable and enforceable.
- Prove that human approval can be inserted into dangerous actions without
  breaking the overall workflow.
- Prove that the resulting audit trail is detailed enough for investigation and
  internal review.

## Non-Goals

The MVP will not attempt to be:

- a general-purpose agent platform
- a workflow builder
- a broad SDK ecosystem
- a fully featured UI product
- a complete enterprise security platform
- a consumer assistant product
- a replacement for existing orchestration frameworks

## Target Users

### Primary users

- platform engineering teams
- site reliability engineering teams
- internal operations teams
- internal AI platform teams

### Internal sponsors / decision-makers

- head of platform engineering
- SRE lead
- security lead
- platform architecture lead
- AI platform owner

## User Needs

The target user needs to be able to say:

- this agent may read logs and metrics, but may not restart services directly
- this child agent may execute only what the caller is already allowed to
  delegate
- this high-risk action must stop and wait for approval
- this action was denied for a specific reason
- I can reconstruct the full chain of what happened after an incident

## MVP Use Case

The MVP use case is a production issue investigation and controlled remediation
flow.

Canonical prompt:

`service-a has elevated 500 errors in the last 10 minutes, investigate and help resolve`

The MVP should support the following operational narrative:

1. Investigate the issue using read-only tools.
2. Determine whether remediation is needed.
3. Request execution through a bounded sub-agent.
4. Interrupt dangerous execution pending human approval.
5. Complete the action after approval.
6. Query the audit trail for the full call chain.

## MVP Scope

### Included

- agent registration with explicit capability binding
- a controlled action path through `invoke`
- capability enforcement before tool execution
- sub-agent invocation with capability intersection
- structured audit logging
- interrupt-and-approve handling for dangerous actions
- audit query for post-hoc inspection

### Excluded

- generalized orchestration authoring
- policy DSL sophistication beyond the MVP needs
- large-scale multi-tenant deployment concerns
- end-user facing analytics UI
- broad tool marketplace support
- generalized sandbox hardening for every runtime target
- full protocol implementation ecosystem

## Functional Surface

### Agents

The MVP defines two agents:

#### `ops-investigator`

Purpose:

- inspect logs
- inspect metrics
- open incidents

Allowed capabilities:

- log read
- metric read
- incident creation

Denied capabilities:

- direct service restart

#### `ops-executor`

Purpose:

- execute bounded remediation actions when delegated

Allowed capabilities:

- service restart

Important constraint:

- execution remains constrained by delegated capability intersection and
  approval policy

### Tools

The MVP defines four tools:

- `read_logs`
- `read_metrics`
- `restart_service`
- `open_incident`

These four tools are sufficient to validate:

- read-only investigation
- privileged execution boundaries
- approval gating
- audit chain reconstruction

## Core Product Concepts

### CapabilitySet

The explicit authority boundary for an agent.

### Action

A typed request submitted through the controlled path.

### KernelHandle

The controlled identity handle for acting on behalf of an agent.

### ActionResult

The structured output of a controlled action.

### AuditEntry

A structured record of attempted or completed work.

### ApprovalInterrupt

A paused execution state for high-risk actions that require human approval.

## User Flow

### Main flow

1. A request is routed to `ops-investigator`.
2. The agent invokes `read_metrics`.
3. The agent invokes `read_logs`.
4. The system determines a restart may be needed.
5. The system issues `CallAgent(target = ops-executor)`.
6. Delegated capabilities are computed using:

`delegated_caps = caller_caps ∩ target_caps`

7. `ops-executor` attempts `restart_service`.
8. The Kernel returns an interrupt rather than executing immediately.
9. A human approves the action.
10. The restart executes through the controlled path.
11. The result is returned.
12. The audit log reconstructs the run and child invocation chain.

### Policy denial flow

1. `ops-investigator` attempts `restart_service` directly.
2. The action is denied before execution.
3. A structured denial result is returned.
4. The denial is visible in audit.

### Approval rejection flow

1. `ops-executor` requests `restart_service`.
2. The action is interrupted.
3. A human rejects the action.
4. The action is terminated without execution.
5. The interrupt and rejection are visible in audit.

## Functional Requirements

### FR1: Single controlled invocation path

All production-facing tool actions in the MVP must enter through one controlled
invocation path.

Acceptance criteria:

- all tool actions demonstrated in the MVP pass through the same invocation
  boundary
- no demo path directly executes a tool without that boundary

### FR2: Explicit capability enforcement

Every attempted action must be checked against the active capability set before
execution.

Acceptance criteria:

- unauthorized actions are blocked
- denials are structured and inspectable
- denials are included in audit

### FR3: Non-amplifying sub-agent delegation

Child execution must be constrained by the intersection of caller and target
capabilities.

Acceptance criteria:

- child capabilities are computed using capability intersection
- child execution never inherits caller full power by default
- child execution never inherits target full power by default

### FR4: Human approval for dangerous actions

High-risk actions must return an interrupt instead of executing immediately.

Acceptance criteria:

- `restart_service` does not execute without approval
- approval allows execution to continue
- rejection prevents execution
- interrupt state is represented in structured output and audit

### FR5: Structured auditability

Every attempted action must generate a structured audit record.

Acceptance criteria:

- successful actions are recorded
- denied actions are recorded
- interrupted actions are recorded
- parent/child relationships are reconstructable

### FR6: Run-level investigation support

Operators must be able to inspect the full execution chain for a run.

Acceptance criteria:

- audit query can group actions by run
- parent/child linkage is visible
- the final state of each action is visible

## Experience Requirements

The MVP experience must make the following visible during a demo:

- which agent is acting
- which tool is being requested
- whether the request is allowed, denied, or interrupted
- when a child agent is invoked
- what approval is being requested
- how the resulting audit chain can be inspected

This visibility matters because the product thesis depends on user trust in the
governance layer, not only in the final output text.

## Data Requirements

At minimum, audit records should support:

- run identifier
- acting agent identifier
- parent-child linkage
- action type
- target/tool
- outcome
- timestamp

If capability scope can be safely represented, it should also be included or
hashed in a way that supports later debugging.

## Security and Governance Requirements

- the governance layer must not rely on prompt wording as the primary
  permission boundary
- high-risk action control must be external to the agent's own reasoning loop
- sub-agent delegation must not amplify authority
- audit records must be treated as first-class product output, not incidental
  logs

## Metrics for MVP Success

### Product success criteria

- a user can complete a read-only investigation flow
- a direct unauthorized restart is blocked
- a delegated restart is interrupted for approval
- an approved restart completes successfully
- the full execution chain can be queried afterward

### Internal evaluation criteria

- the product story is understandable in a live demo
- the governance layer's role is obvious without deep explanation
- the product can be positioned as infrastructure rather than just another
  agent wrapper

## Demo Plan

### Demo 1: Investigation

Show:

- issue intake
- `read_metrics`
- `read_logs`
- diagnosis

### Demo 2: Denial

Show:

- direct restart request from `ops-investigator`
- policy denial
- denial in audit

### Demo 3: Delegation

Show:

- handoff to `ops-executor`
- delegated capability boundary

### Demo 4: Approval

Show:

- interrupted restart
- approval step
- completed restart
- complete audit trail

## Risks

### Risk 1: Product feels too infrastructure-heavy

If the demo over-emphasizes internal model mechanics and under-emphasizes the
user problem, the product may appear abstract.

Mitigation:

- always anchor the story in a real ops incident workflow

### Risk 2: Governance value is hidden behind implementation detail

If the demo only shows final agent output, the audience may not understand why
the Kernel matters.

Mitigation:

- make denials, interruptions, and audit traces visible during the demo

### Risk 3: Scope creep into a generic agent platform

The team may be tempted to add orchestration, UI, tool breadth, or broad
developer platform features too early.

Mitigation:

- keep the MVP pinned to one operations use case and four tools

## Open Questions

- What is the exact approval actor in the MVP: CLI operator, web operator, or
  synthetic demo approver?
- How much audit detail should be shown in the demo versus stored internally?
- Should incident creation be treated as always safe in the MVP, or as a second
  class of controlled action?
- What level of execution isolation is necessary for a credible MVP demo?

## Post-MVP Expansion

The next stage should deepen product value without abandoning the narrow
governance focus.

### Stage 2 goals

- richer policy controls
- more operational tools
- better approval workflow ergonomics
- stronger audit query experience
- clearer packaging for platform-team adoption

### Candidate Stage 2 capabilities

- finer-grained capability scopes
- approval policy by tool, environment, or risk class
- incident-linked audit views
- more than one executor class
- controlled memory/context access
- packaged integrations for common internal ops systems

### Stage 2 guardrail

Do not broaden into a general-purpose agent builder until the governance story
for the internal ops case is proven compelling.

## Product Decision Summary

This MVP is intentionally narrow.

It is not trying to prove that agents can do everything. It is trying to prove
that a high-risk internal operations agent can be allowed to do something real
without turning governance into an afterthought.
