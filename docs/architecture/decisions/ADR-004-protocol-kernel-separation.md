# ADR-004: Protocol-Kernel Separation

## Status

Accepted

## Context

The Agent Kernel project contains two related but distinct components:

1. **Agent Protocol** - The open specification defining interfaces, data types, and semantic constraints
2. **Agent Kernel** - The reference implementation providing enforcement

Key questions:
- Should they be separate projects?
- What licenses should each use?
- Can third parties implement the Protocol independently?

## Decision

**Dual structure with dual licensing:**

### Agent Protocol
- **License:** MIT
- **Location:** `crates/agent-protocol` (within this repo)
- **Nature:** Open specification + trait definitions + compliance test suite
- **Purpose:** Enable third-party implementations; ecosystem interoperability

### Agent Kernel
- **License:** Apache 2.0
- **Location:** `crates/kernel-*` crates
- **Nature:** Reference implementation + enforcement layer
- **Purpose:** Provide production-ready governance with patent protection

### Relationship
```
┌─────────────────────────────────────────┐
│        Agent Protocol (MIT)             │
│  - Interface definitions                │
│  - Data types (AgentId, RunId, etc.)    │
│  - Error taxonomy                       │
│  - Semantic constraints                 │
│  - Compliance test suite                │
└─────────────────┬───────────────────────┘
                  │ "implements"
┌─────────────────▼───────────────────────┐
│        Agent Kernel (Apache 2.0)        │
│  - Protocol Runtime (impl)              │
│  - Permission Engine                    │
│  - Scheduler                            │
│  - Sandbox                              │
│  - Audit Log                            │
└─────────────────────────────────────────┘
```

Third parties can:
- ✅ Implement the Protocol independently (using MIT-licensed spec)
- ✅ Use the Kernel as-is (Apache 2.0 allows commercial use)
- ✅ Fork and modify the Kernel (with patent grant)
- ❌ Sue contributors over patents (Apache 2.0 patent retaliation clause)

## Consequences

### Positive

- **Ecosystem growth** - MIT license encourages Protocol adoption
- **Patent protection** - Apache 2.0 protects Kernel contributors
- **Business friendly** - Both licenses allow commercial use
- **Standardization** - Clear separation enables independent implementations
- **Compliance authority** - Test suite defines "correct" behavior, not implementation

### Negative

- **Complexity** - Two licenses to understand and comply with
- **Repository coupling** - Both in same repo (convenience) but different licenses
- **Patent confusion** - Apache 2.0 patent clause applies only to Kernel, not Protocol

### Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| License contamination | Clear crate-level LICENSE files; separate directories |
| Contributor confusion | Document which license applies to which files |
| Patent trolling | Apache 2.0 retaliation clause for Kernel; MIT has no patent protection |

## License Summary

| Component | License | Patent Grant | Commercial Use | Modification |
|-----------|---------|--------------|----------------|--------------|
| Agent Protocol | MIT | ❌ No | ✅ Yes | ✅ Yes |
| Agent Kernel | Apache 2.0 | ✅ Yes | ✅ Yes | ✅ Yes |

## Alternatives Considered

### Alternative 1: Single MIT License
Both Protocol and Kernel under MIT.

**Rejected because:**
- No patent protection for Kernel contributors
- Business partners may require patent grant for production use
- MIT is excellent for specs but insufficient for implementation

### Alternative 2: Single Apache 2.0 License
Both Protocol and Kernel under Apache 2.0.

**Rejected because:**
- Apache 2.0 is more restrictive than necessary for a spec
- Might discourage Protocol adoption by projects with GPL compatibility concerns
- MIT is industry standard for interface specs (e.g., POSIX specs)

### Alternative 3: Separate Repositories
Protocol in one repo (MIT), Kernel in another (Apache 2.0).

**Rejected because:**
- Complicates development (cross-repo PRs)
- Harder to keep spec and implementation in sync
- Single repo with clear separation is simpler

### Alternative 4: GPL/Copyleft
Use GPL for Kernel to enforce open source.

**Rejected because:**
- Too restrictive for adoption
- Apache 2.0 provides patent protection without copyleft requirements
- Business partners unlikely to integrate GPL components

## Related

- `LICENSE-MIT` (in project root, applies to Protocol crates)
- `LICENSE-APACHE` (in project root, applies to Kernel crates)
- [protocol-spec/overview.md](../../protocol-spec/overview.md) §1 License statement
- [AGENTS.md](../../AGENTS.md) Target workspace structure
