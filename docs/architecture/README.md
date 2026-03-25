# Architecture Docs

This directory is the working area for the repository's internal architecture
materials. It contains the approved architecture note for the current refresh,
the implementation plan that drove the documentation update, and the
source-controlled diagram files.

## Recommended Reading Order

- [../../protocol-spec/overview.md](../../protocol-spec/overview.md)
  Read this first for the Agent Protocol contract.
- [../../ARCHITECTURE.md](../../ARCHITECTURE.md)
  Read this next for the top-level system model and integration framing.
- [2026-03-25-kernel-protocol-architecture.md](2026-03-25-kernel-protocol-architecture.md)
  This is the approved internal architecture note for the current Kernel +
  Protocol refresh.

## Contents

- [2026-03-25-kernel-protocol-architecture.md](2026-03-25-kernel-protocol-architecture.md)
  Approved internal architecture note for the current refresh.
- [2026-03-25-kernel-protocol-implementation-plan.md](2026-03-25-kernel-protocol-implementation-plan.md)
  Working implementation plan used to drive the documentation refresh.
- [diagrams/kernel-protocol-main.mmd](diagrams/kernel-protocol-main.mmd)
  Main layered architecture diagram source.
- [diagrams/kernel-protocol-a2a.mmd](diagrams/kernel-protocol-a2a.mmd)
  Agent-to-Agent governance diagram source.
- [diagrams/kernel-protocol-invoke-sequence.mmd](diagrams/kernel-protocol-invoke-sequence.mmd)
  Invocation-path sequence diagram source.

## Diagram Sources

Mermaid files in [diagrams/](diagrams/) are the source of truth for the
refreshed architecture diagrams. Legacy image assets should be treated as
temporary until rendered outputs from these sources replace them.

## Maintenance

Add new architecture notes to this directory. Keep long-lived entry points
stable, and prefer linking to this directory-level index from top-level docs
instead of linking directly to date-stamped notes unless a specific note is the
point of reference.
