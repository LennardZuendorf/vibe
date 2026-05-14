---
type: entrypoint
scope: design
design_format: google-labs-code/design.md-compatible
children: []
updated: {YYYY-MM-DD}
# Optional visual design tokens. Keep when useful; delete unused groups.
colors:
  primary: "{#000000}"
  secondary: "{#666666}"
  neutral: "{#ffffff}"
typography:
  body:
    fontFamily: "{Font family}"
    fontSize: 16px
    fontWeight: 400
    lineHeight: 1.5
rounded:
  sm: 4px
  md: 8px
spacing:
  sm: 8px
  md: 16px
---

# {Project Name} — Design

Cross-cutting design language for the product. This template follows the spirit
of the google-labs-code `DESIGN.md` format: machine-readable tokens in YAML
frontmatter and human-readable rationale in markdown prose. Keep token groups
when this project has a visual interface; omit them for non-visual workflow
design.

**Product:** [product.md](product.md)
**Architecture:** [tech.md](tech.md)

---

## Overview

{Describe the product's look, feel, interaction personality, target audience,
and what agents should preserve when making design decisions.}

## Colors

{Explain semantic color roles and how the YAML tokens should be applied.}

## Typography

{Explain type hierarchy, font roles, rhythm, and reading density.}

## Layout

{Explain spacing, grid, density, responsive behavior, and containment.}

## Elevation & Depth

{Explain shadows, borders, layers, tonal contrast, or the lack of elevation.}

## Shapes

{Explain radius, edge language, icon shape, and component silhouettes.}

## Components

| Pattern | Use When | Notes |
|---|---|---|
| {Pattern} | {Situation} | {Constraint or example} |

## Do's and Don'ts

- Do {guideline}
- Don't {pitfall}

## Feature Design Index

| Feature | Design Detail |
|---|---|
| {Feature} | [features/{name}/design.md](features/{name}/design.md) |
