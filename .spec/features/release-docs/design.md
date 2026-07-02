---
type: feature-design
feature: release-docs
sibling: product.md
parent: ../../design.md
updated: 2026-07-03
colors:
  rainbow:
    red: "#FF4D4D"
    orange: "#FF9F45"
    yellow: "#FFD93D"
    green: "#6BCB77"
    blue: "#4D96FF"
    violet: "#9B5DE5"
  ink: "#1A1A2E"
  paper: "#FFFFFF"
typography:
  wordmark: "lowercase 'vibe', geometric rounded sans (SVG paths, no font dep)"
  tagline: "system sans, regular"
rounded:
  mark: "fully rounded terminals, friendly not corporate"
---

# Feature: Release Docs — Design (logo + banner)

## Overview

Wordmark-first identity, sibling energy to the `indexed` repo banner: image
banner at README top, tagline below, badge row under that. Rainbow carries
the "vibe" pun — spectrum = mood = flow.

## Colors

Six-stop rainbow (tokens above), applied either as a left-to-right linear
gradient across the wordmark, or one color per letter + mark. Must hold up
on both GitHub light and dark backgrounds — no pure black/white fills;
outline or ink color only for the tagline.

## Shapes

Candidate directions (impl produces 3–5, one per direction minimum):

1. **Gradient wordmark** — "vibe" filled with the 6-stop gradient.
2. **Per-letter** — v/i/b/e in four spectrum colors, dot of the i as accent.
3. **Wave mark + wordmark** — a small sine/tilde wave in gradient left of ink-colored wordmark.
4. **Terminal chrome** — wordmark inside a minimal terminal-window outline (nods to CLI home).

## Do's and Don'ts

- DO: pure SVG, no external fonts (convert text to paths), viewBox scalable, < 20 KB.
- DO: separate `logo.svg` (banner, ~800×200) usable as social preview base.
- DON'T: raster effects, drop shadows, more than 6 colors, uppercase.
