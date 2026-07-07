# vibe logo candidates

Four SVG candidates, one per design direction from `design.md`.

| File | Direction | Description |
|---|---|---|
| `candidates/logo-1.svg` | Gradient wordmark | "vibe" stroked with a left-to-right 6-stop rainbow gradient |
| `candidates/logo-2.svg` | Per-letter | v=red, i=orange/yellow-dot, b=green, e=blue |
| `candidates/logo-3.svg` | Wave + wordmark | Rainbow sine-wave mark to the left of an ink-colored wordmark |
| `candidates/logo-4.svg` | Terminal chrome | Wordmark inside a rounded terminal window; three colored dots top-left |

**Active logo:** `logo.svg` — currently candidate 1 (gradient wordmark).

## Swapping the logo

Copy any candidate over `logo.svg`:

```sh
cp docs/img/candidates/logo-2.svg docs/img/logo.svg
```

## Usage note

`logo.svg` doubles as the social-preview base. GitHub recommends ~1280×640 for
Open Graph; the file ships at 800×200 which scales cleanly. For the social
preview upload (Settings → Social preview), export or rasterize at 2× if you
want sharp edges at 1280 wide.

Logo-3 uses ink (#1A1A2E) for the wordmark — high contrast on light
backgrounds; on very dark GitHub themes the wordmark fades. Candidates 1, 2,
and 4 use rainbow or ink strokes that hold up on both light and dark.
