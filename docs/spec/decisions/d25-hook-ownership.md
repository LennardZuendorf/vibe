---
type: decision
updated: 2026-10-19
---

- **D25 — Hook ownership.** instruct owns SessionStart/UserPromptSubmit composition; flow owns guard/sniffer/Stop/ledger hooks; spec owns its own Stop/PostToolUse validation, and also spec owns a `SessionStart` router injection and a blocking `PreToolUse` spec-path tooth (D47). Only the validate hooks are bound to "exit 0 always".
