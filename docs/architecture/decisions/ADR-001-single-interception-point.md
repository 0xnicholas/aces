# ADR-001: Single Interception Point

## Status

Accepted

## Context

The Agent Kernel needs to enforce governance (permissions, audit, isolation) on all Agent actions. Without a single interception point, we would need to instrument every possible action path (tool calls, LLM calls, memory access, agent-to-agent calls) separately. This leads to:

1. **Incomplete enforcement** - Some paths might be missed or bypassed
2. **Inconsistent audit** - Different paths might log different information
3. **Duplicated logic** - Permission checks replicated across multiple entry points
4. **Framework burden** - Runtime developers must remember to call enforcement at every path

## Decision

All Agent actions MUST flow through a single method: `Invocation::invoke`.

This is the **only** entry point from Runtime to Kernel. All action types are submitted as variants of the `Action` type:

- `InvokeTool` - tool execution
- `LlmInfer` - LLM inference
- `MemoryRead/Write/Search` - context operations
- `CallAgent` - agent-to-agent calls
- `Notify` - event emission

The Kernel enforces the complete governance sequence for every `invoke` call:
```
invoke(action)
  -> validate handle
  -> check capabilities
  -> schedule/queue
  -> sandbox execution
  -> audit WAL write
  -> return result
```

## Consequences

### Positive

- **Complete coverage** - Cannot bypass governance; every action passes through the same enforcement logic
- **Simplified Runtime integration** - Runtime only needs to call one method
- **Centralized audit** - All actions logged in consistent format
- **Easier testing** - Single path to test for enforcement logic
- **Clear architecture** - Obvious where governance happens

### Negative

- **Performance bottleneck** - All traffic through single point; requires careful optimization
- **Complexity concentration** - The `invoke` implementation is critical and complex
- **Coupling** - All action types must be known at compile time (extensibility requires protocol changes)

### Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| Performance degradation | p99 budget: 5ms for invoke overhead (excl. action); optimize critical path |
| Implementation bugs in invoke | Comprehensive compliance test suite; fuzz testing |
| Action type explosion | Protocol versioning; careful design of Action enum |

## Alternatives Considered

### Alternative 1: Multiple Entry Points
Have separate methods for each action type: `invoke_tool()`, `invoke_llm()`, `invoke_memory()`, etc.

**Rejected because:**
- Higher chance of missing enforcement on some paths
- Duplicates validation and audit logic
- Harder to maintain consistency
- Framework might call wrong method or skip enforcement

### Alternative 2: Interceptor/Middleware Pattern
Allow Runtimes to register interceptors that the Kernel calls.

**Rejected because:**
- Relies on Runtime to register interceptors (not enforced)
- Order of interceptors matters and is hard to control
- Complex error handling across interceptor chain
- Violates "mandatory, un-bypassable" principle

### Alternative 3: Code Generation/Instrumentation
Automatically instrument action methods with governance logic.

**Rejected because:**
- Requires language-specific tooling
- Complex to implement correctly
- Harder to reason about (implicit behavior)
- Debuggability issues

## Related

- [protocol-spec/overview.md](../../protocol-spec/overview.md) §5.2 Invocation interface
- [ADR-003](ADR-003-synchronous-audit-write.md) - Related to audit in critical path
- [ADR-004](ADR-004-protocol-kernel-separation.md) - Protocol defines the invoke contract
