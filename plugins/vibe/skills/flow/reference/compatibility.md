# Compatibility and availability

`vibe` is the toolkit guide; `flow` is the workflow skill (`vibe:flow` in a plugin);
`spec` remains standalone. Discovery uses folder names, not just frontmatter.
Flow runtime remains `.vibe/run/flow`; source resources live under `flow/` and
installed resources under `.agents/skills/flow/`. Legacy runtime paths remain
readable during migration. Old `/flow` redirects may remain; no second workflow
body is advertised as `vibe`.

Use `vibe-flow --help` and the packaged machine for available commands/states.
V2 implements revision-checked transitions, additive compound before verification,
immutable run records and candidate-bound observations. Production verified human
authority and trusted evidence remain unavailable/deny. Legacy v1 uses convention
only and cannot write or certify an adopted run. Node/shell hooks refuse adopted
runs when the Rust readiness runtime is unavailable. A configured hook or valid
skill is not a native behavioral proof; report configured/activated/verified separately.

`vibe-flow state candidate` includes tracked, untracked and ignored content,
tracked deletions, modes and symlink targets. It excludes Git metadata, Cargo
`target` output and `.vibe/run/flow` runtime records. It reports a framed
`git-object:` digest without requiring a commit. Candidate identity is not a
trusted collector or approval. See [authorization](authorization.md) for interfaces.

External design, planning, TDD, debugging and review skills provide bounded
techniques. Do not invoke their lifecycle executors or finishing steps. Missing
method helpers may be self-executed from these procedure contracts; missing
independent review remains blocked. Native named agents are optional, and a
generic independent agent can consume the same role brief. Never silently
substitute a model forbidden by controlling user preferences.

Missing Spec means report unavailable artifact operations and normal installation
remediation. Missing root means explicitly requested Spec setup, not automatic
scaffolding. Do not advertise deferred Instruct/provider integrations.

## Bounded method selection

| Step | Available method skill |
| --- | --- |
| Design | `superpowers:brainstorming` |
| Plan | `superpowers:writing-plans` |
| Implement | `superpowers:test-driven-development` |
| Diagnose | `superpowers:systematic-debugging` |
| Review feedback | `superpowers:receiving-code-review` |
| Verify | `superpowers:verification-before-completion` |
| Independent review | `superpowers:requesting-code-review` |

Use each method only for its assigned step. Flow retains sequencing and gate ownership.
