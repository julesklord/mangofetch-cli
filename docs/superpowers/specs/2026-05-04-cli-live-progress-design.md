# CLI Live Progress Redesign

## Summary

This spec defines the first visual redesign phase for the MangoFetch CLI. The scope is limited to live download progress rendering in the terminal. The redesign should make active downloads feel modern, bold, and highly readable while preserving the current non-interactive CLI model and keeping a solid ASCII fallback.

## Goals

- Make live downloads feel premium and visually intentional.
- Prioritize expressive per-download progress over dense compact output.
- Preserve compatibility with the current `indicatif` and `MultiProgress` architecture.
- Improve hierarchy for phase, progress, speed, ETA, and transfer size.
- Add horizontal margin and vertical breathing room so output does not feel glued to terminal edges.
- Keep a reasonable fallback path for terminals with limited Unicode support.

## Non-Goals

- No migration to a full TUI or dashboard workflow in this phase.
- No redesign of `list`, `info`, `check`, or other static command output in this phase.
- No custom stdout renderer that replaces `indicatif`.
- No broad refactor of core download or queue logic.

## User Experience Direction

The visual direction is brutalist and high-contrast: sharp, punchy, terminal-native, and serious rather than playful. The output should feel alive, but not noisy. Every active download should read like a focused activity card rather than a single thin status line.

The layout should favor clarity over density. We accept higher vertical space usage in exchange for stronger hierarchy and more legible status cues.

## Primary Approach

This redesign builds on the existing `indicatif`-based reporter instead of replacing it.

### Why this approach

- It keeps the current progress update pipeline intact.
- It reduces implementation risk and compatibility regressions.
- It allows a large visual improvement without changing the product model from CLI to TUI.
- It leaves room for future evolution if a dedicated interactive mode is added later.

## Visual Structure

Each active download should render as a multi-line block with left and right margin.

### Block anatomy

1. Header line
   Contains download id, truncated title, and colored platform label.
2. Phase line
   Contains the normalized phase name as the most visually prominent status label.
3. Progress bar line
   Contains a wider, higher-impact progress bar and percent.
4. Metrics line
   Contains speed, ETA, and downloaded vs total bytes.
5. Detail line
   Contains a short contextual message secondary to the phase.
6. Spacer
   Adds breathing room between download blocks.

### Margins and spacing

- Apply a fixed horizontal indent to active blocks so they sit away from the terminal edge.
- Keep one blank or near-blank visual spacer between download blocks.
- Completion and error summaries should be more compact than active blocks so finished work clears out cleanly.

## Phase Model

Raw status strings should be normalized into a smaller set of visual phases:

- `PREPARING`
- `FETCHING`
- `DOWNLOADING`
- `PROCESSING`
- `FINALIZING`
- `COMPLETE`
- `ERROR`

### Intent

- `phase` is the dominant state label.
- `detail` is a short secondary message that preserves useful context without overwhelming the layout.
- Unknown or irregular raw messages should map to the closest stable phase rather than leaking inconsistent vocabulary into the UI.

## Transition Behavior

The output should make state changes feel deliberate.

### Rules

- When a download changes phase, emit a short transition message in the shared progress area.
- Transition copy should be brief and readable, for example entering processing, finalizing media, or resolving metadata.
- Active progress blocks should update in place through `indicatif`.
- `COMPLETE` and `ERROR` should remove the active block and replace it with compact terminal messages that preserve the visual style without leaving stale bars on screen.

## Progress Bar Design

The bar should feel heavier and more modern than the current default.

### Requirements

- Increase visual width relative to the current implementation.
- Prefer a Unicode bar set for modern terminals.
- Keep percent visible and easy to scan.
- Preserve an ASCII fallback with the same information architecture.
- Avoid characters known to render inconsistently in common Windows terminals.

## Metrics Hierarchy

The metrics line should expose the most important operational data in a stable order:

1. Speed
2. ETA
3. Downloaded bytes / total bytes

### Formatting expectations

- Speed should be compact and human-readable.
- ETA should stay readable at a glance.
- Byte counts should be normalized and consistent.
- Missing totals should degrade gracefully instead of rendering noisy placeholders.

## Title and Platform Treatment

- Titles should truncate cleanly, preserving useful identity.
- Platform labels should remain color-coded.
- The header should still be readable for long titles and narrow terminals.

If the terminal width is constrained, the renderer should degrade by shortening title content before dropping core metrics.

## System Progress

System-level progress such as dependency setup or tool downloads should reuse the same visual language where practical, but remain distinguishable from media downloads.

### Distinction rules

- Use a system-oriented prefix or label.
- Keep the same margin and hierarchy principles.
- Do not visually imply that a dependency task is a normal user download.

## Accessibility and Compatibility

The preferred experience assumes a modern terminal with strong Unicode and ANSI color support. Even so, the implementation must preserve a solid fallback path.

### Compatibility rules

- Unicode path is the primary path.
- ASCII path must preserve structure, not just raw data.
- Color should enhance meaning but not be the only carrier of meaning.
- Reset codes must remain reliable so adjacent terminal output is not polluted.

## Files In Scope

- `mangofetch-cli/src/reporter.rs`
- `mangofetch-cli/tests/integration_tests.rs`

Potentially small supporting changes may be added if needed, but the redesign should stay centered in the reporter layer.

## Testing Strategy

This work should be implemented with test-first coverage around formatting behavior.

### Test coverage targets

- Phase normalization from raw messages.
- Multi-line active block rendering.
- Margins and spacing around active blocks.
- Compact completion message rendering.
- Compact error message rendering.
- Unicode and ASCII fallback behavior.
- Stable formatting of speed, ETA, and byte counts.

### Test style

- Prefer direct tests against formatting helpers and render output fragments.
- Keep tests focused on behavior, not incidental ANSI byte counts unless those counts matter to correctness.
- Verify that fallback output preserves structure and readability.

## Risks and Constraints

- `indicatif` templates can be limiting for complex multi-line presentation.
- Terminal width varies; aggressive formatting must degrade gracefully.
- Unicode characters that look good in one terminal may render poorly in another, especially on Windows.
- Excessive transition chatter could make the output feel noisy if not constrained.

## Recommendation

Implement the redesign as an evolution of the current reporter with a dedicated formatting layer for:

- phase normalization
- card-like active block rendering
- compact terminal event messages
- Unicode and ASCII variants

This yields the strongest visual improvement for the least architectural risk.
