# Architecture

This document explains what Agent Kernel is, why it is built the way it is,
and how external systems should think about integrating with it.

If you are a contributor working inside this repository, read `AGENTS.md`.
If you are building a Runtime, Framework, or tool that runs on top of the
Kernel — or implementing the Agent Protocol independently — start here.

---

## What this system is

Agent Kernel is an operating system for Agent systems.

That framing is deliberate. The field has produced excellent tools for making
Agents *capable* — Frameworks for orchestration, Runtimes for execution,
Models for reasoning. What it has not produced is infrastructure for making
Agents *governable*: a mandatory, external enforcement layer that no Agent
can bypass, that every action must pass through, and that leaves an
unforgeable record of everything that happened.

That is what this system is.

The analogy to an OS kernel is precise, not metaphorical:

| OS kernel | Agent Kernel |
|---|---|
| Processes cannot access hardware directly | Agents cannot access tools, LLMs, or other Agents directly |
| All system calls pass through the kernel | All Agent actions pass through `invoke` |
| The kernel enforces access control | The Kernel enforces capability policy |
| The kernel schedules CPU time | The Kernel schedules LLM concurrency and tool quotas |
| The kernel audits privileged operations | The Kernel writes a tamper-evident log of every action |
| Processes run in isolated address spaces | Agents run in isolated sandbox contexts |

The practical consequence: a developer who builds on top of the Kernel does
not need to implement permission checking, audit logging, or isolation
themselves. The Kernel provides these as non-optional guarantees, not
optional libraries.

---

## The problem this system solves

As Agent deployments move from development environments into production,
three structural problems emerge that no existing layer addresses:

**Permissions are self-enforced.**
Authorization logic today lives inside Agent code — written by the same
developer whose Agent benefits from broader access. A permission check
inside the thing being controlled is not enforcement; it is a suggestion.

**There is no global visibility.**
When Agent A calls Agent B which calls Agent C, no single system knows
the full call graph, the capabilities in play at each hop, or what
actually executed. Each Runtime instance sees only itself.

**Audit is fragmented or absent.**
Frameworks log what they choose, in whatever format they prefer.
There is no tamper-proof, cross-Agent audit record that could satisfy
a compliance requirement or support a post-incident investigation.

The Kernel is the answer to all three. It is the single point that all
Agent activity must pass through — and therefore the only place where
these guarantees can be made unconditionally.

---

## Design principles

These five principles govern every decision in the system. When a design
choice is unclear, evaluate it against this list.

### 1. Enforcement over convention

Security and correctness must be enforced by the Kernel, not delegated
to developers. If a guarantee depends on the Framework developer doing
the right thing, it is not a guarantee. The Kernel's value proposition
is that its guarantees hold regardless of what the Agent code does.

### 2. Single interception point

All Agent actions pass through one method: `Invocation::invoke`. This
is not an accident of implementation — it is the architectural invariant
that makes the Kernel's guarantees possible. Every capability check,
every audit write, every sandbox boundary derives from the fact that
there is exactly one place where the outside world is reached.

### 3. Capability-based authority

Authority in this system is explicit, composable, and non-ambient.
An Agent can only do what its `CapabilitySet` permits. Capabilities
are bound at spawn time and cannot be expanded at runtime. When an
Agent delegates to another Agent, the delegated capability set is
automatically computed as the intersection of the two Agents' sets —
it is structurally impossible to delegate authority you do not have.

### 4. Full traceability

Every action must be observable, attributable, and auditable. The audit
log uses WAL semantics — an entry is written before the action executes,
not after. Entries form an integrity chain: each entry contains the hash
of its predecessor. The chain can be independently verified by anyone
with access to the log. Traceability is not a feature that can be turned
off; it is a property of the system.

### 5. Protocol-first architecture

The Agent Protocol defines what correct behaviour looks like. The Kernel
enforces it. Frameworks must conform to it. This ordering matters: the
Protocol is the specification, and the Kernel is one implementation of
that specification. Third parties can implement the Protocol
independently. The compliance test suite, not the Kernel's source code,
is the authoritative definition of correctness.

---

## Threat model

Every design decision in this system is evaluated against the following
threat model. If a proposed change weakens the defence against any of
these threats, it requires an explicit justification and maintainer
sign-off.

**Capability amplification**
An Agent, or a chain of Agent-to-Agent calls, acquires capabilities
beyond what the original grant permitted. Defended by the intersection
rule in the permission engine.

**Prompt injection leading to unauthorized actions**
An adversarial payload in a tool response or user input causes an Agent
to attempt actions it was not granted. Defended by capability enforcement
at the `invoke` boundary — the Agent's intent is irrelevant; only its
capability set matters.

**Privilege escalation via Agent delegation**
A low-privilege Agent calls a high-privilege Agent and inherits its
capabilities. Defended by `delegated_caps = caller_caps ∩ target_caps`.

**Audit log tampering**
A compromised process modifies historical log entries to conceal
activity. Defended by the append-only WAL and the SHA-256 integrity
chain. Tampering with any entry invalidates all subsequent hashes.

**Hidden execution paths**
Code creates a side channel that reaches the Execution Substrate without
passing through `invoke`, bypassing capability checks and audit.
Defended by sandbox isolation and the strict `#![forbid(unsafe_code)]`
policy outside the sandbox crate.

**Undetected multi-Agent call graph expansion**
A multi-Agent workflow expands in scope without the user being aware of
what downstream Agents were invoked. Defended by the `parent_span_id`
field in every audit entry, which enables full call graph reconstruction
from the flat log.

---

## System model

```
┌─────────────────────────────────────────────────────┐
│                 Agent Applications                  │  L1
└──────────────────────────┬──────────────────────────┘
                           │
┌──────────────────────────▼──────────────────────────┐
│           Framework / Orchestration                 │  L2
│     (LangGraph, AutoGen, CrewAI, or custom)         │
└──────────────────────────┬──────────────────────────┘
                           │  Agent Protocol
┌──────────────────────────▼──────────────────────────┐
│                  Agent Runtime                      │  L3
│          (reasoning loop, context, state)           │
└──────────────────────────┬──────────────────────────┘
                           │  kernel-api
┌──────────────────────────▼──────────────────────────┐
│                                                     │
│                  Agent Kernel                       │  L4
│                                                     │
│  ┌─────────────┐  ┌───────────┐  ┌──────────────┐  │
│  │  Permission │  │ Scheduler │  │   Sandbox    │  │
│  │   Engine    │  │           │  │  Isolation   │  │
│  └─────────────┘  └───────────┘  └──────────────┘  │
│  ┌─────────────────────────────────────────────┐    │
│  │              Audit Log (WAL)                │    │
│  └─────────────────────────────────────────────┘    │
│  ┌─────────────────────────────────────────────┐    │
│  │          Protocol Runtime (embedded)        │    │
│  └─────────────────────────────────────────────┘    │
│                                                     │
└──────────────────────────┬──────────────────────────┘
                           │
┌──────────────────────────▼──────────────────────────┐
│               Execution Substrate                   │  L5
│      LLM Gateway · Tool Runtime · Memory Store      │
└─────────────────────────────────────────────────────┘
```

The invariant that must hold everywhere in the system:

```
Agent → Protocol → Kernel → Substrate
```

No direct Agent-to-Substrate path exists. No Agent-to-Agent call
bypasses the Kernel. The Protocol is not an optional adapter — it is
the only channel through which the Kernel can be reached.

---

## The Agent Protocol

The Agent Protocol is an open, implementation-independent specification.
It is published separately under the MIT licence and governed
independently of the Kernel implementation.

### What the Protocol defines

The Protocol specifies five interface families:

| Interface | Analogue | Purpose |
|---|---|---|
| `AgentLifecycle` | `fork` / `exec` / `kill` | Spawn, suspend, resume, terminate Agent instances |
| `Invocation` | `read` / `write` | Execute actions; the single interception point |
| `ContextIO` | `mmap` / `lseek` | Read and write Agent memory across scopes |
| `SignalEvent` | `signal` / `wait` | Emit events; interrupt for human confirmation |
| `ObservabilityHook` | `ptrace` | Mandatory trace emission on every invoke |

And six mandatory semantic constraints:

1. **Idempotency** — same `RunId` replayed produces the same result.
2. **Capability non-amplification** — `delegated ⊆ caller ∩ target`.
3. **Forced observability** — every `invoke` must emit a trace span.
4. **Structured errors** — all errors use the canonical `ProtocolError` taxonomy.
5. **Cancellation propagation** — `cancel` cascades to all child invocations.
6. **WAL audit** — log entry written before action result is returned.

### Relationship between Protocol and Kernel

The Protocol defines what correct behaviour looks like.
The Kernel is one implementation of the Protocol — specifically, the
reference implementation, the one that embeds the Protocol Runtime and
provides the enforcement layer.

A third party may implement the Protocol independently. Compatibility is
defined by the compliance test suite in
`crates/agent-protocol/tests/compliance/`, not by the Kernel's source
code. Any system that passes the compliance suite is a conformant
Protocol implementation.

This separation is intentional. The Protocol is a standard. Standards
outlive implementations. The compliance test suite is the authoritative
definition of correctness, and it belongs to the community.

---

## Integration model

There are three ways external systems interact with the Kernel.

### 1. Runtime integration

The most common integration path. A Runtime calls the Kernel via
`kernel-api` to spawn Agents and execute actions.

```
Runtime
  │
  ├─ spawn(AgentDef, CapabilitySet) → KernelHandle
  ├─ invoke(KernelHandle, Action)   → ActionResult
  ├─ revoke(KernelHandle)           → RunSummary
  └─ query_audit(AuditFilter)       → Vec<LogEntry>
```

The Runtime does not need to implement permission checking, audit
logging, or isolation. These are provided by the Kernel unconditionally.
The Runtime's responsibility is to call `invoke` for every action and
to handle the returned `ProtocolError` variants correctly.

SDKs for Rust, Python, and TypeScript are provided in `sdk/`.

### 2. Protocol implementation

A Framework or Runtime that wants to implement the Protocol directly,
rather than using `kernel-api`, must:

- Implement the five interface families in `agent-protocol`.
- Pass the full compliance test suite.
- Handle all six semantic constraints correctly.

Implementing the Protocol without the Kernel is possible but means
accepting responsibility for enforcement, audit, and isolation. This is
the path for teams that need to run the Protocol on hardware or in
environments where the Kernel cannot be deployed.

### 3. Observability integration

Any system may subscribe to the Kernel's audit stream and trace hooks
for monitoring, alerting, or compliance reporting. The audit log is
append-only and exports to structured JSON and OpenTelemetry.

Audit entries follow this schema:

```
{
  seq:         monotonic sequence number (global, cross-agent)
  run_id:      top-level task identifier (propagated through call tree)
  agent_id:    the Agent that initiated the action
  parent_span_id: the parent span (enables call graph reconstruction)
  action:      { type, target, params_hash }
  caps_scope:  the capability set active at execution time
  result:      { status, error_type? }
  ts:          UTC nanosecond timestamp
  integrity:   SHA-256 of the preceding entry
}
```

The `integrity` field enables independent verification. Any party with
access to the log can verify the chain without trusting the Kernel.

---

## Agent-to-Agent calls

Multi-Agent systems introduce a specific class of problem that does not
exist in single-Agent deployments: capability propagation across trust
boundaries.

The Kernel governs Agent-to-Agent calls using the same `invoke` path as
all other actions. When Agent A calls Agent B:

1. A's `KernelHandle` is validated.
2. The delegated capability set is computed: `delegated = A_caps ∩ B_caps`.
3. A `LogEntry` is written with `parent_span_id` linking to A's current span.
4. B is invoked with `delegated` — not A's full set, not B's full set.
5. B's result is returned to A via the Kernel.

The result is that the Kernel maintains a complete, tamper-evident record
of the entire multi-Agent call graph. From any single top-level `RunId`,
the full tree of what was called, by whom, with what capabilities, and
with what result, can be reconstructed from the flat audit log.

---

## What the Kernel is not

It is worth being explicit about what this system does not do, to avoid
building incorrect expectations.

**The Kernel is not a Framework.**
It does not orchestrate Agent workflows, manage DAGs, or define how
Agents communicate with each other at the application level. Those
concerns belong to L2 (Framework/Orchestration). The Kernel operates
below the Framework.

**The Kernel is not a Runtime.**
It does not manage the reasoning loop, maintain context windows, or
handle model inference. Those concerns belong to L3 (Agent Runtime).
The Kernel receives actions from the Runtime; it does not produce them.

**The Kernel is not a monitoring tool.**
Audit and observability are outputs of the Kernel's enforcement activity,
not the primary purpose. The Kernel writes audit entries because
enforcement requires a record — not because it is a logging service.

**The Kernel does not evaluate Agent intent.**
The Kernel does not analyse what an Agent is trying to do or whether its
goal is aligned. It only evaluates whether the specific action being
requested is permitted by the active capability set. Intent is a
higher-level concern.

---

## Compliance

The Kernel's data model is designed to map directly onto the control
frameworks that regulated industries use for access control audits.

| Framework | Control | Kernel evidence |
|---|---|---|
| SOC 2 CC6.1 | Least privilege | Per-handle `CapabilitySet`, immutable after spawn |
| SOC 2 CC6.3 | Access revocation | `revoke()` timestamp + subsequent `PolicyViolation` trail |
| SOC 2 CC7.2 | Anomaly monitoring | `PolicyViolation` frequency, call volume deviation |
| GDPR Art. 25 | Data minimisation | `ContextIO` scope limits on data access |
| GDPR Art. 30 | Processing records | Full `invoke` history with data type, subject, purpose |
| ISO 27001 A.9.4 | System access control | Capability matrix export across all Agent instances |

The key design decision behind this coverage: the audit log schema
captures the five elements that compliance frameworks universally require
— subject, object, operation, outcome, timestamp — as first-class fields,
not as free-text log messages. This makes compliance reporting a query,
not a parsing problem.

---

## Further reading

- `AGENTS.md` — engineering rules for contributors to this repository
- `protocol-spec/overview.md` — the Agent Protocol specification
- `protocol-spec/constraints.md` — the six mandatory semantic constraints
- `crates/agent-protocol/tests/compliance/` — the compliance test suite
- `docs/compliance/` — SOC 2, GDPR, and ISO 27001 mapping tables
- `sdk/` — integration SDKs for Rust, Python, and TypeScript
