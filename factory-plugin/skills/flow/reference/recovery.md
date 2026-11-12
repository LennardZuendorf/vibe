# Recovery and resume

## Cursor

Inspect the canonical cursor first. Fall back to a legacy cursor only when the
canonical file is absent and no persisted adoption marker exists. Any `runs`
record or versioned cursor forbids v1 fallback even after the current file disappears. A present invalid cursor is `blocked`: preserve it,
report the parse error/path, and request an explicit recovery decision. Never
silently select an older record or manufacture idle. Use only the packaged writer
for transitions; do not repair a cursor by editing JSON.

## Work

Read the ledger and compare plan, scope, dependencies and candidate bytes before
resuming. Reuse checked completed tasks while those identities remain valid.
Return missing context as `needs_context`; missing authority/reviewer as `blocked`;
failed checks as `failed`. Report exact remaining work instead of restarting all tasks.

On scope drift, stop affected writes and return to planning through the controller.
Invalidate affected approvals before editing frozen artifacts. On a failed check,
use systematic debugging: reproduce, isolate cause, add a regression check, then
apply the bounded fix and rerun verification. Do not change expected output to
hide a defect. Preserve baseline failures and report their delta.

## Interrupted migration

Preserve legacy cursor and receipts before changing discovery. Conflicting copies
must be diagnosed and left intact until reconciled. Retry idempotently from the
preserved data. Foreign skills, user files and marketplace caches are never
cleanup targets. Use the owning installer/package manager for obsolete owned entries.
