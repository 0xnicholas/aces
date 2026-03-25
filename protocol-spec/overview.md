# Agent Protocol Specification

Version: 0.1-draft  
Status: Working Draft  
Licence: MIT  
Repository: https://github.com/agent-kernel/agent-protocol

---

## Abstract

The Agent Protocol is an open specification that defines the communication
contract between Agent Runtimes and the systems that govern them. It
specifies the interface families, data types, error taxonomy, and mandatory
semantic constraints that any conformant implementation must satisfy.

The Protocol is implementation-independent. The Agent Kernel is the
reference implementation, but any system that passes the compliance test
suite is a conformant implementation. The compliance test suite, not any
particular implementation's source code, is the authoritative definition
of correctness.

---

## Status of this document

This is a working draft. Sections marked `[STABLE]` have frozen semantics
and will not change without a version increment and a deprecation period.
Sections marked `[DRAFT]` may change before the 1.0 release.

---

## Table of contents

1. [Motivation](#1-motivation)
2. [Scope and non-scope](#2-scope-and-non-scope)
3. [Terminology](#3-terminology)
4. [Data model](#4-data-model)
5. [Interface families](#5-interface-families)
   - 5.1 AgentLifecycle
   - 5.2 Invocation
   - 5.3 ContextIO
   - 5.4 SignalEvent
   - 5.5 ObservabilityHook
6. [Error taxonomy](#6-error-taxonomy)
7. [Mandatory semantic constraints](#7-mandatory-semantic-constraints)
8. [Agent-to-Agent calling convention](#8-agent-to-agent-calling-convention)
9. [Capability model](#9-capability-model)
10. [Audit log schema](#10-audit-log-schema)
11. [Compliance](#11-compliance)
12. [Versioning and evolution](#12-versioning-and-evolution)
13. [Glossary](#13-glossary)

---

## 1. Motivation

`[STABLE]`

Agent systems today have two distinct infrastructure layers:

**What exists** — Frameworks and Runtimes that make Agents capable: they
orchestrate multi-Agent workflows, manage reasoning loops, call tools, and
handle context. These layers are well-served by existing open-source projects.

**What is missing** — A mandatory, external enforcement layer that governs
what Agents are permitted to do, records what they actually did, and
prevents any execution path from bypassing these constraints.

The Agent Protocol defines the contract between Runtimes (which produce
actions) and governance implementations (which enforce, record, and
isolate those actions). It exists so that governance can be provided by an
external system — not by the Agent itself — and so that the rules of that
governance are openly specified and independently verifiable.

The analogy to POSIX is deliberate. POSIX defined the interface between
programmes and operating systems, enabling portability across implementations.
The Agent Protocol defines the interface between Agents and governance
systems, enabling portability across enforcement implementations and
auditability across deployments.

---

## 2. Scope and non-scope

`[STABLE]`

### In scope

- The five interface families that Runtimes use to interact with a
  governance implementation.
- The data types shared across all interfaces.
- The error taxonomy that implementations must return.
- The six mandatory semantic constraints that all implementations must
  satisfy.
- The Agent-to-Agent calling convention and capability delegation rules.
- The audit log schema.
- The compliance test suite (normative).

### Out of scope

- How a governance implementation internally enforces policy (scheduling
  algorithms, sandbox technology, storage engines).
- How Frameworks orchestrate multiple Agents at the application level.
- How models perform inference or manage context windows.
- Application-level Agent behaviour, goals, or alignment.
- Transport protocols (gRPC, Unix socket, in-process — all are permitted).

---

## 3. Terminology

`[STABLE]`

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**,
**SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **MAY**, and **OPTIONAL** in
this document are to be interpreted as described in RFC 2119.

**Agent** — an autonomous software entity that produces actions in pursuit
of a goal. An Agent interacts with the outside world exclusively through
the Protocol interfaces.

**Runtime** — the execution environment for a single Agent instance.
Manages the reasoning loop, context, and session state. Calls the Protocol
interfaces on behalf of the Agent.

**Governance implementation** — a system that implements the Protocol
interfaces and provides enforcement of the six mandatory constraints.
The Agent Kernel is the reference governance implementation.

**Capability** — an explicit, unforgeable token granting an Agent the
right to perform a specific class of action. Capabilities are the sole
basis for access decisions in this Protocol.

**CapabilitySet** — the complete set of capabilities bound to an Agent
instance at spawn time. Immutable for the lifetime of the instance.

**KernelHandle** — an opaque reference to a spawned Agent instance,
returned by `spawn` and required for all subsequent Protocol calls.
Implementations MUST treat handles as unforgeable; callers MUST treat
them as opaque.

**RunId** — a globally unique identifier for a top-level Agent invocation.
Propagated through the entire call tree, including sub-Agent calls.

**SpanId** — a locally unique identifier for a single step within a run.
Forms a tree with `parent_span_id` linking child to parent.

**Action** — a typed request produced by a Runtime and submitted to the
governance implementation via `invoke`. Examples: tool call, LLM inference
request, memory read, Agent call.

**Audit entry** — an immutable record of a single action, written by the
governance implementation to the audit log before the action's result is
returned to the caller.

---

## 4. Data model

`[STABLE]`

This section defines the shared types used across all five interface
families. All field names use snake_case. All timestamps are UTC with
nanosecond precision represented as a 64-bit unsigned integer.

### 4.1 Identity types

```
AgentId   ::= UUID v4
RunId     ::= UUID v4
SpanId    ::= UUID v4
HandleId  ::= opaque bytes[32]   // implementation-defined; must be unguessable
```

`AgentId` identifies an Agent definition (the blueprint).
`RunId` identifies a single top-level execution.
`SpanId` identifies one step within a run.
`HandleId` is the unforgeable token inside a `KernelHandle`.

### 4.2 KernelHandle

```
KernelHandle {
  agent_id:   AgentId
  handle_id:  HandleId      // opaque; implementations MUST NOT expose internals
  issued_at:  Timestamp
  expires_at: Timestamp?    // null = no expiry
}
```

A `KernelHandle` is valid until explicitly revoked via `terminate` or
until `expires_at` is reached, whichever comes first. An expired or
revoked handle MUST be rejected with `InvalidHandle` on any subsequent
call.

### 4.3 Capability

```
Capability ::=
  | ToolRead    { tool_id: String }
  | ToolWrite   { tool_id: String }
  | ToolExec    { tool_id: String }
  | MemoryRead  { scope: ContextScope }
  | MemoryWrite { scope: ContextScope }
  | LlmCall     { model_family: String? }   // null = any model
  | AgentCall   { target_agent_id: AgentId? } // null = any agent
  | HttpFetch   { domain_allowlist: Vec<String> }
  | Notify      { channel: String }

ContextScope ::= Session | Agent | Shared
```

Capabilities are additive. A `CapabilitySet` is a set of zero or more
`Capability` values. The empty set means the Agent may take no actions.
There is no wildcard or superuser capability. All capabilities are
explicit.

### 4.4 Action

```
Action ::=
  | InvokeTool  { tool_id: String, params: Bytes, idempotency_key: String? }
  | LlmInfer    { model: String, messages: Vec<Message>, params: InferParams }
  | MemoryRead  { key: ContextKey, scope: ContextScope }
  | MemoryWrite { key: ContextKey, value: Bytes, scope: ContextScope }
  | MemorySearch{ query: SemanticQuery, scope: ContextScope }
  | CallAgent   { target_id: AgentId, payload: Bytes, caps_hint: CapabilitySet? }
  | Notify      { channel: String, body: Bytes }
```

`caps_hint` in `CallAgent` is advisory. The governance implementation
MUST compute the actual delegated capabilities itself using the
intersection rule (§9.3) and MUST NOT trust the hint.

### 4.5 ActionResult

```
ActionResult {
  run_id:    RunId
  span_id:   SpanId
  status:    Ok | Err
  payload:   Bytes?           // present on Ok
  error:     ProtocolError?   // present on Err
  duration:  Duration         // wall-clock time of the action itself
}
```

### 4.6 AgentDef

```
AgentDef {
  agent_id:    AgentId
  name:        String
  description: String?
  model:       String?        // advisory; governance impl may override
  max_steps:   u32?
  timeout:     Duration?
}
```

---

## 5. Interface families

### 5.1 AgentLifecycle

`[STABLE]`

**Purpose** — manage the existence of Agent instances.  
**POSIX analogue** — `fork` / `exec` / `kill`

```
spawn(
  def:  AgentDef,
  caps: CapabilitySet,
) → Result<KernelHandle, ProtocolError>
```

Creates a new Agent instance. The returned `KernelHandle` is valid
immediately. The `CapabilitySet` is atomically bound at spawn time;
there is no intermediate state where the handle exists but capabilities
are not yet set. Implementations MUST NOT permit a two-phase
"create then grant" pattern.

```
suspend(
  handle: KernelHandle,
) → Result<Checkpoint, ProtocolError>
```

Suspends execution and returns a `Checkpoint` that can be passed to
`resume`. In-flight actions are allowed to complete before suspension.
Implementations MUST flush the audit log before returning the
`Checkpoint`.

```
resume(
  checkpoint: Checkpoint,
) → Result<KernelHandle, ProtocolError>
```

Restores an Agent from a `Checkpoint`. The resumed instance has the
same `AgentId` and `CapabilitySet` as the suspended instance. A new
`HandleId` is issued.

```
terminate(
  handle: KernelHandle,
  reason: TerminationReason,
) → Result<RunSummary, ProtocolError>
```

Permanently revokes the handle. In-flight actions receive `Cancelled`.
A `Revocation` entry is written to the audit log. The handle MUST be
rejected on any subsequent call with `InvalidHandle`.

```
TerminationReason ::= UserRequested | Timeout | PolicyViolation | Error

RunSummary {
  agent_id:      AgentId
  run_id:        RunId
  actions_taken: u64
  started_at:    Timestamp
  ended_at:      Timestamp
  termination:   TerminationReason
}
```

---

### 5.2 Invocation

`[STABLE]`

**Purpose** — the single interception point for all Agent actions.  
**POSIX analogue** — `read` / `write`

This is the most critical interface in the Protocol. All action types —
tool calls, LLM inference, memory access, Agent-to-Agent calls — are
submitted through `invoke`. The governance implementation MUST process
every `invoke` call through the full enforcement sequence:
check → schedule → sandbox → audit.

```
invoke(
  handle: KernelHandle,
  action: Action,
  run_id: RunId,
  span_id: SpanId,
  parent_span_id: SpanId?,
) → Result<ActionResult, ProtocolError>
```

`run_id` and `span_id` are assigned by the Runtime. The governance
implementation MUST validate that `run_id` matches the active run for
this handle and MUST record `span_id` and `parent_span_id` in the audit
entry.

```
invoke_stream(
  handle: KernelHandle,
  action: Action,
  run_id: RunId,
  span_id: SpanId,
  parent_span_id: SpanId?,
) → Result<Stream<Chunk, ProtocolError>, ProtocolError>
```

For actions that produce streaming output (primarily `LlmInfer`). The
audit entry MUST be written when the stream is opened, not when it
completes. The audit entry's `result.status` is `Pending` until the
stream closes, at which point a completion entry MUST be written.

```
cancel(
  run_id: RunId,
) → Result<(), ProtocolError>
```

Cancels all in-flight `invoke` and `invoke_stream` calls associated with
`run_id`, including any child spans in sub-Agent calls. The cancellation
MUST propagate to the full call tree (§7.5). Each cancelled action
receives `Cancelled` as its error and MUST have a `Cancelled` audit entry
written.

---

### 5.3 ContextIO

`[DRAFT]`

**Purpose** — standardised read and write access to Agent memory.  
**POSIX analogue** — `mmap` / `lseek`

```
context_read(
  handle: KernelHandle,
  key:    ContextKey,
  scope:  ContextScope,
) → Result<ContextValue, ProtocolError>
```

```
context_write(
  handle: KernelHandle,
  key:    ContextKey,
  value:  ContextValue,
  scope:  ContextScope,
) → Result<(), ProtocolError>
```

```
context_search(
  handle: KernelHandle,
  query:  SemanticQuery,
  scope:  ContextScope,
  limit:  u32,
) → Result<Vec<ContextValue>, ProtocolError>
```

```
snapshot(
  handle: KernelHandle,
) → Result<ContextSnapshot, ProtocolError>
```

```
restore(
  handle:   KernelHandle,
  snapshot: ContextSnapshot,
) → Result<(), ProtocolError>
```

**Scope semantics**

| Scope | Lifetime | Access | Requires capability |
|---|---|---|---|
| `Session` | Current run only | This Agent only | `MemoryRead` / `MemoryWrite { scope: Session }` |
| `Agent` | Across runs | This Agent only | `MemoryRead` / `MemoryWrite { scope: Agent }` |
| `Shared` | Across runs | Multiple Agents | `MemoryRead` / `MemoryWrite { scope: Shared }` |

Writing to `Shared` scope MUST require an explicit `MemoryWrite { scope: Shared }` capability. Implementations MUST NOT infer cross-Agent memory access from a lower-scope capability.

---

### 5.4 SignalEvent

`[DRAFT]`

**Purpose** — event emission, subscription, and human-in-the-loop interruption.  
**POSIX analogue** — `signal` / `wait`

```
emit(
  handle: KernelHandle,
  event:  AgentEvent,
) → Result<(), ProtocolError>
```

```
subscribe(
  handle: KernelHandle,
  filter: EventFilter,
) → Result<Stream<AgentEvent, ProtocolError>, ProtocolError>
```

```
interrupt(
  handle: KernelHandle,
  signal: AgentSignal,
  reason: String,
) → Result<InterruptToken, ProtocolError>
```

Suspends the Agent's current `invoke` execution and places it in the
`Interrupted` state. The caller receives an `InterruptToken` that must
be passed to `confirm` or `reject` to resume or abort.

Implementations MUST NOT time out an interrupted Agent silently.
If a timeout is configured, the Agent MUST receive `Timeout` as the
error on the interrupted `invoke`, and a `Timeout` audit entry MUST be
written.

```
confirm(
  token:    InterruptToken,
  decision: HumanDecision,
) → Result<(), ProtocolError>

HumanDecision ::=
  | Approve
  | ApproveWithModification { modified_action: Action }
  | Reject { reason: String }
```

If `Approve` or `ApproveWithModification`, the interrupted `invoke`
resumes with the original or modified action. If `Reject`, the
interrupted `invoke` returns `Interrupted { rejected: true }` to the
Runtime.

```
AgentSignal ::=
  | HumanConfirmationRequired { risk_level: Low | Medium | High | Critical }
  | ExternalEvent { source: String, payload: Bytes }
  | PolicyAlert   { policy_id: String, detail: String }
```

---

### 5.5 ObservabilityHook

`[STABLE]`

**Purpose** — mandatory trace emission on every invocation.  
**POSIX analogue** — `ptrace`

This interface is not optional. Any conformant implementation MUST call
these hooks at the specified points in the `invoke` lifecycle. Callers
MUST NOT be able to suppress hook emission. The hooks are the foundation
of the forced observability constraint (§7.3).

```
on_invoke_begin(
  run_id:         RunId,
  span_id:        SpanId,
  parent_span_id: SpanId?,
  action:         Action,
  caps_scope:     CapabilitySet,
) → SpanContext
```

Called before any capability check. The `SpanContext` is threaded
through subsequent hooks.

```
on_invoke_end(
  ctx:    SpanContext,
  result: ActionResult,
) → ()
```

Called after the audit entry is written and before the result is
returned to the Runtime.

```
on_state_change(
  handle: KernelHandle,
  event:  LifecycleEvent,
) → ()

LifecycleEvent ::= Spawned | Suspended | Resumed | Terminated | Revoked
```

```
on_error(
  run_id:  RunId,
  span_id: SpanId,
  error:   ProtocolError,
) → ()
```

Called for every error, including `PolicyViolation`. Errors must be
observable even when the action that caused them is not.

---

## 6. Error taxonomy

`[STABLE]`

All errors returned across the Protocol boundary MUST be one of the
following variants. Implementations MUST NOT return untyped strings or
implementation-specific error types to callers.

```
ProtocolError ::=
  | PolicyViolation {
      action:      Action,
      missing_cap: Capability,
      agent_id:    AgentId,
    }
  | ResourceExhausted {
      resource:    ResourceKind,
      retry_after: Duration?,
    }
  | Interrupted {
      token:    InterruptToken,
      rejected: bool,
    }
  | Timeout {
      action:   Action,
      limit:    Duration,
    }
  | ContextOverflow {
      current:  u64,    // tokens
      limit:    u64,
    }
  | Cancelled {
      run_id:   RunId,
    }
  | AuditIntegrityError {
      seq:      u64,    // sequence number of the broken entry
      expected: Bytes,  // expected hash
      actual:   Bytes,  // found hash
    }
  | InvalidHandle {
      reason: HandleInvalidReason,
    }
  | ProtocolViolation {
      detail: String,   // implementation sent a malformed request
    }

ResourceKind  ::= LlmConcurrency | ToolCallRate | ContextBudget | ComputeQuota
HandleInvalidReason ::= Expired | Revoked | Unrecognised
```

**Ordering rule** — when multiple errors could apply, implementations
MUST return them in this priority order:

1. `InvalidHandle` — the handle is not valid; no further evaluation.
2. `PolicyViolation` — the handle is valid but lacks capability.
3. `ResourceExhausted` — the handle has capability but resources are unavailable.
4. All other variants.

This ordering ensures that callers can build reliable error-handling
logic without ambiguity.

---

## 7. Mandatory semantic constraints

`[STABLE]`

These six constraints MUST hold for every conformant implementation.
They are verified by the compliance test suite (§11). An implementation
that fails any constraint is non-conformant regardless of which other
properties it satisfies.

### 7.1 Idempotency

A `invoke` call submitted with a previously-used `(run_id, span_id)`
pair MUST return the same `ActionResult` as the original call without
re-executing the action. Implementations MUST store results indexed
by `(run_id, span_id)` for at least the duration of the run.

*Rationale*: Runtimes must be able to safely retry `invoke` calls after
network failures without causing duplicate side effects.

### 7.2 Capability non-amplification

For any Agent-to-Agent call where Agent A calls Agent B:

```
delegated_caps(A→B) ⊆ caps(A) ∩ caps(B)
```

The governance implementation MUST compute `delegated_caps` itself.
The calling Agent (A) MUST NOT be able to specify a `delegated_caps`
that exceeds this bound. The `caps_hint` field in `CallAgent` is
advisory and MUST be silently capped to the intersection.

*Rationale*: prevents privilege escalation through delegation chains.

### 7.3 Forced observability

For every `invoke` call — successful or not — the implementation MUST:

1. Call `on_invoke_begin` before capability check.
2. Call `on_invoke_end` after the audit entry is written.
3. Call `on_error` for every error, including `PolicyViolation`.

These hooks MUST NOT be suppressible by the caller. There is no
configuration, flag, or mode that disables observability.

*Rationale*: audit completeness requires that every action, including
rejected actions, is observable.

### 7.4 Structured errors

All errors returned across the Protocol boundary MUST be one of the
variants defined in §6. Implementations MUST NOT return untyped error
strings, HTTP status codes, or implementation-specific error objects.

*Rationale*: callers must be able to write reliable error-handling code
without parsing strings or special-casing implementation details.

### 7.5 Cancellation propagation

A `cancel(run_id)` call MUST cancel all in-flight `invoke` and
`invoke_stream` calls associated with `run_id`, including calls
that originated in sub-Agent invocations. Each cancelled call MUST
receive `Cancelled` as its error. The cancellation MUST be propagated
depth-first through the full call tree.

*Rationale*: partial cancellation leaves sub-Agents in undefined states
and creates unaudited orphan activity.

### 7.6 WAL audit

The audit `LogEntry` for an action MUST be durably written before
the `ActionResult` is returned to the caller. "Durable" means
the entry survives a process restart. The entry MUST be written
with the `status` field set to `Pending` if the action has not yet
completed (streaming case), and a completion entry MUST be written
when the stream closes.

*Rationale*: if execution succeeds but the audit entry is lost, the
action is undetectable. The audit must be the record of authority.

---

## 8. Agent-to-Agent calling convention

`[STABLE]`

When an Agent submits a `CallAgent` action, the governance implementation
MUST follow this sequence exactly.

**Step 1 — validate caller**  
Verify that the caller's `KernelHandle` is valid and non-revoked. If not,
return `InvalidHandle`.

**Step 2 — check caller capability**  
Verify that the caller's `CapabilitySet` includes an `AgentCall`
capability that covers `target_id`. If not, return `PolicyViolation`.

**Step 3 — resolve target**  
Look up the target `AgentId` in the registry. If not found or not
currently spawned, return `PolicyViolation { missing_cap: AgentCall }`.

**Step 4 — compute delegated capabilities**  
```
delegated = caller_caps ∩ target_caps
```
This computation is performed by the governance implementation. The
caller cannot influence the result beyond the constraint of its own
`caller_caps`.

**Step 5 — write call-begin audit entry**  
Write a `LogEntry` with:
- `agent_id` = caller's `AgentId`
- `parent_span_id` = caller's current `SpanId`
- `action` = `CallAgent { target_id, delegated_caps: delegated }`
- `status` = `Pending`

**Step 6 — invoke target**  
Execute the target Agent's action with `delegated` as its effective
`CapabilitySet`. The target's `run_id` MUST be inherited from the
caller's `run_id`. The target receives a fresh `SpanId`.

**Step 7 — write call-end audit entry**  
Write a completion entry linking to the step 5 entry, with the final
`status` and `result`.

**Step 8 — return result to caller**  
Return the target's `ActionResult` to the calling Agent.

### Call depth limit

Implementations SHOULD enforce a maximum call depth (default: 16).
At the limit, `invoke` MUST return `ResourceExhausted { resource: ComputeQuota }`.

### Capability inheritance across call depth

The non-amplification constraint is applied at each hop independently.
If A calls B calls C:

```
caps(A→B) = caps(A) ∩ caps(B)
caps(B→C) = caps(A→B) ∩ caps(C)   // NOT caps(B) ∩ caps(C)
```

The effective capability set at each hop is bounded by the intersection
accumulated along the path, not by the direct intersection of adjacent
nodes. This prevents capability recovery through indirection.

---

## 9. Capability model

`[STABLE]`

### 9.1 Capability binding

Capabilities are bound to a `KernelHandle` at `spawn` time. The binding
is atomic: there is no observable state where a handle exists but
capabilities are not yet set.

Capabilities are immutable for the lifetime of a handle. To change an
Agent's capabilities, the current handle must be terminated and a new
handle spawned with the updated `CapabilitySet`.

### 9.2 Capability evaluation

For every `invoke` call, the governance implementation MUST evaluate
whether the submitted `Action` is permitted by the handle's
`CapabilitySet`. The evaluation MUST happen before execution and MUST
produce a binary decision: permit or `PolicyViolation`.

Capability evaluation is not advisory. There is no "warn but proceed"
mode.

### 9.3 Delegation intersection rule

When capability A delegates to capability B via an Agent-to-Agent call:

```
delegated = A ∩ B
```

Where `∩` is defined as:
- Both sets must contain a matching `Capability` variant.
- For parameterised capabilities (e.g. `ToolRead { tool_id }`),
  the parameters must match exactly. There is no wildcard matching
  at delegation time.
- `AgentCall { target_agent_id: None }` (any-agent) intersected with
  `AgentCall { target_agent_id: Some(X) }` (specific) yields
  `AgentCall { target_agent_id: Some(X) }` (specific wins).
- `LlmCall { model_family: None }` (any-model) intersected with
  `LlmCall { model_family: Some("claude") }` yields
  `LlmCall { model_family: Some("claude") }` (specific wins).

### 9.4 Capability revocation

Revoking a `KernelHandle` via `terminate` immediately invalidates all
capabilities bound to that handle. Subsequent `invoke` calls MUST return
`InvalidHandle`. Revocation is instantaneous from the perspective of
the Protocol — there is no grace period during which a revoked handle
is accepted.

In-flight `invoke` calls at the moment of revocation MUST be allowed to
complete. The revocation takes effect for the next `invoke` call.

---

## 10. Audit log schema

`[STABLE]`

Every `invoke` call, including rejected calls and sub-Agent calls, MUST
produce at least one `LogEntry` in the audit log before the call's
result is returned to the caller.

```
LogEntry {
  seq:            u64,         // monotonically increasing, global, gapless
  run_id:         RunId,
  span_id:        SpanId,
  parent_span_id: SpanId?,     // null for top-level spans
  agent_id:       AgentId,
  action:         Action,
  caps_scope:     CapabilitySet,  // capabilities active at time of this entry
  result: {
    status:       Pending | Ok | Err,
    error_type:   ProtocolError variant name?,
    payload_hash: Bytes?,       // SHA-256 of payload, not the payload itself
  },
  ts:             Timestamp,   // UTC nanoseconds
  integrity:      Bytes,       // SHA-256( encode(this entry with integrity=null) || prev_integrity )
}
```

### Integrity chain

The `integrity` field creates a hash chain. To compute entry N's integrity:

```
integrity[N] = SHA-256(
  canonical_encode(entry[N] with integrity = null)
  ||
  integrity[N-1]
)
```

Where `canonical_encode` is deterministic (e.g. canonical JSON or
protobuf with sorted fields). The first entry uses a genesis hash of
`SHA-256("agent-protocol-v1")`.

Any party with access to the log can verify the chain by recomputing
hashes from genesis. A broken chain MUST be surfaced as
`AuditIntegrityError` on `query_audit` calls.

### Append-only guarantee

Implementations MUST NOT expose any API that modifies or deletes
existing `LogEntry` records. The audit log is a write-once data structure.

### Retention

The minimum required retention period is 90 days. Implementations MAY
offer configurable retention. Entries older than the retention period MAY
be archived or deleted, but MUST be flagged as `archived` in the index
so that `query_audit` callers know the entry existed but is no longer
immediately accessible.

### Streaming audit entries

For `invoke_stream` calls, two entries are written:

1. An initial entry with `status: Pending`, written when the stream is opened.
2. A completion entry with `status: Ok` or `Err`, written when the stream closes.

Both entries share the same `span_id`. The completion entry's `seq`
is greater than the initial entry's `seq`.

---

## 11. Compliance

`[STABLE]`

### Compliance test suite

The normative compliance test suite is published at:

```
crates/agent-protocol/tests/compliance/
```

An implementation is conformant if and only if it passes all tests in
the suite at the version of the suite corresponding to the Protocol
version being claimed.

### What the suite tests

| Test group | Constraints verified |
|---|---|
| `lifecycle` | Handle validity, spawn atomicity, termination finality |
| `invocation` | Enforcement sequence, audit WAL ordering, stream audit entries |
| `capability` | Non-amplification at each delegation hop, revocation immediacy |
| `errors` | Error variant correctness, ordering rule, no untyped errors |
| `idempotency` | Replayed `(run_id, span_id)` returns same result, no re-execution |
| `cancellation` | Full tree propagation, orphan detection |
| `observability` | Hook call ordering, hook non-suppressibility |
| `audit_integrity` | Hash chain validity, append-only enforcement |

### Claiming conformance

To claim conformance with Agent Protocol version N:

1. Run the compliance suite at version N against your implementation.
2. All tests must pass with zero failures and zero skips.
3. Publish your test results alongside your implementation.

There is no certification body. Conformance is self-declared and
independently verifiable by anyone who runs the test suite.

---

## 12. Versioning and evolution

`[STABLE]`

The Protocol uses semantic versioning: `MAJOR.MINOR.PATCH`.

**MAJOR** increments when a constraint is strengthened, an interface is
removed, or a data type changes in a backward-incompatible way. Existing
conformant implementations may become non-conformant after a MAJOR
increment and must be updated.

**MINOR** increments when new optional interface families are added, new
`Action` variants are added, or new `Capability` types are introduced.
Existing conformant implementations remain conformant.

**PATCH** increments for clarifications, editorial fixes, and test suite
corrections that do not change the normative requirements.

### Proposing changes

All changes to normative requirements (§5–§10) require a formal proposal:

1. File a proposal as a Markdown document at
   `protocol-spec/proposals/YYYY-MM-DD-title.md`.
2. The proposal must state: motivation, the precise change to normative
   text, backward-compatibility impact, and compliance test additions.
3. A 30-day review period applies to MAJOR changes; 14 days for MINOR.
4. A proposal is merged only after two maintainer approvals and zero
   unresolved objections from the community review period.

Implementation code MUST NOT be merged before the corresponding proposal
is merged. The specification leads the implementation.

---

## 13. Glossary

| Term | Definition |
|---|---|
| Action | A typed request submitted via `invoke`. |
| AgentDef | The static definition of an Agent: identity, description, defaults. |
| AgentId | UUID identifying an Agent definition. |
| Audit entry | An immutable `LogEntry` written before an action's result is returned. |
| Capability | An explicit, unforgeable token granting a specific class of action. |
| CapabilitySet | The complete set of capabilities bound to a handle at spawn. |
| Checkpoint | A serialised snapshot of an Agent's suspended state. |
| Conformant | An implementation that passes the full compliance test suite. |
| Delegated caps | The capability set computed by the intersection rule for a sub-Agent call. |
| Governance implementation | A system that implements the Protocol interfaces with enforcement. |
| HandleId | The opaque, unforgeable token inside a `KernelHandle`. |
| Integrity chain | The hash chain linking successive audit entries. |
| InterruptToken | An opaque token returned by `interrupt`, required for `confirm`. |
| KernelHandle | The opaque reference to a spawned Agent instance. |
| Protocol | The Agent Protocol: this specification. |
| RunId | UUID identifying a top-level invocation, propagated through the call tree. |
| Runtime | The execution environment for a single Agent instance. |
| SpanId | UUID identifying one step within a run. |
| WAL | Write-Ahead Log: the pattern of writing a record before executing an action. |
