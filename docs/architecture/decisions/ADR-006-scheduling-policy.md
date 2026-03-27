# ADR-006: Scheduling Policy

## Status

Accepted

## Context

The Kernel must manage access to limited resources:
- **LLM API rate limits** - External services (OpenAI, Anthropic, etc.) have TPM/RPM limits
- **Tool call quotas** - Some tools have rate limits or cost per call
- **Compute resources** - CPU/memory for local tool execution
- **Fairness** - Multiple Agents sharing the Kernel should get fair access

We need a scheduling policy that:
1. Respects external rate limits (avoid being throttled/banned)
2. Prioritizes interactive vs background work
3. Handles bursty workloads
4. Provides fairness across Agents
5. Meets latency budgets

## Decision

**Hybrid: Token bucket + Priority queue**

### Token Bucket (Rate Limiting)
For LLM concurrency and tool quotas:

```
┌─────────────────────────────────────┐
│         Token Bucket                │
│  ┌─────┐  ┌─────┐  ┌─────┐         │
│  │ 🪙  │  │ 🪙  │  │ 🪙  │  ...    │
│  └─────┘  └─────┘  └─────┘         │
│       ↑ tokens refill at fixed rate │
│       ↓ each request consumes 1     │
│  ┌─────────────────────────────┐    │
│  │       Request Queue         │    │
│  │  ┌───┐┌───┐┌───┐┌───┐      │    │
│  │  │ R ││ R ││ R ││ R │      │    │
│  │  └───┘└───┘└───┘└───┘      │    │
│  └─────────────────────────────┘    │
└─────────────────────────────────────┘
```

- **Tokens** represent quota units (e.g., 1 token = 1 LLM call)
- **Refill rate** matches external API limits (e.g., 100 TPM)
- **Bucket size** allows burst up to limit (e.g., bucket size = 10)
- **Multiple buckets** - One per resource type (LLM, expensive tools, etc.)

### Priority Queue (Workload Prioritization)
For ordering requests within quota:

| Priority | Use Case | Example |
|----------|----------|---------|
| **Critical** | User-facing, blocking | Chat response, UI interaction |
| **High** | Important business logic | Workflow step, data sync |
| **Normal** | Standard operations | Background task, batch job |
| **Low** | Best-effort, deferrable | Analytics, cleanup, indexing |

```
┌────────────────────────────────────────────┐
│           Priority Queue                   │
│                                            │
│  ┌────────────────────────────────────┐    │
│  │ 🔴 Critical │ R1 │ R2 │ R3 │       │    │
│  ├────────────────────────────────────┤    │
│  │ 🟠 High     │ R4 │ R5 │            │    │
│  ├────────────────────────────────────┤    │
│  │ 🟡 Normal   │ R6 │ R7 │ R8 │ R9 │  │    │
│  ├────────────────────────────────────┤    │
│  │ 🟢 Low      │ R10 │ R11 │ ... │   │    │
│  └────────────────────────────────────┘    │
│                                            │
└────────────────────────────────────────────┘
```

- Higher priority requests are dequeued first
- Within same priority: FIFO (fair queuing)
- Preemption: Critical priority can interrupt Normal/Low (optional, TBD)

### Combined Flow

```
Incoming Request
       ↓
   ┌─────────┐
   │ Priority │──→ Priority Queue
   │  Label  │        ↓
   └─────────┘   ┌──────────┐
       ↓         │  Head of │──→ Check Token Bucket
   ┌─────────┐   │  Queue   │
   │  Token  │◀──│          │◀── Tokens available?
   │ Bucket  │   └──────────┘
   └────┬────┘       │No
        │Yes         └──→ Wait in queue
        ↓
   Execute Action
```

## Consequences

### Positive

- **Rate limit compliance** - Token bucket prevents exceeding external limits
- **Latency control** - Priority ensures critical work isn't stuck behind batch jobs
- **Fairness** - FIFO within priority prevents starvation
- **Flexibility** - Runtime can specify priority; Kernel enforces
- **Predictability** - Token bucket provides smooth rate limiting

### Negative

- **Complexity** - Two mechanisms to understand and tune
- **Priority inversion** - Low-priority action holding resource can block high-priority (mitigated by preemption)
- **Configuration burden** - Requires tuning bucket sizes, refill rates, priorities
- **Head-of-line blocking** - If head of queue lacks tokens, queue stalls

### Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| Priority starvation | Timeout on queue wait; priority boost for aging requests |
| Token bucket overflow | Configurable bucket size; overflow queue with backoff |
| Priority abuse | Runtime must specify priority; audit log records priority used |
| Thundering herd | Jitter on token refill; exponential backoff on rate limit |

## Configuration

```rust
struct SchedulingConfig {
    // Token bucket for LLM calls
    llm_bucket: TokenBucketConfig {
        capacity: 10,           // Burst up to 10 concurrent calls
        refill_rate: 100,       // 100 tokens per minute
        refill_period: 60s,
    },
    
    // Token bucket for expensive tools
    tool_bucket: TokenBucketConfig {
        capacity: 5,
        refill_rate: 10,
        refill_period: 60s,
    },
    
    // Queue limits
    max_queue_depth: 1000,      // Per-priority queue limit
    queue_timeout: 30s,         // Max time in queue before timeout
}
```

## Performance Characteristics

| Metric | Target | Notes |
|--------|--------|-------|
| Scheduling decision | < 0.1ms | Token bucket check + queue insertion |
| Queue wait time | < 10s p99 | Depends on load and priorities |
| Token refill | Smooth | No bursts at refill boundaries |

## Alternatives Considered

### Alternative 1: Fair Queuing Only
Pure fair queuing without rate limiting.

**Rejected because:**
- No protection against external rate limits
- Could overwhelm LLM APIs, leading to throttling
- No differentiation between interactive and batch work

### Alternative 2: Strict Priority (No Token Bucket)
Priority-only scheduling without rate limiting.

**Rejected because:**
- High-priority requests could exhaust external quotas
- No burst control
- Still vulnerable to rate limit violations

### Alternative 3: Weighted Fair Queuing
Assign weights to Agents; schedule proportionally.

**Rejected because:**
- Complex to configure and understand
- Doesn't address external rate limits
- Weights don't map well to priority semantics

### Alternative 4: Earliest Deadline First (EDF)
Schedule based on request deadlines.

**Rejected because:**
- Hard to set appropriate deadlines
- Doesn't prevent rate limit violations
- Complex admission control

### Alternative 5: Admission Control Only
Reject requests when load is high.

**Rejected because:**
- Poor user experience (failures instead of queuing)
- Doesn't differentiate priority
- Wastes work already done (e.g., context loaded)

## Future Considerations

- **Work stealing** - Idle Agents can process low-priority work
- **Priority inheritance** - If high-priority waits on low-priority, boost low priority
- **Dynamic priorities** - Adjust based on wait time (aging)
- **Resource reservation** - Reserve capacity for critical workloads
- **Multi-tenant isolation** - Separate token buckets per tenant/organization

## Related

- [protocol-spec/overview.md](../../protocol-spec/overview.md) §5.2 Invocation - `ResourceExhausted` error
- [protocol-spec/overview.md](../../protocol-spec/overview.md) §6 Error taxonomy
- [AGENTS.md](../../AGENTS.md) Performance budgets (5ms invoke p99)
- [ADR-001](ADR-001-single-interception-point.md) - Scheduler is step 3 of invoke path
