# ADR-005: Sandbox Isolation Strategy

## Status

Accepted

## Context

Agent actions must run in isolated contexts to prevent:
- Escape from sandbox to host system
- Unauthorized access to other Agents' data
- Resource exhaustion attacks
- Side-channel attacks

We need to choose an isolation technology:
1. **seccomp-bpf + Linux namespaces** - Kernel-level filtering + process isolation
2. **gVisor** - User-space kernel in Go
3. **Firecracker microVMs** - Lightweight virtualization
4. **Containers (Docker/runc)** - Process + filesystem isolation
5. **WebAssembly (WASM)** - Bytecode sandbox

## Decision

**Linux seccomp-bpf + namespaces** for the Kernel sandbox implementation.

### Architecture
```
┌─────────────────────────────────────┐
│         Agent Kernel Process        │
│  ┌─────────────────────────────┐    │
│  │      Sandbox Module         │    │
│  │  ┌─────────────────────┐    │    │
│  │  │  seccomp-bpf filter │    │    │
│  │  │  (syscall whitelist)│    │    │
│  │  └─────────────────────┘    │    │
│  │           ↓                 │    │
│  │  ┌─────────────────────┐    │    │
│  │  │  Linux namespaces   │    │    │
│  │  │  - PID (process)    │    │    │
│  │  │  - Mount (fs view)  │    │    │
│  │  │  - Network (isolate)│    │    │
│  │  │  - IPC (shm/msg)    │    │    │
│  │  └─────────────────────┘    │    │
│  │           ↓                 │    │
│  │  ┌─────────────────────┐    │    │
│  │  │  Agent Action       │    │    │
│  │  │  (restricted process)│   │    │
│  │  └─────────────────────┘    │    │
│  └─────────────────────────────┘    │
└─────────────────────────────────────┘
```

### seccomp-bpf Filter
- Whitelist approach: only explicitly allowed syscalls permitted
- Default deny: any non-whitelisted syscall returns EPERM
- Action-specific profiles: different filters for different action types

### Namespaces Used
- **PID namespace** - Process ID isolation (PID 1 in container)
- **Mount namespace** - Filesystem view restriction (read-only root, tmpfs for /tmp)
- **Network namespace** - Network isolation (optional; can be shared for outbound)
- **IPC namespace** - SysV IPC and POSIX message queues isolation
- **UTS namespace** - Hostname isolation (minor, completeness)

### Resource Limits (cgroups v2)
- CPU time quotas
- Memory limits with OOM killer
- File descriptor limits
- Process count limits

## Consequences

### Positive

- **Performance** - Native execution speed; no virtualization overhead
- **Ecosystem** - Mature, well-documented Linux primitives
- **Deployability** - Works on any Linux system (containers, VMs, bare metal)
- **Granularity** - Fine-grained syscall control via seccomp
- **Proven** - Used by Docker, Chrome, systemd, and many others

### Negative

- **Linux-only** - Not portable to macOS/Windows (requires separate implementation)
- **Complexity** - seccomp-bpf filters are complex to write and audit
- **Kernel attack surface** - Bugs in namespaces/seccomp could allow escape
- **Privilege requirements** - Requires CAP_SYS_ADMIN for some namespace operations

### Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| seccomp filter bypass | Minimal whitelist; extensive fuzz testing; kernel security updates |
| Namespace escape | Defense in depth: seccomp + namespaces + cgroups; run Kernel as non-root |
| Resource exhaustion | cgroup limits; ulimit; watchdog timers |
| Linux kernel CVE | Rapid patching process; vulnerability scanning |
| Privilege escalation | Drop all capabilities after setup; use user namespaces where possible |

## Performance Characteristics

| Metric | Target | Notes |
|--------|--------|-------|
| Sandbox creation | < 5ms | Clone() + namespace setup |
| Syscall overhead | < 1% | seccomp-bpf JIT compiled |
| Memory overhead | < 10MB | Per-sandbox baseline |

## Alternatives Considered

### Alternative 1: gVisor
User-space kernel implementing Linux syscall interface in Go.

**Rejected because:**
- Performance overhead: ~20-30% syscall latency increase
- Memory overhead: ~100MB per sandbox (too heavy for lightweight Agents)
- Complexity: More moving parts, harder to debug
- Use case mismatch: gVisor designed for untrusted containers, not fine-grained Agent actions

### Alternative 2: Firecracker MicroVMs
Lightweight virtualization using KVM.

**Rejected because:**
- Startup time: ~125ms (too slow for per-invoke sandbox)
- Memory overhead: ~15-20MB per VM
- Complexity: Requires KVM, nested virtualization concerns
- Better suited for long-running isolation, not per-action

### Alternative 3: Docker/runc Containers
Standard Linux container runtime.

**Rejected because:**
- Heavyweight for per-action isolation
- Docker daemon adds complexity and attack surface
- RunC alone is viable but adds layers we don't need
- We want tighter control than generic container runtime

### Alternative 4: WebAssembly (WASM)
Bytecode sandbox with capability-based security.

**Rejected because:**
- Requires rewriting tools/actions in WASM
- Limited ecosystem (most tools are native binaries)
- Performance: Good for compute, overhead for I/O-heavy operations
- Future option: Could add WASM sandbox as alternative execution mode

### Alternative 5: No Sandbox (Process-only)
Run actions in separate process without kernel-level isolation.

**Rejected because:**
- Insufficient isolation: process can access host filesystem, network
- No syscall filtering
- Violates security requirements

## Future Considerations

- **User namespaces** - May enable unprivileged sandbox creation (safer)
- **Landlock LSM** - Complementary to seccomp for filesystem access control
- **eBPF** - Could replace seccomp-bpf for more complex policy logic
- **WASM integration** - As alternative sandbox for supported workloads

## Related

- [AGENTS.md](../../AGENTS.md) §Code style - Only crate allowed to use `unsafe`
- Linux seccomp man page: `man 2 seccomp`
- Linux namespaces man page: `man 7 namespaces`
- [ADR-001](ADR-001-single-interception-point.md) - Sandbox is step 4 of invoke path
