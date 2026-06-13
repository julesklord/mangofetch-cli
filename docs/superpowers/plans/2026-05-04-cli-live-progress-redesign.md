# CLI Live Progress Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Redesign MangoFetch CLI live download progress so active downloads render as expressive multi-line blocks with stronger phase hierarchy, wider bars, spacing, and Unicode/ASCII variants.

**Architecture:** Keep `indicatif::MultiProgress` as the rendering engine, but move message assembly into explicit formatting helpers in `mangofetch-cli/src/reporter.rs`. Normalize raw phases into stable visual states, generate card-like block strings for active items and compact terminal events for transitions, completion, and errors, then verify with focused tests.

**Tech Stack:** Rust, `indicatif`, Cargo test

---

### Task 1: Add formatting-focused reporter tests

**Files:**
- Modify: `mangofetch-cli/src/reporter.rs`

- [ ] **Step 1: Write the failing tests**

Add reporter unit tests covering:

```rust
#[test]
fn normalizes_common_phase_labels() {
    assert_eq!(normalize_phase_label("Fetching info"), "FETCHING");
    assert_eq!(normalize_phase_label("Downloading media"), "DOWNLOADING");
    assert_eq!(normalize_phase_label("Merging streams"), "PROCESSING");
    assert_eq!(normalize_phase_label("Final rename"), "FINALIZING");
}

#[test]
fn renders_unicode_progress_block_with_margin_and_metrics() {
    let theme = BrutalistTheme::new(true);
    let info = QueueItemProgress {
        id: 7,
        title: "A very long CLI redesign walkthrough".to_string(),
        platform: "YouTube".to_string(),
        percent: 62.4,
        speed_bytes_per_sec: 3_145_728.0,
        downloaded_bytes: 148 * 1024 * 1024,
        total_bytes: Some(238 * 1024 * 1024),
        phase: "Downloading media".to_string(),
    };

    let rendered = format_progress_block(&theme, 7, &info);

    assert!(rendered.contains("  DL#07"));
    assert!(rendered.contains("DOWNLOADING"));
    assert!(rendered.contains("ETA"));
    assert!(rendered.contains("148.0 MB / 238.0 MB"));
}

#[test]
fn renders_ascii_progress_block_with_same_structure() {
    let theme = BrutalistTheme::new(false);
    let info = QueueItemProgress {
        id: 5,
        title: "ASCII fallback".to_string(),
        platform: "Generic".to_string(),
        percent: 12.0,
        speed_bytes_per_sec: 0.0,
        downloaded_bytes: 12,
        total_bytes: None,
        phase: "Preparing".to_string(),
    };

    let rendered = format_progress_block(&theme, 5, &info);

    assert!(rendered.contains("  DL#05"));
    assert!(rendered.contains("PREPARING"));
    assert!(rendered.contains("ETA --"));
    assert!(rendered.contains("12 B / --"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p mangofetch-cli reporter::tests -- --nocapture`
Expected: FAIL because the new formatting helpers do not exist yet.

### Task 2: Implement the new formatting layer in the reporter

**Files:**
- Modify: `mangofetch-cli/src/reporter.rs`

- [ ] **Step 1: Add helper functions and small state structures**

Implement:

```rust
fn normalize_phase_label(raw: &str) -> &'static str { /* phase mapping */ }
fn truncate_title(title: &str, max_chars: usize) -> String { /* smart truncation */ }
fn format_transfer(downloaded: u64, total: Option<u64>) -> String { /* bytes summary */ }
fn format_eta(info: &QueueItemProgress) -> String { /* ETA or -- */ }
fn format_speed(speed_bytes_per_sec: f64) -> String { /* MB/s or -- */ }
fn render_bar(percent: f64, width: usize, unicode: bool) -> String { /* unicode/ascii bar */ }
fn format_progress_block(theme: &BrutalistTheme, download_id: u64, info: &QueueItemProgress) -> String { /* multi-line block */ }
```

- [ ] **Step 2: Update theme formatting for events**

Keep completion/error/phase events compact and aligned with the new visual style, including:

```rust
fn format_phase_transition(&self, download_id: u64, phase: &str) -> String;
fn format_complete(&self, title: &str, file_path: Option<&str>, size_bytes: Option<u64>) -> String;
fn format_error(&self, download_id: u64, error: &str) -> String;
```

- [ ] **Step 3: Wire `CLIReporter` to use the new block renderer**

Adjust `create_progress_bar`, `on_progress`, `on_phase_change`, `on_complete`, `on_error`, and `on_system_progress` so the progress bar message is the multi-line block, phase changes print compact transitions, and finished items collapse to compact summaries.

- [ ] **Step 4: Run tests to verify the implementation passes**

Run: `cargo test -p mangofetch-cli reporter::tests -- --nocapture`
Expected: PASS

### Task 3: Run broader CLI verification

**Files:**
- Modify: `mangofetch-cli/src/reporter.rs`
- Review: `mangofetch-cli/tests/integration_tests.rs`

- [ ] **Step 1: Run package tests**

Run: `cargo test -p mangofetch-cli`
Expected: PASS

- [ ] **Step 2: Run lints if the code compiles cleanly**

Run: `cargo clippy -p mangofetch-cli -- -D warnings`
Expected: PASS, or address any new warnings introduced by the redesign.

- [ ] **Step 3: Commit**

```bash
git add mangofetch-cli/src/reporter.rs docs/superpowers/plans/2026-05-04-cli-live-progress-redesign.md
git commit -m "feat: redesign CLI live progress rendering"
```
