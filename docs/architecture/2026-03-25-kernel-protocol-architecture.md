# Kernel + Protocol Internal Architecture

## Purpose

This document captures the internal architecture position for the current
repository discussion:

- `Agent Protocol` is both an open specification and part of the Kernel
  product surface.
- The Kernel contains a concrete `Protocol Runtime` module that implements
  the Protocol.
- `Framework / Orchestration` and `Runtime` remain separate layers.
- Security-critical governance remains centralized in the Kernel.

This is an internal architecture note intended to guide diagram revision and
future implementation planning.

## Architectural Position

The system should represent `Protocol` in two ways at the same time:

1. As an external contract:
   `Agent Protocol` is the open spec and integration contract exposed to
   runtimes and third-party implementations.
2. As an internal Kernel module:
   `Protocol Runtime` is the Kernel's reference implementation of that
   contract.

These two roles must not be collapsed into a single unlabeled box. If they
are merged visually, the design becomes ambiguous:

- It becomes unclear whether the Protocol is an independent standard or just
  an internal Kernel detail.
- It becomes unclear whether the Kernel implements the Protocol or sits beside
  it.

The correct internal framing is:

`Protocol = standard / contract`

`Protocol Runtime = Kernel module implementing that standard`

## Main Layered View

The primary layered view is maintained in
`docs/architecture/diagrams/kernel-protocol-main.mmd`. It keeps
`Framework / Orchestration` and `Runtime` as separate layers, presents
`Agent Protocol` as the external contract, and shows `Protocol Runtime`
inside `Agent Kernel` with the validation, policy, registry, scheduler,
sandbox, and audit modules grouped beneath it.

## Why Identity / Handle Validation Should Be Inside Kernel

For the internal diagram, `Identity / Handle Validation` should not be drawn
as a vertical sidebar crossing all layers.

Reasoning:

- The internal review needs clear ownership boundaries.
- Identity propagation and authorization enforcement are different concerns.
- Authorization enforcement is a Kernel responsibility, not a shared
  cross-layer responsibility.

Recommended split:

- `Protocol Runtime` carries identity context such as `agent_id`, `run_id`,
  and `span_id`.
- `Identity / Handle Validation` validates `KernelHandle`, expiry, and
  revocation state.
- `Permission Engine` performs capability evaluation.

This keeps identity propagation visible without implying that every upper
layer participates in enforcement.

## Kernel Responsibilities

The Kernel is the mandatory enforcement layer. Its responsibilities are:

- Validate caller identity and handle state.
- Enforce capability policy.
- Resolve target agents for Agent-to-Agent calls.
- Schedule execution according to capacity and quota constraints.
- Run actions inside isolated execution context.
- Write tamper-evident audit records.
- Return results only after required governance steps complete.

The Kernel should be described internally as:

`Reference implementation of the Agent Protocol and the sole governance enforcement point.`

## Critical Invocation Path

The dynamic path is more important than the static boxes. The architecture
must preserve this exact order:

```text
invoke
  -> identity / handle validation
  -> permission check
  -> scheduler
  -> sandbox execution
  -> audit WAL write
  -> return ActionResult
```

The architecture note attached to the diagram should explicitly state:

- All external actions enter through `invoke`.
- No direct `Agent -> Execution Substrate` path exists.
- Result must not be observable before audit WAL requirements are satisfied.

## Agent-to-Agent Governance View

Agent-to-Agent calling is maintained in
`docs/architecture/diagrams/kernel-protocol-a2a.mmd`. The diagram preserves
the governance sequence for caller handle validation, target lookup,
`delegated_caps = caller ∩ target`, child span creation, run/span
propagation, audit log entry, and child dispatch. `caps_hint` remains
advisory only, and the child always executes with `delegated_caps` only.

## Required Invariants To Show In Review

Any internal architecture review should evaluate the design against these
explicit invariants:

1. Single interception point
   Every externally meaningful action enters through `invoke`.

2. Capability non-amplification
   `delegated_caps = caller_caps ∩ target_caps`

3. Forced governance
   No Agent-to-Agent call bypasses the Kernel.

4. Trace continuity
   `run_id` propagates through the call tree and `parent_span_id` links
   parent/child execution.

5. WAL audit ordering
   Required audit persistence must complete before the result becomes
   observable.

## Diagram Guidance

For the revised internal diagram set:

- Use one main structural diagram for layers and Kernel modules.
- Use one A2A sub-diagram for delegation and trace propagation.
- Optionally use one small sequence diagram for the invocation path.

Recommended labels:

- External layer: `Agent Protocol (Open Spec / Contract)`
- Internal Kernel module: `Protocol Runtime`
- Validation module: `Identity / Handle Validation`
- Policy module: `Permission Engine`
- Audit module: `Audit Log (WAL + Integrity Chain)`

Avoid these ambiguous patterns:

- Drawing `Protocol` only outside the Kernel without showing Kernel
  implementation ownership.
- Drawing `Protocol` only inside the Kernel and losing the "open standard"
  meaning.
- Drawing `Identity / Handle Validation` as a single cross-layer sidebar for
  internal review.

## Summary

The internal architecture should present the system as a layered stack with
an explicit external `Agent Protocol` contract and an internal Kernel
`Protocol Runtime` implementation. The Kernel remains the only enforcement
point for identity validation, authorization, scheduling, isolation, and
audit. `Framework / Orchestration` and `Runtime` remain separate layers, and
Agent-to-Agent governance should be shown in a dedicated sub-diagram focused
on non-amplifying delegation and trace continuity.

## Diagram Sources

- `docs/architecture/diagrams/kernel-protocol-main.mmd`
- `docs/architecture/diagrams/kernel-protocol-a2a.mmd`
- `docs/architecture/diagrams/kernel-protocol-invoke-sequence.mmd`
