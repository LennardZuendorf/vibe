# Research: Framework Landscape

Captured: 2026-03-13

## Frameworks Evaluated

### GSD (Get Stuff Done)
- **What it does well:** Wave-based task execution, XML plan format, plan immutability, gap closure pattern
- **What's too much:** ~50+ files, 12 custom agents, custom CLI tool, heavy infrastructure
- **Verdict:** Cherry-pick patterns (waves, immutability, gap closure), don't adopt wholesale

### Superpowers
- **What it does well:** Brainstorming skill, TDD enforcement, dual-stage review, pressure-resistant phase enforcement
- **What's too much:** Tightly coupled — hard to use just one piece
- **Verdict:** Use as a plugin provider for DISCUSS (brainstorm), IMPLEMENT (TDD), REVIEW (dual-stage)

### Feature-Dev
- **What it does well:** Specialized agents (code-explorer, code-architect, code-reviewer), confidence scoring, architecture proposals
- **What's too much:** Doesn't enforce a full lifecycle
- **Verdict:** Use as a plugin provider for RESEARCH (explorer), PLAN (architect), REVIEW (reviewer)

### Simplify
- **What it does well:** Multi-agent code review with different perspectives (reuse, quality, efficiency)
- **What's too much:** Nothing — it's focused and well-scoped
- **Verdict:** Bundle as default REVIEW provider

## Key Insight

No single framework gets everything right. The best approach is orchestration:
- Use GSD's execution patterns as **built-in defaults**
- Use Superpowers' skills as **optional plugins**
- Use Feature-Dev's agents as **optional plugins**
- Use Simplify as a **bundled default**
- Use our Spec system as the **non-negotiable backbone**

## Patterns Worth Adopting (Built-In)

| Pattern | Source | Why It's Good |
|---------|--------|---------------|
| Wave-based task grouping | GSD | Natural parallelization boundary |
| Plan immutability | GSD | Prevents drift during implementation |
| Gap closure | GSD | Forces plan updates before continuing |
| Phase enforcement | Superpowers | Prevents skipping research/spec |
| Pressure resistance | Superpowers | Politely refuses "just code it" |
| Fresh subagent per task | GSD | Prevents context rot |
| File-based communication | Our spec system | Survives session boundaries |

## Patterns That Stay Plugin-Only

| Pattern | Source | Why It's Optional |
|---------|--------|------------------|
| Brainstorming skill | Superpowers | Not every task needs it |
| TDD enforcement | Superpowers | Not every project uses TDD |
| Competing architect proposals | Feature-Dev | Overhead for small features |
| Confidence-scored reviews | Feature-Dev | Built-in review is often enough |
