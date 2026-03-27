# ADR-003: Synchronous Audit Write

## Status

Accepted

## Context

The audit log must provide a complete, tamper-evident record of all Agent actions. A critical question is: **when is the audit entry written relative to the action result being observable?**

Options:
1. **Asynchronous** - Write audit in background; return result immediately
2. **Synchronous** - Wait for audit write to complete before returning result
3. **Hybrid** - Write initial entry synchronously, completion entry asynchronously

The key requirement: if an action executes but the audit entry is lost, the action is undetectable.

## Decision

Audit log entries MUST be written **synchronously** before the action result is returned to the caller.

Specifically:
1. For non-streaming actions: Complete WAL write before returning `ActionResult`
2. For streaming actions: Write initial entry with `status: Pending` before stream opens; write completion entry when stream closes
3. For errors: Write audit entry even for failed/rejected actions

WAL (Write-Ahead Log) semantics:
- Entry is durably persisted (survives process restart)
- Entry is written to append-only storage
- Integrity chain includes hash of previous entry
- No buffering or batching that could lose entries on crash

## Consequences

### Positive

- **Audit completeness** - Every observable action has a corresponding audit entry
- **Tamper evidence** - Synchronous write ensures entry exists before result can be used
- **Debugging** - Audit log is authoritative record; can reconstruct exact sequence
- **Compliance** - Meets "before result is returned" requirements for regulations

### Negative

- **Latency increase** - Audit write adds to critical path latency
- **Throughput limit** - Audit storage I/O becomes bottleneck
- **Failure mode** - Audit write failure blocks action result (availability vs durability tradeoff)

### Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| p99 latency exceeds budget (2ms for audit) | Optimize storage: SSD, dedicated audit partition, batched fsync |
| Audit storage unavailable | Fail closed: return error rather than proceed without audit |
| Audit corruption detected | `AuditIntegrityError`; halt operations until resolved |

## Performance Budget

| Operation | p50 Target | p99 Budget | Hard Limit |
|-----------|-----------|-----------|-----------|
| Audit write | < 0.5ms | < 2ms | 5ms |
| Full invoke | < 1ms | < 5ms | 10ms |

## Alternatives Considered

### Alternative 1: Asynchronous Audit
Write audit entry in background; return result immediately.

**Rejected because:**
- Race condition: action result used before audit persisted
- Crash between result return and audit write = lost audit entry
- Violates "audit is authoritative record" principle

### Alternative 2: Fire-and-Forget
Send audit entry to external service; don't wait for confirmation.

**Rejected because:**
- Network/service failures cause silent audit loss
- External service latency unpredictable
- Complex error handling (what if audit service is down?)

### Alternative 3: Periodic Batch Write
Buffer audit entries; write batch periodically.

**Rejected because:**
- Crash loses buffered entries
- Higher latency spikes during batch writes
- Complex recovery logic

### Alternative 4: Two-Phase Audit
Phase 1: Write "intent" entry synchronously, return result
Phase 2: Write "completion" entry asynchronously

**Rejected because:**
- Still allows result use before full audit
- Complex state machine (what if Phase 2 never happens?)
- Doesn't solve the core problem

## Related

- [protocol-spec/overview.md](../../protocol-spec/overview.md) §7.6 WAL Audit constraint
- [protocol-spec/overview.md](../../protocol-spec/overview.md) §10 Audit Log Schema
- [ADR-001](ADR-001-single-interception-point.md) - Audit is part of invoke critical path
