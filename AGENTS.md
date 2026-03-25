# AGENTS.md

This file is the authoritative guide for any AI coding agent working in this
repository. Read it completely before writing or modifying any code.

> **If you are building on top of this system** (Runtime integration,
> Protocol implementation, observability tooling), read `ARCHITECTURE.md`
> first. This file is for contributors working *inside* the repository.

---

## Project overview

**agent-kernel** is the trust infrastructure layer for Agent systems.
It sits between Agent Runtimes and the Execution Substrate, acting as the
mandatory, un-bypassable enforcement point for permissions, scheduling,
isolation, and audit across all Agent activity — including Agent-to-Agent
calls.

## Current repository state

This repository is currently a design and architecture workspace. It does not
yet contain the full Rust implementation workspace described later in this
document.

Files and directories that exist today:

- Top-level project documents such as `AGENTS.md`, `ARCHITECTURE.md`, and
  `README.md`
- `protocol-spec/overview.md`
- `docs/architecture/` notes, plans, and Mermaid diagram sources

Files and directories that do not exist yet in this checkout:

- `Cargo.toml`
- `crates/`
- `sdk/`
- `docs/compliance/`

When working in the repository as it exists today, treat implementation paths,
crate ownership guidance, and Rust commands in this file as target-state
guidance unless those files have actually been added.

---

## Target implementation workspace

The long-term repository structure is expected to look like this:

The repository contains two tightly related sub-projects:

| Sub-project | Crate path | Purpose |
|---|---|---|
| **Agent Protocol** | `crates/agent-protocol` | Open specification + runtime: the formal contract for how Agents communicate with the Kernel and with each other. MIT-licensed. |
| **Agent Kernel** | `crates/kernel-*` | The enforcement implementation: permission engine, scheduler, sandbox, audit log, and the Protocol runtime that embeds the spec. Apache 2.0. |

The Protocol defines the rules. The Kernel enforces them. Neither is useful
without the other; neither is the same thing.

---

Target repository layout:

```
agent-kernel/
├── AGENTS.md                   ← you are here
├── Cargo.toml                  ← workspace root
├── crates/
│   ├── agent-protocol/         ← Protocol spec types, traits, test suite
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── types.rs        ← AgentId, RunId, Capability, ActionResult …
│   │   │   ├── interfaces/     ← five interface families (see below)
│   │   │   │   ├── lifecycle.rs
│   │   │   │   ├── invocation.rs
│   │   │   │   ├── context_io.rs
│   │   │   │   ├── signal_event.rs
│   │   │   │   └── observability.rs
│   │   │   └── errors.rs       ← ProtocolError, structured error taxonomy
│   │   └── tests/
│   │       └── compliance/     ← Protocol compliance test suite
│   │
│   ├── kernel-api/             ← public surface exposed to Runtime callers
│   │   └── src/lib.rs          ← KernelHandle, AgentSyscall trait, 4 methods
│   │
│   ├── kernel-core/            ← internal orchestration; calls all subsystems
│   │   └── src/
│   │       ├── lib.rs
│   │       └── dispatch.rs     ← the single critical path: check→schedule→sandbox→audit
│   │
│   ├── permission-engine/      ← capability evaluation and policy enforcement
│   ├── scheduler/              ← resource budgets, priority queues
│   ├── sandbox/                ← process isolation, seccomp, namespace
│   └── audit-log/              ← append-only WAL, integrity chain
│
├── sdk/
│   ├── python/                 ← Python SDK (PyO3 bindings)
│   └── typescript/             ← TypeScript SDK (napi-rs bindings)
│
├── protocol-spec/              ← human-readable Protocol specification (Markdown)
│   ├── overview.md
│   ├── interfaces.md
│   ├── errors.md
│   └── constraints.md          ← the six mandatory semantic constraints
│
└── docs/
    ├── architecture.md
    └── compliance/             ← SOC 2, GDPR, ISO 27001 mapping tables
```

---

## Core concepts you must understand before editing code

### The dispatch path

Every action an Agent attempts follows exactly this sequence inside
`kernel-core/src/dispatch.rs`. Nothing bypasses it.

```
invoke(handle, action)
  │
  ├─ 1. identity check     — is this KernelHandle valid and non-revoked?
  ├─ 2. permission check   — does the handle's CapabilitySet permit this action?
  │      on failure → write PolicyViolation to audit log, return Err
  ├─ 3. scheduler          — is capacity available? enqueue if not
  ├─ 4. sandbox            — execute inside isolated process context
  ├─ 5. audit write        — append LogEntry BEFORE result is returned (WAL)
  └─ 6. return result
```

**Do not add any shortcut that skips steps 2 or 5.** PRs that do so will be
rejected regardless of justification. These two steps are the reason the
Kernel exists.

### Protocol interfaces

The Protocol defines five interface families. Each maps to a POSIX analogue
to make the design intent clear:

| Interface | POSIX analogue | Core operations |
|---|---|---|
| `AgentLifecycle` | `fork` / `exec` / `kill` | `spawn`, `suspend`, `resume`, `terminate` |
| `Invocation` | `read` / `write` | `invoke`, `invoke_stream`, `cancel` |
| `ContextIO` | `mmap` / `lseek` | `read`, `write`, `search`, `snapshot`, `restore` |
| `SignalEvent` | `signal` / `wait` | `emit`, `subscribe`, `interrupt`, `confirm` |
| `ObservabilityHook` | `ptrace` | `on_invoke_begin`, `on_invoke_end`, `on_error` |

`Invocation::invoke` is the single most important method in the codebase.
All tool calls, LLM calls, memory access, and Agent-to-Agent calls arrive
here. The Kernel only needs one interception point because everything is
funnelled through `invoke`.

### Capability non-amplification

When Agent A calls Agent B, the capabilities delegated to B are computed as:

```
delegated_caps = caller_caps ∩ target_caps
```

This is enforced in `permission-engine` automatically. No caller API is
required. No developer decision is needed. It must never be possible for a
sub-Agent to hold capabilities its parent did not have.

### Audit log integrity

Each `LogEntry` contains an `integrity` field: the SHA-256 hash of the
immediately preceding entry. This forms a hash chain. Any tampering with a
historical entry invalidates all subsequent hashes.

The integrity chain must be verified on every `query_audit` call.
If verification fails, return `AuditIntegrityError` — never silently serve
a broken chain.

---

## The four public Kernel API methods

`kernel-api` exposes exactly four methods. This surface is intentionally
minimal and stable. Do not add methods without an approved design doc.

```rust
/// Spawn a new Agent instance bound to the given capabilities.
/// Capabilities are immutable for the lifetime of the returned handle.
spawn(def: AgentDef, caps: CapabilitySet) -> Result<KernelHandle>

/// Execute an action on behalf of the Agent identified by handle.
/// This is the single critical path: check → schedule → sandbox → audit.
invoke(handle: &KernelHandle, action: Action) -> Result<ActionResult>

/// Revoke an Agent. Flushes in-flight actions, marks handle invalid,
/// writes a Revocation entry to the audit log.
revoke(handle: KernelHandle) -> Result<RunSummary>

/// Query the audit log. Read-only. Returns immutable entries.
query_audit(filter: AuditFilter) -> Result<Vec<LogEntry>>
```

---

## Mandatory semantic constraints

These are not guidelines. Every constraint must hold for every commit on
`main`. In the target implementation workspace, the compliance test suite in
`crates/agent-protocol/tests/compliance` verifies all six.

1. **Idempotency** — replaying the same `RunId` must produce the same
   observable result.
2. **Capability non-amplification** — `delegated_caps ⊆ caller_caps ∩ target_caps`.
   No exceptions.
3. **Forced observability** — every `invoke` call must emit a trace span via
   `ObservabilityHook`. This hook cannot be disabled or mocked in production
   builds.
4. **Structured errors** — all errors must be one of the canonical
   `ProtocolError` variants. Untyped strings are not permitted in public
   interfaces.
5. **Cancellation propagation** — `cancel(run_id)` must cascade to all
   child invocations in the call tree. Partial cancellation is a bug.
6. **WAL audit** — the `LogEntry` for an action must be durably written
   before the action's result is returned to the caller.

---

## What to do before writing code

If you are working only in the current docs-only repository state, adapt these
steps to the materials that actually exist. Review the spec and architecture
documents first instead of assuming the implementation workspace is present.

1. **Identify which crate owns the change.** Changes to the Protocol
   interface contract belong in `agent-protocol`. Changes to enforcement
   logic belong in the relevant `kernel-*` crate. Changes to the public API
   belong in `kernel-api`. Do not put Protocol logic in Kernel crates or
   vice versa — the separation is intentional.

2. **Check if a constraint is affected.** If your change touches `dispatch.rs`,
   `permission-engine`, or `audit-log`, re-read the six constraints above
   and confirm your change does not weaken any of them.

3. **Run the compliance suite before opening a PR.**
   ```bash
   cargo test -p agent-protocol --test compliance
   ```
   In the target implementation workspace, all tests must pass. The
   compliance suite is the authoritative check for Protocol correctness — it
   is not optional.

4. **Run the full test suite.**
   ```bash
   cargo test --workspace
   ```

5. **Check performance budgets** if touching the dispatch path.
   ```bash
   cargo bench -p kernel-core
   ```
   In the target implementation workspace, the `invoke` p99 latency budget is
   **5 ms**. The audit write p99 budget is **2 ms**. Regressions block merge.

---

## Code style and conventions

These conventions apply to the target Rust workspace once implementation files
exist in the repository.

### Rust

- **Stable Rust only.** No nightly features. The Kernel must build on the
  current stable toolchain.
- **No `unwrap()` or `expect()` in library crates.** Every fallible
  operation must propagate errors via `Result`. Panics in the dispatch path
  are bugs, not acceptable error handling.
- **No `unsafe` outside `sandbox/`.** The sandbox crate interacts with
  `seccomp` and Linux namespaces and has a justified `unsafe` budget.
  All other crates must be `#![forbid(unsafe_code)]`.
- **`thiserror` for error types, `tracing` for structured logging.**
  Do not use `println!` or `eprintln!` anywhere in library crates.
- **Every public function needs a doc comment.** Include the invariants the
  caller must satisfy and the invariants the function guarantees on return.
- Formatting: `cargo fmt --all`. Linting: `cargo clippy --all-targets -- -D warnings`.

### Error taxonomy

All errors returned across the `kernel-api` boundary must be one of:

| Variant | Meaning |
|---|---|
| `PolicyViolation` | Capability check failed; action not permitted |
| `ResourceExhausted` | Scheduler budget exhausted; retry after backoff |
| `Interrupted` | HITL confirmation pending; call `confirm()` to resume |
| `Timeout` | Action exceeded the configured time limit |
| `ContextOverflow` | Context window budget exceeded |
| `Cancelled` | Cancelled by an upstream `cancel()` call |
| `AuditIntegrityError` | Audit chain verification failed |
| `InvalidHandle` | KernelHandle is expired or revoked |

Do not introduce new top-level variants without a Protocol change proposal.

### Naming conventions

| Concept | Convention | Example |
|---|---|---|
| Agent identity | `AgentId` (newtype over `Uuid`) | `AgentId::new()` |
| Run identity | `RunId` (newtype over `Uuid`) | propagated from spawn |
| Capability | `Capability` enum variant | `Capability::ToolRead { tool_id }` |
| Audit entries | `LogEntry` | never `AuditRecord`, never `Event` |
| Public API handle | `KernelHandle` | opaque to callers |

---

## Testing

### Test categories

| Category | Location | When to run |
|---|---|---|
| Unit tests | `src/` inline `#[cfg(test)]` | In target workspace — always run `cargo test --workspace` |
| Integration tests | `crates/*/tests/` | In target workspace — always |
| Protocol compliance | `crates/agent-protocol/tests/compliance/` | In target workspace — must pass on every PR |
| Benchmarks | `crates/kernel-core/benches/` | In target workspace — on dispatch-path changes |

### What must be tested for any dispatch-path change

- The happy path: valid capability, action executes, audit entry written.
- `PolicyViolation` path: capability missing, action blocked, audit entry
  still written.
- Audit WAL ordering: the log entry sequence number for an action must be
  less than the sequence number of the action's result being observable.
- Cancellation propagation: cancelling a parent run cancels all child spans.

### Mocking policy

- **Do not mock the audit log in dispatch-path tests.** Use an in-memory
  WAL implementation — the real write ordering must be tested.
- **Do not mock the permission engine.** Test with real `CapabilitySet`
  instances. Policy logic is too critical to test through a mock.
- The sandbox can be replaced with a no-op in unit tests. Use the
  `sandbox::NoopSandbox` fixture.

---

## Agent-to-Agent calls

This is the most security-sensitive code path in the repository.

When `invoke` receives an action of type `Action::CallAgent { target_id, … }`:

1. Look up `target_id` in the Kernel's agent registry.
2. Compute `delegated_caps = caller_caps ∩ target_caps`.
3. Write a `LogEntry` with `parent_span_id` set to the caller's current span.
4. Spawn a child invocation with `delegated_caps` — not `caller_caps`,
   not `target_caps`, only the intersection.
5. The child's `RunId` inherits from the parent; the `SpanId` is fresh.

**Never pass `caller_caps` directly to the child.** This is the
capability amplification vulnerability. The intersection must always be
computed explicitly in step 2.

---

## What not to do

- **Do not add a `bypass_permission_check` flag, parameter, or feature.**
  There is no legitimate use case. If you think you need one, open a
  discussion issue instead.
- **Do not add async cancel points inside the audit write path.** The WAL
  write must be atomic from the perspective of the calling future.
- **Do not log capability values at `INFO` level or above.** Capabilities
  are security tokens. Log at `TRACE` only, behind a compile-time feature
  flag `trace-capabilities` that is off by default.
- **Do not change the `LogEntry` schema without a migration plan.** Existing
  audit data must remain queryable after the change.
- **Do not add dependencies to `kernel-api`.** It must remain as thin as
  possible — `serde`, `uuid`, `thiserror`, and `async-trait` only. Every
  additional dependency in the public API crate is a dependency that all
  callers must accept.

---

## Protocol changes

The Agent Protocol is an open specification. Any change to the interfaces,
error taxonomy, or semantic constraints requires:

1. A Markdown proposal in `protocol-spec/proposals/YYYY-MM-DD-title.md`
   describing the motivation, the change, and the backward-compatibility
   impact.
2. Updates to the compliance test suite in
   `crates/agent-protocol/tests/compliance/` once that target implementation
   path exists in the repository.
3. The proposal merged to `main` before any implementation code is merged.

This process exists because the Protocol is consumed by third parties who
implement their own compatible layers. Breaking the Protocol silently is
worse than breaking the Kernel — it breaks the ecosystem.

---

## Performance budgets

| Operation | p50 target | p99 budget | Hard limit |
|---|---|---|---|
| `invoke` overhead (excl. action) | < 1 ms | < 5 ms | 10 ms |
| Audit write | < 0.5 ms | < 2 ms | 5 ms |
| Permission check | < 0.2 ms | < 1 ms | 2 ms |
| `query_audit` (30-day window) | < 50 ms | < 200 ms | 500 ms |

In the target implementation workspace, benchmarks live in
`crates/kernel-core/benches/`. Run with:

```bash
cargo bench -p kernel-core -- --output-format bencher | tee bench.txt
```

A PR that regresses any p99 budget requires an explicit sign-off from a
maintainer with a written justification.

---

## Building and running

These commands apply to the target Rust workspace once it exists in this
repository. They are not runnable in the current docs-only repository state.

```bash
# Build everything
cargo build --workspace

# Run all tests including compliance suite
cargo test --workspace

# Run only the Protocol compliance suite
cargo test -p agent-protocol --test compliance

# Run benchmarks
cargo bench -p kernel-core

# Check formatting and lints
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings

# Build the Python SDK (requires maturin)
cd sdk/python && maturin develop

# Build the TypeScript SDK (requires napi-rs)
cd sdk/typescript && npm run build
```

---

## Getting oriented

If you are new to this codebase, read in this order:

Current repository state:

1. `protocol-spec/overview.md` — understand the contract as currently documented
2. `ARCHITECTURE.md` — understand the top-level system model
3. `docs/architecture/README.md` — find the current architecture notes and diagram sources
4. `docs/architecture/2026-03-25-kernel-protocol-architecture.md` — the approved Kernel + Protocol refresh note

Target implementation state:

1. `protocol-spec/overview.md` — understand the contract before the implementation
2. `protocol-spec/constraints.md` — the six invariants everything else serves
3. `crates/kernel-api/src/lib.rs` — the four public methods; this is the whole surface
4. `crates/kernel-core/src/dispatch.rs` — the critical path
5. `crates/permission-engine/src/lib.rs` — the most security-critical subsystem
6. `crates/audit-log/src/lib.rs` — WAL implementation and integrity chain
7. `crates/agent-protocol/tests/compliance/` — what correct behaviour looks like

The compliance tests are executable documentation. When in doubt about
what the correct behaviour is in the target implementation workspace, read the
compliance tests first.
