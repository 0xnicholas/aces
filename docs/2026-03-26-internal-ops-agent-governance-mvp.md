# Internal Ops Agent Governance MVP

## One-Line Definition

A governance layer that allows internal operations agents to access production
tools safely through permission control, call auditing, sub-agent capability
reduction, and human approval for high-risk actions.

## Product Positioning

This product does not try to make agents more intelligent. It makes high-
privilege agents governable enough to be used against real internal systems.

The core promise is:

`Let internal ops agents do useful work without giving them unbounded power.`

## Target Users

Primary users:

- Platform engineering teams
- SRE and operations teams
- Security teams
- Internal AI platform teams

Economic buyer / internal sponsor:

- Head of platform engineering
- SRE lead
- Security lead
- AI platform owner

## Core User Problem

Teams want to use agents for internal operations work such as reading logs,
checking metrics, opening incidents, or restarting services. The blocker is
not only model quality. The blocker is governance.

Typical objections:

- We do not want an agent to have unchecked production access.
- We cannot trust prompt-level restrictions as our main guardrail.
- We need to know exactly what happened after an incident.
- We need high-risk actions to stop for approval.
- We need sub-agents to stay inside the caller's authority boundary.

## MVP Goal

Prove that an internal operations agent can interact with production-facing
tools through a single controlled path and remain auditable, interruptible, and
bounded by explicit capabilities.

## MVP Scope

Included:

- Agent registration with capability binding
- Unified `invoke` path for tool actions
- Capability checks before tool execution
- Sub-agent invocation with capability intersection
- Structured audit logging
- Human approval for high-risk actions
- Audit query for replay and investigation

Excluded:

- General-purpose workflow builder
- Broad SDK matrix
- Large multi-tenant platform features
- Full UI product surface
- Complete sandbox implementation for all deployment targets
- General consumer assistant use cases

## MVP Domain

The first domain is internal operations.

Representative use case:

- Investigate a production issue
- Read logs
- Read metrics
- Open an incident
- Request a service restart
- Require human confirmation before restart

## Core Product Objects

### Agent

A bounded execution subject that operates under an explicit capability set.

### CapabilitySet

The set of allowed actions for an agent. This is the hard boundary for what
the agent may attempt through the Kernel.

### Action

A typed request such as:

- `read_logs`
- `read_metrics`
- `restart_service`
- `open_incident`
- `CallAgent`

### KernelHandle

The controlled identity reference used to act on behalf of an agent.

### ActionResult

The structured result returned after a controlled action attempt.

### AuditEntry

A structured record of an action, including who initiated it, what was
attempted, the result, and the call-chain linkage.

### ApprovalInterrupt

A suspended state indicating that execution may continue only after human
confirmation.

## MVP Tools

The first demo only needs four tools:

- `read_logs`
- `read_metrics`
- `restart_service`
- `open_incident`

These are sufficient to demonstrate read-only investigation, bounded
escalation, approval gating, and auditability.

## MVP Agents

### `ops-investigator`

Purpose:

- inspect logs
- inspect metrics
- open an incident

Allowed capabilities:

- log read
- metric read
- incident creation

Not allowed:

- direct service restart

### `ops-executor`

Purpose:

- execute an operational action when explicitly delegated

Allowed capabilities:

- service restart

Constraint:

- execution still depends on delegated capability intersection and approval
  policy

## MVP User Flow

Example prompt:

`service-a has elevated 500 errors in the last 10 minutes, investigate and help resolve`

Expected flow:

1. The user request is routed to `ops-investigator`.
2. The agent invokes `read_metrics` through the Kernel.
3. The agent invokes `read_logs` through the Kernel.
4. The agent concludes that restart may be required.
5. The agent requests `CallAgent(target = ops-executor)`.
6. The Kernel computes `delegated_caps = caller_caps ∩ target_caps`.
7. `ops-executor` attempts `restart_service`.
8. The Kernel classifies restart as high risk and returns an interrupt.
9. A human approves or rejects the action.
10. If approved, the restart executes through the controlled path.
11. The result is returned.
12. The full chain is queryable through audit.

## MVP Functional Requirements

### 1. Capability enforcement

- Every tool action must be checked before execution.
- Unauthorized actions must be blocked.
- The denial must be auditable.

### 2. Single controlled path

- Production-facing actions must pass through a single invocation path.
- Direct tool execution outside that path is out of scope for the MVP.

### 3. Sub-agent capability reduction

- Child execution must use:

`delegated_caps = caller_caps ∩ target_caps`

- The child must not inherit the caller's full capability set.
- The child must not automatically gain the target's full capability set.

### 4. Human approval

- High-risk actions must return an interrupt instead of executing immediately.
- Approval must be explicit.
- Rejection must terminate the pending high-risk action cleanly.

### 5. Auditability

- Every controlled action must create an audit record.
- The record must include enough structure to reconstruct parent/child calls.
- Failures and policy denials must also be visible in audit.

### 6. Replay and investigation support

- The operator must be able to inspect the action chain for a single incident
  or run.
- The audit view must make it possible to answer:
  - which agent acted
  - which sub-agent was called
  - which tool was requested
  - whether the action was approved, denied, or completed

## MVP Success Criteria

The MVP is successful if it demonstrates all of the following:

- A read-only investigation flow works end to end.
- An unauthorized action is blocked.
- A sub-agent receives only intersected capabilities.
- A restart action is interrupted for human approval.
- An approved restart completes successfully.
- The full run can be reconstructed from audit records.

## Demo Script

### Demo 1: Normal investigation

- Ask the system to investigate a service issue.
- Show successful `read_metrics` and `read_logs` actions.
- Show the resulting diagnosis.

### Demo 2: Unauthorized action blocked

- Attempt a restart directly from `ops-investigator`.
- Show the policy denial and corresponding audit entry.

### Demo 3: Sub-agent execution

- Route the action to `ops-executor`.
- Show delegated capability reduction.

### Demo 4: Human approval

- Attempt `restart_service`.
- Show the interrupt.
- Approve it.
- Show successful completion and the resulting audit chain.

## Why This MVP Matters

Without a governance layer, teams can build an operations agent that appears
useful but is difficult to trust in production. Permissions often live inside
application code, agent-to-agent delegation is hard to reason about, and logs
are fragmented.

This MVP demonstrates a more credible path:

- explicit permissions
- forced control path
- auditable execution
- approval on dangerous actions
- bounded multi-agent collaboration

## Product Thesis

LangGraph-style orchestration and MCP-style tool access help agents do work.
This product addresses the missing layer that helps enterprises let those
agents touch real systems safely.

The product thesis is:

`The blocker for production internal agents is not only intelligence. It is governability.`

## Follow-On Milestones

After the MVP, likely next steps are:

- richer policy model
- more operational tools
- better audit query experience
- more explicit approval workflows
- stronger execution isolation
- packaging for platform-team adoption
