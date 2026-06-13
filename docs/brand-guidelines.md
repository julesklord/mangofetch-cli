# MangoFetch Brand Guidelines

## Purpose

This document establishes the core brand identity, voice, and messaging for MangoFetch. It is intended to guide documentation, repository copy, and command-line branding.

## Product Name and Tagline

- Product name: **MangoFetch**
- Core tagline: **High-performance, standalone download engine for the modern terminal.**
- Supporting phrase: **Built in Rust for speed, reliability, and broad platform compatibility.**

## Brand Promise

MangoFetch gives power users a fast, reliable, and brutalist way to download media and files from the terminal. It is optimized for low-latency execution and high-volume batch processing.

Key promises:
- Universal platform support (1000+ sites)
- Native performance (Rust core)
- Scriptable, automation-friendly workflows
- Transparent dependency lifecycle management

## Audience

Primary audiences:
- Developers and sysadmins who need automated download workflows
- Power users who prefer terminal-first tools
- Archivists performing high-volume batch processing
- Operators managing downloads in headless environments

## Voice and Tone

MangoFetch copy should feel:
- **Practical and direct**: Monospace-first, minimal fluff.
- **Brutalist**: Prioritizing function and speed over decorative UI.
- **Reliable**: Technical details are precise and grounded in performance.

Avoid:
- "Lite" or "Companion" messaging (MangoFetch is a standalone evolution).
- Marketing fluff or vague claims.
- Complex setups that distract from the "one command" experience.

## Messaging Pillars

1. **Native Performance**
   - "Rust-powered speed."
   - Focus on low resource usage and fast startup.

2. **Automation-First**
   - Built for pipes, scripts, and headless servers.
   - Clean JSON output and predictable CLI flags.

3. **Lifecycle Management**
   - No manual setup for FFmpeg or yt-dlp.
   - "Just run it; we handle the rest."

4. **Evolution**
   - An independent evolution of the OmniGet project, refactored for the terminal.

## Visual Style

- **Theme**: Brutalist, high-contrast, terminal-friendly.
- **Colors**: Deep oranges (Rust), clean whites, and terminal greens (Success).
- **Typography**: Monospace (JetBrains Mono, Fira Code) for all technical examples.

## Documentation Guidelines

- Primary documentation language: English.
- Keep headings short and scannable.
- Use terminal-style code blocks for all examples.
- Use consistent terminology: "download", "queue", "platform", "dependency".

## Relationship with OmniGet

MangoFetch is an independent evolution of the OmniGet project. While it shares technical lineage, it is its own brand with a specific focus on CLI performance and standalone reliability. Documentation should acknowledge this lineage in "History" or "About" sections but avoid calling it a "companion" or "version" of OmniGet.

## Remaining CLI UI Work Instructions

This section is the handoff for `gemini-cli` / `antigravity` to complete the remaining CLI interface work with minimal guesswork.

### Current State

- Phase 1 of the live progress redesign is already implemented in `mangofetch-cli/src/reporter.rs`.
- The live progress renderer now uses:
  - multi-line activity blocks
  - left margin (`ACTIVE_BLOCK_MARGIN`)
  - normalized phases (`PREPARING`, `FETCHING`, `DOWNLOADING`, `PROCESSING`, `FINALIZING`, `COMPLETE`, `ERROR`)
  - stronger Unicode-first bars with ASCII fallback
  - compact transition, retry, completion, error, and system-progress events
- The remaining work is not in `reporter.rs`. It is primarily the static CLI output layer in `mangofetch-cli/src/output.rs`.

### Primary Remaining Scope

Redesign the following static command outputs so they match the new brutalist live-progress language:

- `format_info_card`
- `format_queue_list`
- `format_dependency_check`
- `format_clean_summary`
- `format_batch_summary`
- `format_config_display` only if needed for consistency

Do not turn this into a TUI. Keep the product as a CLI with rich text output.

### Visual Direction To Preserve

The remaining output must match the existing live-progress redesign:

- brutalist, high-contrast, terminal-native
- modern and expressive, but not cute or playful
- obvious hierarchy with strong primary labels and quieter secondary detail
- generous left margin so content does not touch terminal edges
- compact but readable spacing between sections
- Unicode-first presentation with sane ASCII fallback

Do not reintroduce mojibake-prone characters like the old box-drawing set currently present in `output.rs` (`Ú`, `¿`, `À`, `Ù`, `Ä`, `³`) unless terminal compatibility is explicitly revalidated. Prefer plain ASCII structure or Unicode known to render reliably.

### Non-Negotiable UX Rules

- All redesigned static output must have left padding comparable to the live progress blocks.
- Section titles must be short, uppercase or near-uppercase, and visually dominant.
- Platform labels should stay color-coded through `CliTheme`.
- Success, warning, active, and error states must remain visually distinguishable without relying only on color.
- Long titles must truncate cleanly rather than breaking alignment.
- Empty states must feel intentional, not like fallback debug text.

### File Ownership

- Main implementation file: `mangofetch-cli/src/output.rs`
- If shared helpers are needed and truly reusable, they may be moved into a small formatting helper module, but avoid broad refactors.
- `mangofetch-cli/src/reporter.rs` should only be touched if visual parity requires a tiny shared helper extraction. Do not reopen the live-progress architecture casually.

### Concrete Output Goals By Function

#### `format_info_card`

Target shape:
- a compact media detail card
- strong title/header
- consistent label/value rhythm
- colored platform treatment
- no legacy pseudo-box characters

Requirements:
- preserve title, author, platform, type, and duration
- add left margin
- keep width stable enough to scan in a terminal
- if a border is used, prefer simple and robust characters

#### `format_queue_list`

Target shape:
- a queue overview that feels like a stack of active records rather than a flimsy table
- active items should feel visually closer to the new live progress blocks

Requirements:
- preserve `id`, `title`, `platform`, `status`, and optional inline progress
- improve empty state copy
- align status treatment with the new phase language where practical
- avoid brittle fixed-width layouts that collapse badly on long titles

#### `format_dependency_check`

Target shape:
- a system readiness panel
- clear distinction between ready, missing, and auto-install behavior

Requirements:
- keep `yt-dlp` and `FFmpeg`
- success path should feel crisp and trustworthy
- missing path should still feel controlled, not alarming

#### `format_clean_summary`

Target shape:
- short success receipt
- minimal but polished

Requirements:
- keep removed item count
- keep freed disk space
- singular/plural handling must remain correct

#### `format_batch_summary`

Target shape:
- punchy end-of-run summary
- clear separation between total, queued, and failed

Requirements:
- success-only case should still feel complete, not sparse
- failure case must be prominent without becoming noisy

### Testing Expectations

The remaining work must add or improve tests. Do not rely on visual hope.

Minimum test coverage to add:
- info card contains the expected semantic fields after redesign
- queue list empty state
- queue list active item rendering with inline progress
- dependency check success and missing states
- batch summary success and failure variants
- clean summary singular/plural behavior

Preferred test location:
- `mangofetch-cli/tests/integration_tests.rs`

If that file is too noisy, it is acceptable to add focused tests inside `output.rs`, but keep the coverage explicit.

### Implementation Constraints

- Keep shared business logic out of the formatting layer.
- This is a presentation refactor, not a queue-engine refactor.
- Reuse `CliTheme`; do not invent a parallel theme system.
- Avoid speculative abstractions unless at least two output functions clearly share the same helper.
- Respect existing command behavior and data shape from `main.rs`.

### Verification Commands

At minimum, run:

```powershell
cargo test -p mangofetch-cli
```

If linting is run, interpret failures carefully:
- `cargo clippy -p mangofetch-cli -- -D warnings` currently surfaces pre-existing issues in `mangofetch-core`
- do not mix unrelated core cleanup into this UI pass unless explicitly asked

### Suggested Order Of Work

1. Redesign `format_info_card`
2. Redesign `format_queue_list`
3. Redesign `format_dependency_check`
4. Redesign `format_clean_summary` and `format_batch_summary`
5. Tighten tests
6. Run `cargo test -p mangofetch-cli`

### Definition Of Done

The remaining work is done when:

- static CLI outputs visibly match the live-progress redesign
- old mojibake-prone pseudo-box output is removed from the redesigned paths
- left-margin spacing is consistent across progress and static views
- `cargo test -p mangofetch-cli` passes
- no unrelated architecture churn is introduced
