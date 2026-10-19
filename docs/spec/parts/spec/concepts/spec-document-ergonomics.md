---
type: concept
updated: 2026-10-19
---

# Spec Document Ergonomics

The spec framework should feel like a small map, not a documentation maze:

- Root docs answer project-level questions; feature docs answer one buildable unit.
- Branch docs are rare — only for concerns spanning multiple features.
- `design.md` is first-class when UX, interaction, language, or workflow ergonomics matter; omit token groups for non-visual workflow design.
- Feature authoring follows the 6-step interview flow in `spec`'s `feature.md` (locate → WHAT → rigor gate → HOW → plan → skip).
- Requirements adopt OpenSpec's grammar for interchange: `### Requirement: <ID> — <name>` (SHALL/MUST), `#### Scenario:` with GIVEN/WHEN/THEN bullets, and `## ADDED|MODIFIED|REMOVED Requirements` delta headers for feature proposals. The document model — root vs. feature vs. branch docs — stays vibe's own; only the requirement grammar is shared with OpenSpec.
- The spec root defaults to `docs/spec/`; `.spec/` still resolves for repos that haven't migrated. <!-- literal -->
- Global lessons (v0.5) answer "what has this developer learned anywhere"; repo lessons answer "what has this project learned." On conflict, the more specific memory — the repo's — wins.
