# Internal Ops Agent Governance Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the first end-to-end Internal Ops Agent Governance MVP from the current docs-only repository state through a demo-capable controlled action path.

**Architecture:** Start by turning the repository into a minimal Rust workspace that can express the MVP object model, `kernel-api`, and a narrow `kernel-core` dispatch path. Then add explicit capability enforcement, structured audit recording, approval interrupts, and bounded `CallAgent` support until the internal ops demo flow is fully supported.

**Tech Stack:** Rust workspace, Cargo, `thiserror`, `serde`, `uuid`, `tracing`, Markdown docs, example in-memory fixtures

---

## File Structure

Planned implementation structure for the MVP:

- Create: `Cargo.toml`
  Responsibility: workspace root for the MVP implementation crates.
- Create: `crates/agent-protocol/src/lib.rs`
  Responsibility: MVP protocol subset exports.
- Create: `crates/agent-protocol/src/types.rs`
  Responsibility: core shared types for the MVP object model.
- Create: `crates/agent-protocol/src/errors.rs`
  Responsibility: canonical ProtocolError subset for the MVP.
- Create: `crates/kernel-api/src/lib.rs`
  Responsibility: public Kernel API surface (`spawn`, `invoke`, `revoke`, `query_audit`).
- Create: `crates/kernel-core/src/lib.rs`
  Responsibility: core orchestration exports and Kernel construction.
- Create: `crates/kernel-core/src/dispatch.rs`
  Responsibility: the controlled invocation path.
- Create: `crates/permission-engine/src/lib.rs`
  Responsibility: explicit capability enforcement and delegation intersection.
- Create: `crates/audit-log/src/lib.rs`
  Responsibility: in-memory MVP audit recording and query support.
- Create: `crates/agent-registry/src/lib.rs`
  Responsibility: target lookup for child-agent invocation.
- Create: `crates/approval/src/lib.rs`
  Responsibility: interrupt, approve, reject lifecycle for dangerous actions.
- Create: `crates/example-tools/src/lib.rs`
  Responsibility: MVP tool fixtures (`read_logs`, `read_metrics`, `restart_service`, `open_incident`).
- Create: `crates/demo-scenarios/src/lib.rs`
  Responsibility: stable demo flows and sample runs.
- Create: `crates/kernel-core/tests/*.rs`
  Responsibility: end-to-end MVP behavior tests.
- Modify: `README.md`
  Responsibility: point to the implementation workspace once it exists.
- Modify: `AGENTS.md`
  Responsibility: update repository state from docs-only to mixed docs + MVP implementation workspace once the bootstrap lands.
- Modify: `ARCHITECTURE.md`
  Responsibility: align any now-existing workspace paths after the MVP skeleton lands.

Notes:

- Keep the MVP narrower than the long-term architecture. Only build the object
  and module surface required for the internal ops demo.
- Prefer in-memory implementations for registry, audit, and tool fixtures in
  the first pass.

## Task 1: Bootstrap The Rust Workspace

**Files:**
- Create: `Cargo.toml`
- Create: `crates/agent-protocol/Cargo.toml`
- Create: `crates/kernel-api/Cargo.toml`
- Create: `crates/kernel-core/Cargo.toml`
- Create: `crates/permission-engine/Cargo.toml`
- Create: `crates/audit-log/Cargo.toml`
- Create: `crates/agent-registry/Cargo.toml`
- Create: `crates/approval/Cargo.toml`
- Create: `crates/example-tools/Cargo.toml`
- Create: `crates/demo-scenarios/Cargo.toml`

- [ ] **Step 1: Create the workspace directories**

Run:

```bash
mkdir -p crates/agent-protocol/src crates/kernel-api/src crates/kernel-core/src \
  crates/permission-engine/src crates/audit-log/src crates/agent-registry/src \
  crates/approval/src crates/example-tools/src crates/demo-scenarios/src \
  crates/kernel-core/tests
```

Expected: directories are created with no errors

- [ ] **Step 2: Write the root workspace manifest**

Create `Cargo.toml` with:

```toml
[workspace]
members = [
  "crates/agent-protocol",
  "crates/kernel-api",
  "crates/kernel-core",
  "crates/permission-engine",
  "crates/audit-log",
  "crates/agent-registry",
  "crates/approval",
  "crates/example-tools",
  "crates/demo-scenarios",
]
resolver = "2"
```

- [ ] **Step 3: Write each crate manifest with only MVP dependencies**

Each crate `Cargo.toml` should be minimal and explicit. Example for
`crates/agent-protocol/Cargo.toml`:

```toml
[package]
name = "agent-protocol"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { version = "1", features = ["derive"] }
thiserror = "2"
uuid = { version = "1", features = ["serde", "v4"] }
```

Use similarly narrow dependency sets for each crate.

- [ ] **Step 4: Add stub `lib.rs` files so the workspace compiles**

Example `crates/kernel-core/src/lib.rs`:

```rust
pub mod dispatch;
```

Example `crates/kernel-core/src/dispatch.rs`:

```rust
pub fn placeholder_dispatch() {}
```

- [ ] **Step 5: Run workspace compilation to validate the bootstrap**

Run: `cargo check --workspace`
Expected: workspace compiles successfully

- [ ] **Step 6: Commit**

Run:

```bash
git add Cargo.toml crates
git commit -m "build: bootstrap internal ops governance workspace"
```

## Task 2: Define The MVP Protocol Subset And Core Object Model

**Files:**
- Create: `crates/agent-protocol/src/lib.rs`
- Create: `crates/agent-protocol/src/types.rs`
- Create: `crates/agent-protocol/src/errors.rs`
- Test: `crates/agent-protocol/tests/mvp_types.rs`

- [ ] **Step 1: Write the failing protocol type tests**

Create `crates/agent-protocol/tests/mvp_types.rs` with tests for:

```rust
use agent_protocol::{Action, Capability, ProtocolError};

#[test]
fn restart_service_action_exists() {
    let action = Action::RestartService { service: "service-a".into() };
    match action {
        Action::RestartService { service } => assert_eq!(service, "service-a"),
        _ => panic!("wrong variant"),
    }
}

#[test]
fn interrupted_error_exists() {
    let err = ProtocolError::Interrupted;
    assert_eq!(err.to_string(), "interrupted");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p agent-protocol --test mvp_types`
Expected: FAIL because types are not fully defined yet

- [ ] **Step 3: Implement the MVP object model**

Define in `types.rs`:

- `AgentId`
- `RunId`
- `SpanId`
- `KernelHandle`
- `Capability`
- `CapabilitySet`
- `Action`
- `ActionResult`
- `AuditEntry`
- `ApprovalInterrupt`

Minimum `Action` variants:

```rust
pub enum Action {
    ReadLogs { service: String },
    ReadMetrics { service: String },
    RestartService { service: String },
    OpenIncident { service: String, summary: String },
    CallAgent { target_id: AgentId, action: Box<Action> },
}
```

- [ ] **Step 4: Implement the ProtocolError subset**

Define in `errors.rs`:

```rust
#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("policy violation")]
    PolicyViolation,
    #[error("interrupted")]
    Interrupted,
    #[error("cancelled")]
    Cancelled,
    #[error("invalid handle")]
    InvalidHandle,
}
```

- [ ] **Step 5: Export the public types**

`lib.rs` should re-export:

```rust
pub mod errors;
pub mod types;

pub use errors::ProtocolError;
pub use types::*;
```

- [ ] **Step 6: Run the protocol tests again**

Run: `cargo test -p agent-protocol --test mvp_types`
Expected: PASS

- [ ] **Step 7: Commit**

Run:

```bash
git add crates/agent-protocol
git commit -m "feat: define internal ops mvp protocol subset"
```

## Task 3: Implement The Public Kernel API Surface

**Files:**
- Create: `crates/kernel-api/src/lib.rs`
- Test: `crates/kernel-core/tests/kernel_api_surface.rs`

- [ ] **Step 1: Write the failing API-surface test**

Create `crates/kernel-core/tests/kernel_api_surface.rs`:

```rust
use kernel_api::KernelApi;

#[test]
fn kernel_api_exposes_four_methods() {
    fn assert_api<T: KernelApi>() {}
    struct Marker;
    assert_api::<Marker>();
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p kernel-core --test kernel_api_surface`
Expected: FAIL because `KernelApi` is not defined yet

- [ ] **Step 3: Define the API trait and handle-oriented surface**

Create `crates/kernel-api/src/lib.rs`:

```rust
use agent_protocol::{Action, ActionResult, AuditEntry, CapabilitySet, KernelHandle, ProtocolError};

pub trait KernelApi {
    fn spawn(&mut self, caps: CapabilitySet) -> Result<KernelHandle, ProtocolError>;
    fn invoke(&mut self, handle: &KernelHandle, action: Action) -> Result<ActionResult, ProtocolError>;
    fn revoke(&mut self, handle: KernelHandle) -> Result<(), ProtocolError>;
    fn query_audit(&self) -> Result<Vec<AuditEntry>, ProtocolError>;
}
```

- [ ] **Step 4: Provide a dummy implementer in the test**

Update the test to include a minimal `Marker` implementer so the trait shape is
validated.

- [ ] **Step 5: Run the test again**

Run: `cargo test -p kernel-core --test kernel_api_surface`
Expected: PASS

- [ ] **Step 6: Commit**

Run:

```bash
git add crates/kernel-api crates/kernel-core/tests/kernel_api_surface.rs
git commit -m "feat: add mvp kernel api surface"
```

## Task 4: Build The Minimal Dispatch Skeleton

**Files:**
- Modify: `crates/kernel-core/src/lib.rs`
- Modify: `crates/kernel-core/src/dispatch.rs`
- Test: `crates/kernel-core/tests/dispatch_order.rs`

- [ ] **Step 1: Write the failing dispatch behavior test**

Create `crates/kernel-core/tests/dispatch_order.rs` with a simple event-capture
test:

```rust
#[test]
fn dispatch_records_permission_before_execution() {
    let events = vec!["validate", "permission", "execute", "audit"];
    assert_eq!(events[0], "validate");
    assert_eq!(events[1], "permission");
}
```

Then replace the placeholder with a real dispatch-backed assertion once the
dispatch object exists.

- [ ] **Step 2: Implement a dispatch context and event order**

In `dispatch.rs`, introduce a minimal function that performs:

- handle validation
- permission check hook
- tool execution hook
- audit write hook

The first implementation can use in-memory collaborators and strings rather
than final abstractions.

- [ ] **Step 3: Expose the dispatch entry from `kernel-core`**

`crates/kernel-core/src/lib.rs` should export the core type or function needed
for the MVP.

- [ ] **Step 4: Update the test to assert the actual dispatch order**

The final expected order for the MVP skeleton should be:

```text
validate -> permission -> execute_or_interrupt -> audit -> return
```

- [ ] **Step 5: Run the dispatch test**

Run: `cargo test -p kernel-core --test dispatch_order`
Expected: PASS

- [ ] **Step 6: Commit**

Run:

```bash
git add crates/kernel-core
git commit -m "feat: add kernel dispatch skeleton"
```

## Task 5: Add Explicit Capability Enforcement

**Files:**
- Create: `crates/permission-engine/src/lib.rs`
- Test: `crates/kernel-core/tests/permission_enforcement.rs`

- [ ] **Step 1: Write the failing permission test**

Create `crates/kernel-core/tests/permission_enforcement.rs`:

```rust
#[test]
fn investigator_cannot_restart_service() {
    let denied = true;
    assert!(denied);
}
```

Then replace with a real dispatch-backed denial assertion.

- [ ] **Step 2: Implement capability matching**

In `crates/permission-engine/src/lib.rs`, define simple matching logic between:

- `Capability::ReadLogs`
- `Capability::ReadMetrics`
- `Capability::RestartService`
- `Capability::OpenIncident`

and the corresponding `Action` variants.

- [ ] **Step 3: Integrate permission checks into dispatch**

If permission is missing:

- return `ProtocolError::PolicyViolation`
- do not execute the tool action

- [ ] **Step 4: Run the permission test**

Run: `cargo test -p kernel-core --test permission_enforcement`
Expected: PASS

- [ ] **Step 5: Commit**

Run:

```bash
git add crates/permission-engine crates/kernel-core
git commit -m "feat: enforce mvp capabilities before execution"
```

## Task 6: Add Audit Recording And Query

**Files:**
- Create: `crates/audit-log/src/lib.rs`
- Modify: `crates/kernel-api/src/lib.rs`
- Modify: `crates/kernel-core/src/dispatch.rs`
- Test: `crates/kernel-core/tests/audit_query.rs`

- [ ] **Step 1: Write the failing audit test**

Create `crates/kernel-core/tests/audit_query.rs`:

```rust
#[test]
fn denied_actions_are_visible_in_audit() {
    let entries = vec!["policy_violation"];
    assert_eq!(entries.len(), 1);
}
```

Then replace with a real audit query assertion.

- [ ] **Step 2: Implement an in-memory audit store**

In `crates/audit-log/src/lib.rs`, provide:

- append
- list
- filtering by run id if practical in the MVP

- [ ] **Step 3: Record both success and denial paths**

Update dispatch so:

- successful actions append an audit entry
- denied actions append an audit entry
- interrupted actions append an audit entry

- [ ] **Step 4: Wire `query_audit` through the public API**

The MVP can return the full in-memory vector of entries.

- [ ] **Step 5: Run the audit test**

Run: `cargo test -p kernel-core --test audit_query`
Expected: PASS

- [ ] **Step 6: Commit**

Run:

```bash
git add crates/audit-log crates/kernel-api crates/kernel-core
git commit -m "feat: add in-memory audit log and query support"
```

## Task 7: Add Approval Interrupts For Dangerous Actions

**Files:**
- Create: `crates/approval/src/lib.rs`
- Modify: `crates/kernel-core/src/dispatch.rs`
- Test: `crates/kernel-core/tests/approval_interrupt.rs`

- [ ] **Step 1: Write the failing interrupt test**

Create `crates/kernel-core/tests/approval_interrupt.rs`:

```rust
#[test]
fn restart_service_requires_approval() {
    let interrupted = true;
    assert!(interrupted);
}
```

Then replace with a real assertion on the dispatch result state.

- [ ] **Step 2: Implement a simple dangerous-action classifier**

In the MVP, only `Action::RestartService` should trigger approval.

- [ ] **Step 3: Implement interrupt state**

Use `ProtocolError::Interrupted` or `ActionResult` status metadata to represent
approval pending.

- [ ] **Step 4: Add approve/reject behavior in the approval crate**

The first version can be an in-memory approval registry keyed by run/span or
pending action id.

- [ ] **Step 5: Run the interrupt test**

Run: `cargo test -p kernel-core --test approval_interrupt`
Expected: PASS

- [ ] **Step 6: Commit**

Run:

```bash
git add crates/approval crates/kernel-core
git commit -m "feat: interrupt dangerous actions for approval"
```

## Task 8: Add Child-Agent Registry And Capability Intersection

**Files:**
- Create: `crates/agent-registry/src/lib.rs`
- Modify: `crates/permission-engine/src/lib.rs`
- Modify: `crates/kernel-core/src/dispatch.rs`
- Test: `crates/kernel-core/tests/call_agent.rs`

- [ ] **Step 1: Write the failing child-invocation test**

Create `crates/kernel-core/tests/call_agent.rs`:

```rust
#[test]
fn child_agent_receives_intersected_capabilities() {
    let intersection_applied = true;
    assert!(intersection_applied);
}
```

Then replace with a real assertion against the active child capability set.

- [ ] **Step 2: Implement the agent registry**

The MVP registry needs only:

- register agent
- look up agent by id
- retrieve target capability set

- [ ] **Step 3: Implement capability intersection**

In `permission-engine`, add a helper equivalent to:

```rust
pub fn delegated_caps(caller: &CapabilitySet, target: &CapabilitySet) -> CapabilitySet
```

- [ ] **Step 4: Integrate `CallAgent` into dispatch**

The dispatch path must:

- resolve the target
- compute intersected capabilities
- spawn child execution context
- preserve `run_id`
- create a fresh `span_id`

- [ ] **Step 5: Run the child-invocation test**

Run: `cargo test -p kernel-core --test call_agent`
Expected: PASS

- [ ] **Step 6: Commit**

Run:

```bash
git add crates/agent-registry crates/permission-engine crates/kernel-core
git commit -m "feat: support bounded child agent invocation"
```

## Task 9: Add Example Tools And Stable Demo Fixtures

**Files:**
- Create: `crates/example-tools/src/lib.rs`
- Create: `crates/demo-scenarios/src/lib.rs`
- Test: `crates/kernel-core/tests/demo_flow.rs`

- [ ] **Step 1: Write the failing demo-flow test**

Create `crates/kernel-core/tests/demo_flow.rs`:

```rust
#[test]
fn ops_demo_flow_reaches_interrupt_then_completion() {
    let flow = vec!["metrics", "logs", "call_agent", "interrupt", "approved", "restart"];
    assert_eq!(flow[0], "metrics");
}
```

Then replace with a real end-to-end scenario assertion.

- [ ] **Step 2: Implement example tool adapters**

In `crates/example-tools/src/lib.rs`, provide deterministic in-memory behavior
for:

- `read_logs`
- `read_metrics`
- `open_incident`
- `restart_service`

- [ ] **Step 3: Implement the demo scenario helper**

In `crates/demo-scenarios/src/lib.rs`, provide a canonical scenario that:

- creates `ops-investigator`
- creates `ops-executor`
- runs the investigation
- requests a restart
- triggers an interrupt
- resumes after approval

- [ ] **Step 4: Run the demo-flow test**

Run: `cargo test -p kernel-core --test demo_flow`
Expected: PASS

- [ ] **Step 5: Commit**

Run:

```bash
git add crates/example-tools crates/demo-scenarios crates/kernel-core
git commit -m "feat: add internal ops demo fixtures"
```

## Task 10: Align Repository Docs With The New MVP Workspace

**Files:**
- Modify: `README.md`
- Modify: `AGENTS.md`
- Modify: `ARCHITECTURE.md`
- Modify: `docs/architecture/README.md`

- [ ] **Step 1: Update the README to reflect the mixed docs + implementation repository**

Add links to the new MVP implementation plan and workspace overview.

- [ ] **Step 2: Update AGENTS.md current repository state**

Change the `Current repository state` section so it no longer says the
repository is docs-only.

- [ ] **Step 3: Update ARCHITECTURE.md where target-state references have become real**

Only revise references that now exist in the repository.

- [ ] **Step 4: Verify docs for broken links**

Run: `python3 - <<'PY'\nfrom pathlib import Path\nimport re\nfiles=[Path('README.md'),Path('docs/architecture/README.md')]\npat=re.compile(r'\\[[^\\]]+\\]\\(([^)]+)\\)')\nfor f in files:\n    for target in pat.findall(f.read_text()):\n        if '://' in target or target.startswith('#'):\n            continue\n        assert (f.parent/target).resolve().exists(), (f, target)\nprint('ok')\nPY`
Expected: `ok`

- [ ] **Step 5: Commit**

Run:

```bash
git add README.md AGENTS.md ARCHITECTURE.md docs/architecture/README.md
git commit -m "docs: align repository docs with mvp workspace"
```

## Verification Sequence

Before claiming the MVP implementation complete, run:

```bash
cargo check --workspace
cargo test --workspace
```

Expected:

- all crates compile
- all kernel-core integration tests pass
- the internal ops flow supports:
  - read-only investigation
  - denied direct restart
  - child-agent delegation
  - approval-gated restart
  - audit replay

## Execution Notes

- Keep every crate narrow. Resist early platform breadth.
- Use in-memory collaborators in the first implementation unless a persistence
  abstraction is required for clarity.
- Prefer integration tests in `crates/kernel-core/tests/` for MVP behavior
  rather than prematurely distributing complex test harnesses across crates.
- If the MVP object model changes materially during implementation, update
  `docs/2026-03-26-internal-ops-agent-governance-technical-design-plan.md`
  before proceeding further.

## Review Constraint

This implementation plan is meant to be executed against the current repository
state, which is documentation-heavy and implementation-light. The first
meaningful milestone is not feature breadth; it is a coherent governed control
path that makes the internal ops demo credible.
