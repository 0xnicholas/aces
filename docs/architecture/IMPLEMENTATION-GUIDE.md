# Implementation Guide

This guide is for developers working on the Agent Kernel implementation. It provides practical information for building, testing, and contributing to the Kernel.

**Prerequisites:**
- Familiarity with [protocol-spec/overview.md](../../protocol-spec/overview.md)
- Familiarity with [ARCHITECTURE.md](../../ARCHITECTURE.md)
- Familiarity with [AGENTS.md](../../AGENTS.md)

---

## Quick Start

### Development Environment

**Required:**
- Rust 1.75+ (stable toolchain)
- Linux (for sandbox development)
- Git

**Recommended:**
- Mermaid extension for diagram viewing
- `cargo-watch` for auto-rebuild
- `cargo-nextest` for faster test runs

### Building

```bash
# Build entire workspace
cargo build --workspace

# Build specific crate
cargo build -p kernel-core

# Build with optimizations (for benchmarks)
cargo build --release -p kernel-core
```

### Testing

```bash
# Run all tests
cargo test --workspace

# Run specific crate tests
cargo test -p kernel-core

# Run compliance suite only
cargo test -p agent-protocol --test compliance

# Run with nextest (faster)
cargo nextest run --workspace
```

---

## Code Organization

### Crate Structure

```
crates/
├── agent-protocol/        # MIT - Protocol spec, types, traits
│   ├── src/
│   │   ├── lib.rs        # Public interface
│   │   ├── types.rs      # AgentId, RunId, Capability, etc.
│   │   ├── errors.rs     # ProtocolError taxonomy
│   │   └── interfaces/   # Five interface families
│   └── tests/compliance/ # Protocol compliance tests
│
├── kernel-api/            # Apache 2.0 - Public API surface
│   └── src/lib.rs        # Four methods: spawn, invoke, revoke, query_audit
│
├── kernel-core/           # Apache 2.0 - Core orchestration
│   ├── src/
│   │   ├── lib.rs        # Public exports
│   │   └── dispatch.rs   # CRITICAL: invoke implementation
│   └── tests/            # Integration tests
│
├── permission-engine/     # Apache 2.0 - Capability evaluation
│   └── src/lib.rs        # CapabilitySet operations, intersection
│
├── scheduler/             # Apache 2.0 - Resource scheduling
│   └── src/lib.rs        # Token bucket, priority queue
│
├── sandbox/               # Apache 2.0 - Process isolation
│   └── src/lib.rs        # seccomp, namespaces (only unsafe code allowed)
│
├── audit-log/             # Apache 2.0 - WAL and integrity
│   └── src/lib.rs        # Append-only log, hash chain
│
└── agent-registry/        # Apache 2.0 - Agent lookup
    └── src/lib.rs        # AgentId -> AgentDef mapping
```

### Key Dependencies

**Protocol Crates:**
- `serde` - Serialization
- `uuid` - UUID v4 for identifiers
- `thiserror` - Error type definitions

**Kernel Crates:**
- `tokio` - Async runtime
- `tracing` - Structured logging
- `seccomp` - Syscall filtering (sandbox only)

**Common (both):**
- None - Keep minimal

---

## Critical Path: Dispatch

The `dispatch.rs` file in `kernel-core` is the **single most critical code path**. Every change here affects security and correctness.

### Implementation Requirements

```rust
// In kernel-core/src/dispatch.rs
pub async fn invoke(
    handle: &KernelHandle,
    action: Action,
    run_id: RunId,
    span_id: SpanId,
) -> Result<ActionResult, ProtocolError> {
    // 1. Identity validation (must be first)
    identity::validate(handle)?;
    
    // 2. Capability check (must be second)
    let caps = handle.capability_set();
    permission::check(caps, &action)?;
    
    // 3. Scheduler (reserve resources)
    let slot = scheduler::reserve(caps, &action).await?;
    
    // 4. Sandbox execution
    let result = sandbox::execute(action, slot).await;
    
    // 5. Audit WAL write (MUST complete before returning)
    let audit_entry = create_audit_entry(&result);
    audit_log::append(audit_entry).await?;  // Sync write!
    
    // 6. Return result
    Ok(result)
}
```

### Critical Invariants

1. **Step 2 and 5 cannot be skipped**
   - No shortcuts, no bypass flags
   - These steps are the reason the Kernel exists

2. **Audit write is synchronous**
   - Must be durable (fsync) before returning
   - Budget: < 2ms p99

3. **No panics in dispatch path**
   - Use `Result`, not `unwrap()` or `expect()`
   - Panics are bugs, not error handling

4. **Capability non-amplification**
   - For A2A calls: `delegated = caller ∩ target`
   - Must be computed by Kernel, not caller

### Testing Requirements

Every dispatch path change must test:

```rust
// Happy path
#[test]
fn invoke_valid_cap_succeeds() {
    // Valid capability, action executes, audit written
}

// Policy violation
#[test]
fn invoke_missing_cap_returns_policy_violation() {
    // Action blocked, audit written with error
}

// Audit ordering
#[test]
fn audit_seq_before_result_observable() {
    // Prove audit entry seq < result return timestamp
}

// Cancellation propagation
#[test]
fn cancel_propagates_to_children() {
    // Parent cancel cascades to all child spans
}
```

See `crates/kernel-core/tests/` for examples.

---

## Interface Contracts

### Between Crates

**kernel-api → kernel-core:**
- Input: `KernelHandle`, `Action`
- Output: `Result<ActionResult, ProtocolError>`
- Contract: All governance applied; result is final

**kernel-core → permission-engine:**
- Input: `CapabilitySet`, `Action`
- Output: `Result<(), PolicyViolation>`
- Contract: Pure function, no side effects

**kernel-core → scheduler:**
- Input: `CapabilitySet`, `Action`, priority hint
- Output: `Reservation` (or `ResourceExhausted`)
- Contract: Reservation valid until dropped; slot granted when available

**kernel-core → sandbox:**
- Input: `Action`, `Reservation`
- Output: `ActionResult` (success or error)
- Contract: Action executes in isolated context; resource limits enforced

**kernel-core → audit-log:**
- Input: `LogEntry`
- Output: `Result<(), AuditError>`
- Contract: Durable persistence before returning; strict ordering

### Public API (kernel-api)

Four methods, no more:

```rust
/// Spawn new Agent with given capabilities
pub fn spawn(
    def: AgentDef,
    caps: CapabilitySet,
) -> Result<KernelHandle, ProtocolError>;

/// Execute action (critical path)
pub async fn invoke(
    handle: &KernelHandle,
    action: Action,
) -> Result<ActionResult, ProtocolError>;

/// Revoke Agent handle
pub fn revoke(
    handle: KernelHandle,
) -> Result<RunSummary, ProtocolError>;

/// Query audit log (read-only)
pub fn query_audit(
    filter: AuditFilter,
) -> Result<Vec<LogEntry>, ProtocolError>;
```

**Do not add methods without ADR approval.**

---

## Testing Strategy

### Test Pyramid

```
       ┌─────────┐
       │Compliance│  (Protocol conformance)
       │   10%   │
      ┌┴─────────┴┐
      │ Integration│  (Cross-crate workflows)
      │    30%    │
     ┌┴───────────┴┐
     │    Unit      │  (Single function)
     │     60%      │
     └──────────────┘
```

### Unit Tests

- Location: Inline `#[cfg(test)]` in source files
- Scope: Single function, pure logic
- Mock: Dependencies via traits

```rust
// In permission-engine/src/lib.rs
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn intersection_empty_when_no_overlap() {
        let a = capability_set![ToolRead("foo")];
        let b = capability_set![ToolRead("bar")];
        assert!(a.intersection(b).is_empty());
    }
}
```

### Integration Tests

- Location: `crates/*/tests/*.rs`
- Scope: Cross-crate workflows
- Use real crate implementations (no mocks for audit/permission)

```rust
// In kernel-core/tests/dispatch_order.rs
#[tokio::test]
async fn invoke_enforces_permission_check() {
    let kernel = TestKernel::new();
    let agent = kernel.spawn_test_agent(
        capabilities: CapabilitySet::empty()
    );
    
    let result = kernel.invoke(agent, Action::ToolCall).await;
    
    assert_matches!(result, Err(ProtocolError::PolicyViolation { .. }));
    assert_audit_entry_written(agent.id(), PolicyViolation);
}
```

### Compliance Tests

- Location: `crates/agent-protocol/tests/compliance/`
- Scope: Protocol conformance
- Verifies all 6 semantic constraints
- Must pass for any Protocol implementation

```bash
# Run compliance suite
cargo test -p agent-protocol --test compliance
```

### Test Fixtures

**TestKernel:** In-memory Kernel for integration tests

```rust
let kernel = TestKernel::new()
    .with_mock_sandbox()  // Fast, no real isolation
    .with_memory_audit(); // In-memory audit log
```

**MockSandbox:** No-op sandbox for unit tests

```rust
// sandbox returns success immediately
// use for testing permission/audit logic only
```

---

## Performance Budgets

Hard limits for the critical path:

| Operation | p50 Target | p99 Budget | Hard Limit | Test |
|-----------|-----------|-----------|-----------|------|
| invoke overhead | < 1ms | < 5ms | 10ms | `benches/invoke_latency.rs` |
| Audit write | < 0.5ms | < 2ms | 5ms | `benches/audit_write.rs` |
| Permission check | < 0.2ms | < 1ms | 2ms | `benches/permission_check.rs` |
| query_audit (30d) | < 50ms | < 200ms | 500ms | `benches/audit_query.rs` |

### Benchmarking

```bash
# Run all benchmarks
cargo bench -p kernel-core

# Compare to baseline
cargo bench -p kernel-core -- --baseline main

# Generate report
cargo bench -p kernel-core -- --output-format bencher > bench.txt
```

### Performance Regression Policy

- PR must include benchmark results for dispatch-path changes
- p99 regression > 10% requires written justification
- Hard limit violations block merge

---

## Error Handling

### In Dispatch Path

```rust
// GOOD: Explicit error propagation
let validated = identity::validate(handle)
    .map_err(|e| {
        audit::write_error(&e);  // Audit even errors
        e
    })?;

// BAD: Panic in library code
let validated = identity::validate(handle).unwrap();  // NEVER
```

### Error Conversion

All errors must map to `ProtocolError` variants:

```rust
impl From<PermissionError> for ProtocolError {
    fn from(e: PermissionError) -> Self {
        match e {
            PermissionError::MissingCap(cap) => {
                ProtocolError::PolicyViolation { missing_cap: cap, .. }
            }
            // ... other variants
        }
    }
}
```

### No Untyped Errors

```rust
// BAD: String errors
return Err("something went wrong".into());

// GOOD: Structured error
return Err(ProtocolError::ResourceExhausted {
    resource: ResourceKind::LlmConcurrency,
    retry_after: Some(Duration::from_secs(60)),
});
```

---

## Debugging

### Logging

Use `tracing` (not `println!`):

```rust
use tracing::{info, debug, trace, error};

#[tracing::instrument(skip(handle, action))]
pub async fn invoke(handle: &KernelHandle, action: Action) -> Result<...> {
    debug!(agent_id = %handle.agent_id(), "invoking action");
    
    // ...
    
    if result.is_err() {
        error!(error = ?result, "invoke failed");
    }
    
    result
}
```

### Trace Levels

- **ERROR** - Actual bugs or security events
- **WARN** - Unusual but handled conditions
- **INFO** - Lifecycle events (spawn, revoke)
- **DEBUG** - Request/response (sanitized, no capabilities)
- **TRACE** - Detailed flow (capability values behind feature flag)

### Common Issues

**Issue:** `invoke` hangs
- **Check:** Scheduler deadlock? Audit log I/O blocked?
- **Debug:** `RUST_LOG=debug` to see where it stops

**Issue:** `PolicyViolation` when capability exists
- **Check:** Capability intersection for A2A calls
- **Debug:** Trace `caps_scope` in audit log

**Issue:** Audit integrity error
- **Check:** Storage corruption? Concurrent writes?
- **Debug:** Verify hash chain manually

---

## Release Process

### Versioning

Follow Semantic Versioning for each crate:

- **MAJOR** - Breaking API change (requires migration)
- **MINOR** - New functionality, backward compatible
- **PATCH** - Bug fixes, no API change

### Pre-Release Checklist

- [ ] All tests pass (`cargo test --workspace`)
- [ ] Compliance suite passes
- [ ] Benchmarks meet budgets
- [ ] Clippy clean (`cargo clippy --all-targets -- -D warnings`)
- [ ] Formatted (`cargo fmt --all -- --check`)
- [ ] Documentation builds (`cargo doc --no-deps`)
- [ ] CHANGELOG.md updated
- [ ] Version bumped in Cargo.toml

### Release Steps

```bash
# 1. Update version in crates/*/Cargo.toml
# 2. Update CHANGELOG.md
# 3. Commit: "Release kernel-core v0.2.0"
# 4. Tag: "git tag kernel-core-0.2.0"
# 5. Push: "git push && git push --tags"
# 6. CI builds and publishes to crates.io
```

---

## Getting Help

- **Architecture questions:** Review ADRs in `docs/architecture/decisions/`
- **Protocol questions:** Read `protocol-spec/overview.md`
- **Code questions:** Check inline docs (`cargo doc --open`)
- **Bugs:** Open issue with reproduction steps
- **Security issues:** See `SECURITY.md` for disclosure process

---

## Glossary

| Term | Definition |
|------|------------|
| **Critical path** | The `invoke` dispatch sequence; must be fast and correct |
| **WAL** | Write-Ahead Log; audit entry written before result observable |
| **Capability** | Unforgeable token granting right to perform action |
| **Intersection** | `caller ∩ target` for A2A delegation |
| **Sandbox** | Isolated execution context (seccomp + namespaces) |
| **Slot** | Scheduler reservation allowing execution to proceed |
| **Span** | Single step within a run; forms call tree via `parent_span_id` |
| **Run** | Top-level invocation; propagates through A2A calls |
