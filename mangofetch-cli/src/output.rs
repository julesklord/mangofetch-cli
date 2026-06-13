// output.rs
// ============================================================================
// Enhanced Output Formatting Module for MangoFetch CLI
// Place in: mangofetch-cli/src/output.rs
// ============================================================================

use crate::formatting::{format_bytes, format_duration, truncate_text, MARGIN};
use crate::reporter::CliTheme;
use std::sync::Arc;

/// Formats the `info` command output with a beautiful card-style layout
pub fn format_info_card(
    title: &str,
    author: &str,
    platform: &str,
    duration_seconds: Option<f64>,
    media_type: &str,
    theme: &Arc<dyn CliTheme>,
) -> String {
    let duration_str = duration_seconds
        .map(|d| format_duration(d as u64))
        .unwrap_or_else(|| "—".to_string());

    let platform_colored = theme.format_platform(platform);

    format!(
        "\n{margin}{info}Info{reset}  {accent}Media detail{reset}\n\
             {margin}{bar}\n\
             {margin}Title:     {title}\n\
             {margin}Author:    {author}\n\
             {margin}Platform:  {platform}\n\
             {margin}Type:      {media_type}\n\
             {margin}Duration:  {duration}\n",
        margin = MARGIN,
        info = theme.color_info(),
        accent = theme.color_accent(),
        reset = theme.color_reset(),
        bar = "—".repeat(50),
        title = truncate_text(title, 40),
        author = truncate_text(author, 40),
        platform = platform_colored,
        media_type = media_type,
        duration = duration_str,
    )
}

/// Formats the `list` command output with better visual hierarchy
pub fn format_queue_list(
    items: Vec<(u64, String, String, String, String)>, // (id, title, platform, status, progress)
    theme: &Arc<dyn CliTheme>,
) -> String {
    if items.is_empty() {
        return format!(
            "\n{margin}{info}Queue{reset}  No downloads found in history.\n",
            margin = MARGIN,
            info = theme.color_info(),
            reset = theme.color_reset()
        );
    }

    let mut output = format!(
        "\n{margin}{info}Queue{reset}  {accent}Status (Total: {}){reset}\n{margin}{bar}\n",
        items.len(),
        margin = MARGIN,
        info = theme.color_info(),
        accent = theme.color_accent(),
        reset = theme.color_reset(),
        bar = "—".repeat(50),
    );

    for (id, title, platform, status, progress) in items.iter() {
        let status_icon = match status.as_str() {
            s if s.contains("Active") => ">>",
            s if s.contains("Queued") => "○ ",
            s if s.contains("Complete") => "OK",
            s if s.contains("Error") => "!!",
            _ => "--",
        };

        let status_colored = match status.as_str() {
            s if s.contains("Active") => {
                format!("{}{}{}", theme.color_info(), status, theme.color_reset())
            }
            s if s.contains("Complete") => {
                format!("{}{}{}", theme.color_success(), status, theme.color_reset())
            }
            s if s.contains("Error") => {
                format!("{}{}{}", theme.color_error(), status, theme.color_reset())
            }
            _ => format!("{}{}{}", theme.color_warning(), status, theme.color_reset()),
        };

        let platform_colored = theme.format_platform(platform);

        output.push_str(&format!(
            "{margin}{icon} #{id:<2}  {title:<30}  {platform}  {status}\n",
            margin = MARGIN,
            icon = status_icon,
            id = id,
            title = truncate_text(title, 30),
            platform = platform_colored,
            status = status_colored,
        ));

        // Show progress bar inline for active downloads
        if status.contains("Active") && !progress.is_empty() {
            output.push_str(&format!(
                "{margin}       {progress}\n",
                margin = MARGIN,
                progress = progress
            ));
        }
    }

    output.push('\n');
    output
}

/// Formats the `config list` output with better structure
pub fn format_config_display(config_json: &str, theme: &Arc<dyn CliTheme>) -> String {
    let mut output = format!(
        "\n{margin}{info}Config{reset}  {accent}Application settings{reset}\n{margin}{bar}\n",
        margin = MARGIN,
        info = theme.color_info(),
        accent = theme.color_accent(),
        reset = theme.color_reset(),
        bar = "—".repeat(50),
    );

    if let Ok(value) = serde_json::from_str::<serde_json::Value>(config_json) {
        output.push_str(&format_json_pretty(&value, theme, 0));
    } else {
        output.push_str(config_json);
    }

    output
}

/// Helper to pretty-print JSON with colors
fn format_json_pretty(
    value: &serde_json::Value,
    theme: &Arc<dyn CliTheme>,
    indent: usize,
) -> String {
    let margin = MARGIN;
    let indent_str = "  ".repeat(indent);
    let next_indent = "  ".repeat(indent + 1);

    match value {
        serde_json::Value::Object(map) => {
            let mut result = String::from("{\n");
            for (i, (key, val)) in map.iter().enumerate() {
                result.push_str(&format!(
                    "{margin}{next_indent}{accent}{key}{reset}: {val}",
                    margin = margin,
                    next_indent = next_indent,
                    accent = theme.color_accent(),
                    key = key,
                    reset = theme.color_reset(),
                    val = format_json_pretty(val, theme, indent + 1).trim_start()
                ));
                if i < map.len() - 1 {
                    result.push_str(",\n");
                } else {
                    result.push('\n');
                }
            }
            result.push_str(&format!(
                "{margin}{indent_str}}}\n",
                margin = margin,
                indent_str = indent_str
            ));
            result
        }
        serde_json::Value::Array(arr) => {
            let mut result = String::from("[\n");
            for (i, item) in arr.iter().enumerate() {
                let comma = if i < arr.len() - 1 { "," } else { "" };
                result.push_str(margin);
                result.push_str(&next_indent);
                result.push_str(format_json_pretty(item, theme, indent + 1).trim_start());
                if !comma.is_empty() {
                    result.push(',');
                }
                result.push('\n');
            }
            result.push_str(&format!(
                "{margin}{indent_str}]\n",
                margin = margin,
                indent_str = indent_str
            ));
            result
        }
        serde_json::Value::String(s) => {
            format!("{}\"{}\"{}", theme.color_info(), s, theme.color_reset())
        }
        serde_json::Value::Number(n) => {
            format!("{}{}{}", theme.color_accent(), n, theme.color_reset())
        }
        serde_json::Value::Bool(b) => {
            let color = if *b {
                theme.color_success()
            } else {
                theme.color_error()
            };
            format!("{}{}{}", color, b, theme.color_reset())
        }
        serde_json::Value::Null => {
            format!("{}null{}", theme.color_warning(), theme.color_reset())
        }
    }
}

/// Formats the `check` command output
pub fn format_dependency_check(
    yt_dlp: Option<&str>,
    ffmpeg: Option<&str>,
    theme: &Arc<dyn CliTheme>,
) -> String {
    let mut output = format!(
        "\n{margin}{info}Check{reset}  {accent}System readiness{reset}\n{margin}{bar}\n",
        margin = MARGIN,
        info = theme.color_info(),
        accent = theme.color_accent(),
        reset = theme.color_reset(),
        bar = "—".repeat(50),
    );

    let check_yt_dlp = match yt_dlp {
        Some(path) => format!("{margin}OK  yt-dlp: {path}", margin = MARGIN, path = path),
        None => format!(
            "{margin}{err}!!  yt-dlp: Not found{reset} (will auto-install)",
            margin = MARGIN,
            err = theme.color_error(),
            reset = theme.color_reset()
        ),
    };

    let check_ffmpeg = match ffmpeg {
        Some(path) => format!("{margin}OK  FFmpeg: {path}", margin = MARGIN, path = path),
        None => format!(
            "{margin}{err}!!  FFmpeg: Not found{reset} (will auto-install)",
            margin = MARGIN,
            err = theme.color_error(),
            reset = theme.color_reset()
        ),
    };

    output.push_str(&check_yt_dlp);
    output.push('\n');
    output.push_str(&check_ffmpeg);
    output.push_str("\n\n");

    output
}

/// Formats the `clean` command confirmation + summary
pub fn format_clean_summary(
    total_removed: usize,
    bytes_freed: Option<u64>,
    theme: &Arc<dyn CliTheme>,
) -> String {
    let size_str = bytes_freed
        .map(format_bytes)
        .unwrap_or_else(|| "0 B".to_string());

    format!(
        "\n{margin}{ok}Clean{reset}  {count} item{s} removed. Freed {size} of space.\n",
        margin = MARGIN,
        ok = theme.color_success(),
        reset = theme.color_reset(),
        count = total_removed,
        s = if total_removed == 1 { "" } else { "s" },
        size = size_str
    )
}

/// Formats batch download summary
pub fn format_batch_summary(
    total: usize,
    queued: usize,
    failed: usize,
    theme: &Arc<dyn CliTheme>,
) -> String {
    let mut output = format!(
        "\n{margin}{info}Batch{reset}  {accent}Download summary{reset}\n{margin}{bar}\n",
        margin = MARGIN,
        info = theme.color_info(),
        accent = theme.color_accent(),
        reset = theme.color_reset(),
        bar = "—".repeat(50),
    );

    output.push_str(&format!(
        "{margin}Total:   {total}\n\
         {margin}Queued:  {queued}\n",
        margin = MARGIN,
        total = total,
        queued = queued,
    ));

    if failed > 0 {
        output.push_str(&format!(
            "{margin}{err}Failed:  {failed}{reset}\n",
            margin = MARGIN,
            err = theme.color_error(),
            failed = failed,
            reset = theme.color_reset()
        ));
    } else {
        output.push_str(&format!("{margin}Failures: None\n", margin = MARGIN));
    }

    output.push('\n');
    output
}

pub fn format_about_info(
    version: &str,
    author: &str,
    repo: &str,
    theme: &Arc<dyn CliTheme>,
) -> String {
    format!(
        "\n{margin}{info}About{reset}  {accent}Application info{reset}\n{margin}{bar}\n\
         {margin}Version:    {version}\n\
         {margin}Author:     {author}\n\
         {margin}Repository: {repo}\n\
         {margin}License:    GPL-3.0\n\
         {margin}Edition:    2021\n",
        margin = MARGIN,
        info = theme.color_info(),
        accent = theme.color_accent(),
        reset = theme.color_reset(),
        bar = "—".repeat(50),
        version = version,
        author = author,
        repo = repo,
    )
}

pub fn format_about_roadmap(theme: &Arc<dyn CliTheme>) -> String {
    format!(
        "\n{margin}{info}Roadmap{reset}  {accent}Future plans{reset}\n{margin}{bar}\n\
         {margin}v0.6.0 - Plugin ecosystem & Community sites\n\
         {margin}v0.7.0 - Native GUI Umbrella (Tauri v2)\n\
         {margin}v1.0.0 - Distributed swarm downloads\n",
        margin = MARGIN,
        info = theme.color_info(),
        accent = theme.color_accent(),
        reset = theme.color_reset(),
        bar = "—".repeat(50),
    )
}

pub fn format_about_changelog(theme: &Arc<dyn CliTheme>) -> String {
    format!(
        "\n{margin}{info}Changes{reset}  {accent}Project history{reset}\n{margin}{bar}\n\
         {margin}v0.5.3 - TUI Fix: Resolved ANSI artifacts in About tab\n\
         {margin}v0.5.2 - Maintenance: Relaxed tone & security cleanup\n\
         {margin}v0.5.1 - UX & Polish: Tropical themes & mouse support\n\
         {margin}v0.5.0 - P2P file sharing & Distributed downloads\n\
         {margin}v0.4.0 - Interactive TUI implementation & Stability release\n\
         {margin}v0.3.1 - Rebranding cleanup and test fixes\n\
         {margin}v0.3.0 - The mango is growing: Brutalist UI redesign & cleanup\n\
         {margin}v0.2.0 - Standalone rewrite: GUI removed, core refactored\n\
         {margin}v0.1.1 - Fixed build architecture\n\
         {margin}v0.1.0 - Initial release\n",
        margin = MARGIN,
        info = theme.color_info(),
        accent = theme.color_accent(),
        reset = theme.color_reset(),
        bar = "—".repeat(50),
    )
}

pub fn format_about_terms(theme: &Arc<dyn CliTheme>) -> String {
    format!(
        "\n{margin}{info}Terms{reset}  {accent}Legal & usage{reset}\n{margin}{bar}\n\
         {margin}License:    GPL-3.0\n\
         ",
        margin = MARGIN,
        info = theme.color_info(),
        accent = theme.color_accent(),
        reset = theme.color_reset(),
        bar = "—".repeat(50),
    )
}

// format_bytes and format_duration are now imported from formatting.rs

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reporter::BrutalistTheme;

    fn get_theme() -> Arc<dyn CliTheme> {
        Arc::new(BrutalistTheme::new(true))
    }

    #[test]
    fn test_format_info_card() {
        let theme = get_theme();
        let output = format_info_card(
            "Test Title",
            "Test Author",
            "YouTube",
            Some(125.0),
            "Video",
            &theme,
        );

        assert!(output.contains("Info"));
        assert!(output.contains("Media detail"));
        assert!(output.contains("Title:     Test Title"));
        assert!(output.contains("Author:    Test Author"));
        assert!(output.contains("Platform:"));
        assert!(output.contains("YouTube"));
        assert!(output.contains("Duration:  2m 5s"));
        assert!(output.contains(MARGIN));
    }

    #[test]
    fn test_format_queue_list_empty() {
        let theme = get_theme();
        let output = format_queue_list(vec![], &theme);

        assert!(output.contains("Queue"));
        assert!(output.contains("No downloads found"));
        assert!(output.contains(MARGIN));
    }

    #[test]
    fn test_format_queue_list_items() {
        let theme = get_theme();
        let items = vec![
            (
                1,
                "Title 1".to_string(),
                "YouTube".to_string(),
                "Active".to_string(),
                "==> 50%".to_string(),
            ),
            (
                2,
                "Title 2".to_string(),
                "TikTok".to_string(),
                "Queued".to_string(),
                "".to_string(),
            ),
        ];
        let output = format_queue_list(items, &theme);

        assert!(output.contains("Queue"));
        assert!(output.contains("Status (Total: 2)"));
        assert!(output.contains("#1"));
        assert!(output.contains("Title 1"));
        assert!(output.contains("Active"));
        assert!(output.contains("==> 50%"));
        assert!(output.contains("#2"));
        assert!(output.contains("Queued"));
        assert!(output.contains(MARGIN));
    }

    #[test]
    fn test_format_dependency_check() {
        let theme = get_theme();

        // Success case
        let output_ok = format_dependency_check(Some("/path/ytdlp"), Some("/path/ffmpeg"), &theme);
        assert!(output_ok.contains("Check"));
        assert!(output_ok.contains("System readiness"));
        assert!(output_ok.contains("OK  yt-dlp"));
        assert!(output_ok.contains("OK  FFmpeg"));

        // Missing case
        let output_missing = format_dependency_check(None, None, &theme);
        assert!(output_missing.contains("Not found"));
        assert!(output_missing.contains("will auto-install"));
    }

    #[test]
    fn test_format_clean_summary() {
        let theme = get_theme();

        let output_single = format_clean_summary(1, Some(1024), &theme);
        assert!(output_single.contains("Clean"));
        assert!(output_single.contains("1 item removed"));
        assert!(output_single.contains("1.0 KB"));

        let output_plural = format_clean_summary(5, Some(1024 * 1024), &theme);
        assert!(output_plural.contains("5 items removed"));
        assert!(output_plural.contains("1.0 MB"));
    }

    #[test]
    fn test_format_batch_summary() {
        let theme = get_theme();

        // Success only
        let output_ok = format_batch_summary(10, 10, 0, &theme);
        assert!(output_ok.contains("Batch"));
        assert!(output_ok.contains("Download summary"));
        assert!(output_ok.contains("Total:   10"));
        assert!(output_ok.contains("Queued:  10"));
        assert!(output_ok.contains("Failures: None"));

        // With failures
        let output_fail = format_batch_summary(10, 7, 3, &theme);
        assert!(output_fail.contains("Failed:  3"));
    }
}
