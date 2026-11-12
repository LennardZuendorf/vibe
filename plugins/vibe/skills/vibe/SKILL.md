---
name: vibe
description: Use when asking what the Vibe toolkit does, which component to use, or how to get started. Direct workflow requests use flow; direct knowledge and artifact operations use spec.
---

<!-- vibe-spec: literal-file -->

# Vibe toolkit

Vibe connects durable project knowledge with a guided development lifecycle.
This guide orients and dispatches once. It does not own workflow state.

## Choose one capability

| Request | Owner |
| --- | --- |
| Build, fix, refactor, plan implementation, resume work or verify a candidate | `flow` (plugin: `vibe:flow`) |
| Discover, author, validate or query a spec; record a durable decision | `spec` (plugin: `vibe:spec`) |
| Explain the toolkit or compare its installed capabilities | This guide |
| Ordinary read-only code question or unrelated task | Answer directly |

Explicit `$flow` and `$spec` requests go straight to that component. Do not
require this guide first. Once dispatched, the component owns the task. A Spec
operation requested by Flow returns its result to Flow; it does not restart it.
A delegated worker follows its bounded brief and returns to its controller.

## Discover before dispatch

Read the available skill catalog and its source paths. Folder names determine
skill discovery: `vibe` is toolkit orientation, `flow` is lifecycle, `spec` is
knowledge. Report duplicate/shadowed copies and the selected source. Conflicting
versions require the user's normal package manager remediation; do not rewrite
foreign caches or invent a preferred provider.

For project context, run `vibe-spec root` when available; read the discovered
root's `plan.md` and relevant lessons. Without the binary, inspect existing
`docs/spec/` or legacy `.spec/` read-only and report that validation is unavailable. <!-- literal -->
No root is an onboarding condition: explain Spec setup, then obtain the task
scope before scaffolding. Do not initialize a spec merely to answer a question.

If the chosen component is absent, name it and give its installation or package
repair action. Do not install tools or modify host configuration without a
request. Standalone Spec works without this guide or Flow. Instruct integration
is deferred; do not advertise an Instruct skill or commands as installed.

## Example

“Add CSV export” selects Flow once. Flow discovers the spec and obtains a design
and plan before implementation approval. “Validate the spec” selects Spec once.
“What is Vibe?” explains this capability map without starting a lifecycle.
