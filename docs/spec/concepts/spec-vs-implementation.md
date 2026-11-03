---
type: concept
updated: 2026-11-03
---

# Spec vs Implementation

The spec Rust implementation is present: F1, F2’s Spec JSON slice, F3, F4, F16–F20 and F25 are DONE in the feature sequence. `vibe-spec` provides discovery, queries, validation, promotion, drift checks, OpenSpec interchange, hooks, setup, init and migration. The legacy `spec/scripts/*.sh` remain until F23; F21–F24 remain unfinished. The flow and instruct Rust ports remain pending; the existing JS/bash implementations still run.

C0 global-store is implemented on this branch; publication and runtime installation require their own evidence. The v0.5 arc C1–C6 remains NOT STARTED. It gates on F22, with C4 after F16 and C5 after F14. C0’s thin `vibe spec` dispatch does not complete C1’s peer-discovery and registry scope.
