use async_trait::async_trait;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use mangofetch_core::core::events::QueueItemProgress;
use mangofetch_core::core::traits::DownloadReporter;
use mangofetch_core::models::queue::QueueItemInfo;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

const ACTIVE_BAR_WIDTH: usize = 36;
const SYSTEM_DOWNLOAD_ID: u64 = u64::MAX;

use crate::formatting::{
    format_bytes, format_duration, truncate_text, MARGIN as ACTIVE_BLOCK_MARGIN,
};

pub trait CliTheme: Send + Sync {
    fn color_success(&self) -> String;
    fn color_error(&self) -> String;
    fn color_warning(&self) -> String;
    fn color_info(&self) -> String;
    fn color_platform(&self, platform: &str) -> String;
    fn color_accent(&self) -> String;
    fn color_reset(&self) -> String;
    fn progress_template(&self) -> String;
    fn format_phase(&self, download_id: u64, phase: &str) -> String;
    fn format_complete(
        &self,
        title: &str,
        file_path: Option<&str>,
        size_bytes: Option<u64>,
    ) -> String;
    fn format_error(&self, download_id: u64, error: &str) -> String;
    fn format_platform(&self, platform: &str) -> String;

    fn supports_unicode(&self) -> bool {
        true
    }
}

pub struct BrutalistTheme {
    supports_unicode: bool,
}

impl BrutalistTheme {
    pub fn new(supports_unicode: bool) -> Self {
        Self { supports_unicode }
    }

    fn unicode_or_ascii<'a>(&self, unicode: &'a str, ascii: &'a str) -> &'a str {
        if self.supports_unicode {
            unicode
        } else {
            ascii
        }
    }
}

pub struct PlainTheme {
    supports_unicode: bool,
}

impl PlainTheme {
    pub fn new(supports_unicode: bool) -> Self {
        Self { supports_unicode }
    }
}

impl CliTheme for PlainTheme {
    fn color_success(&self) -> String {
        "".to_string()
    }

    fn color_error(&self) -> String {
        "".to_string()
    }

    fn color_warning(&self) -> String {
        "".to_string()
    }

    fn color_info(&self) -> String {
        "".to_string()
    }

    fn color_accent(&self) -> String {
        "".to_string()
    }

    fn color_platform(&self, _platform: &str) -> String {
        "".to_string()
    }

    fn color_reset(&self) -> String {
        "".to_string()
    }

    fn progress_template(&self) -> String {
        "{msg}".to_string()
    }

    fn format_phase(&self, download_id: u64, phase: &str) -> String {
        format!("DL#{} -> {}", download_id, phase)
    }

    fn format_complete(
        &self,
        title: &str,
        _file_path: Option<&str>,
        _size_bytes: Option<u64>,
    ) -> String {
        format!("COMPLETE: {}", title)
    }

    fn format_error(&self, download_id: u64, error: &str) -> String {
        format!("ERROR: DL#{} {}", download_id, error)
    }

    fn format_platform(&self, platform: &str) -> String {
        platform.to_string()
    }

    fn supports_unicode(&self) -> bool {
        self.supports_unicode
    }
}

impl CliTheme for BrutalistTheme {
    fn color_success(&self) -> String {
        "\x1b[1;32m".to_string()
    }

    fn color_error(&self) -> String {
        "\x1b[1;31m".to_string()
    }

    fn color_warning(&self) -> String {
        "\x1b[1;33m".to_string()
    }

    fn color_info(&self) -> String {
        "\x1b[1;36m".to_string()
    }

    fn color_accent(&self) -> String {
        "\x1b[1;97m".to_string()
    }

    fn color_platform(&self, platform: &str) -> String {
        match platform.to_lowercase().as_str() {
            "youtube" => "\x1b[1;31m".to_string(),
            "instagram" => "\x1b[1;35m".to_string(),
            "tiktok" => "\x1b[1;36m".to_string(),
            "twitter" | "twitter/x" | "x" => "\x1b[1;34m".to_string(),
            "reddit" => "\x1b[1;33m".to_string(),
            "twitch" => "\x1b[1;35m".to_string(),
            "pinterest" => "\x1b[1;31m".to_string(),
            "vimeo" => "\x1b[1;36m".to_string(),
            "bluesky" => "\x1b[1;34m".to_string(),
            "bilibili" => "\x1b[1;36m".to_string(),
            _ => "\x1b[1;37m".to_string(),
        }
    }

    fn color_reset(&self) -> String {
        "\x1b[0m".to_string()
    }

    fn progress_template(&self) -> String {
        "{msg}".to_string()
    }

    fn format_phase(&self, download_id: u64, phase: &str) -> String {
        let icon = match normalize_phase_label(phase) {
            "FETCHING" => self.unicode_or_ascii(">>", ">>"),
            "DOWNLOADING" => self.unicode_or_ascii("=>", "=>"),
            "PROCESSING" => self.unicode_or_ascii("~>", "~>"),
            "FINALIZING" => self.unicode_or_ascii("::", "::"),
            "COMPLETE" => self.unicode_or_ascii("OK", "OK"),
            "ERROR" => self.unicode_or_ascii("!!", "!!"),
            _ => self.unicode_or_ascii("--", "--"),
        };

        format!(
            "{margin}{color}{icon}{reset} DL#{download_id:02} -> {phase_label}",
            margin = ACTIVE_BLOCK_MARGIN,
            color = self.color_info(),
            icon = icon,
            reset = self.color_reset(),
            phase_label = normalize_phase_label(phase),
        )
    }

    fn format_complete(
        &self,
        title: &str,
        file_path: Option<&str>,
        size_bytes: Option<u64>,
    ) -> String {
        let size = size_bytes
            .map(format_bytes)
            .unwrap_or_else(|| "--".to_string());

        match file_path {
            Some(path) => format!(
                "{margin}{ok} COMPLETE{reset} {title} [{size}] -> {path}",
                margin = ACTIVE_BLOCK_MARGIN,
                ok = self.color_success(),
                reset = self.color_reset(),
                title = truncate_text(title, 44),
                size = size,
                path = path,
            ),
            None => format!(
                "{margin}{ok} COMPLETE{reset} {title} [{size}]",
                margin = ACTIVE_BLOCK_MARGIN,
                ok = self.color_success(),
                reset = self.color_reset(),
                title = truncate_text(title, 44),
                size = size,
            ),
        }
    }

    fn format_error(&self, download_id: u64, error: &str) -> String {
        format!(
            "{margin}{err} ERROR{reset} DL#{download_id:02} {error}",
            margin = ACTIVE_BLOCK_MARGIN,
            err = self.color_error(),
            reset = self.color_reset(),
            error = truncate_text(error, 80),
        )
    }

    fn format_platform(&self, platform: &str) -> String {
        format!(
            "{}{}{}",
            self.color_platform(platform),
            platform,
            self.color_reset()
        )
    }

    fn supports_unicode(&self) -> bool {
        self.supports_unicode
    }
}

struct ProgressEntry {
    bar: ProgressBar,
    last_progress: Option<QueueItemProgress>,
    last_phase: Option<String>,
    label: Option<String>,
}

impl ProgressEntry {
    fn new(bar: ProgressBar) -> Self {
        Self {
            bar,
            last_progress: None,
            last_phase: None,
            label: None,
        }
    }
}

pub struct CLIReporter {
    multi_progress: MultiProgress,
    bars: Arc<Mutex<HashMap<u64, ProgressEntry>>>,
    theme: Arc<dyn CliTheme>,
}

impl CLIReporter {
    pub fn new() -> Self {
        Self::with_theme(Arc::new(BrutalistTheme::new(true)))
    }

    pub fn with_theme(theme: Arc<dyn CliTheme>) -> Self {
        Self {
            multi_progress: MultiProgress::new(),
            bars: Arc::new(Mutex::new(HashMap::new())),
            theme,
        }
    }

    pub fn _ascii_only() -> Self {
        Self::with_theme(Arc::new(BrutalistTheme::new(false)))
    }

    fn create_progress_bar(&self) -> ProgressBar {
        let pb = self.multi_progress.add(ProgressBar::new(100));
        let style = ProgressStyle::default_bar()
            .template(&self.theme.progress_template())
            .unwrap();
        pb.set_style(style);
        pb
    }
}

impl Default for CLIReporter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DownloadReporter for CLIReporter {
    fn on_progress(&self, download_id: u64, info: QueueItemProgress) {
        let mut bars = self.bars.lock().unwrap();
        let entry = bars
            .entry(download_id)
            .or_insert_with(|| ProgressEntry::new(self.create_progress_bar()));

        entry.last_phase = Some(normalize_phase_label(&info.phase).to_string());
        entry.last_progress = Some(info.clone());

        entry
            .bar
            .set_position(info.percent.clamp(0.0, 100.0) as u64);
        entry.bar.set_message(format_progress_block(
            self.theme.as_ref(),
            download_id,
            &info,
        ));
    }

    fn on_complete(
        &self,
        download_id: u64,
        file_path: Option<String>,
        file_size_bytes: Option<u64>,
    ) {
        let mut bars = self.bars.lock().unwrap();
        if let Some(entry) = bars.remove(&download_id) {
            let title = entry
                .last_progress
                .as_ref()
                .map(|info| info.title.as_str())
                .unwrap_or("Download");

            entry.bar.finish_with_message(self.theme.format_complete(
                title,
                file_path.as_deref(),
                file_size_bytes,
            ));
        }
    }

    fn on_error(&self, download_id: u64, error_message: String) {
        let mut bars = self.bars.lock().unwrap();
        let formatted = self.theme.format_error(download_id, &error_message);

        if let Some(entry) = bars.remove(&download_id) {
            entry.bar.abandon_with_message(formatted);
        } else {
            self.multi_progress.println(formatted).unwrap_or_default();
        }
    }

    fn on_retry(&self, download_id: u64, attempt: u32, delay_ms: u64) {
        let icon = if self.theme.supports_unicode() {
            "<<"
        } else {
            "R"
        };
        let message = format!(
            "{margin}{warn}{icon} RETRY{reset} DL#{download_id:02} attempt {attempt} in {delay_ms}ms",
            margin = ACTIVE_BLOCK_MARGIN,
            warn = self.theme.color_warning(),
            icon = icon,
            reset = self.theme.color_reset(),
            download_id = download_id,
            attempt = attempt,
            delay_ms = delay_ms,
        );

        self.multi_progress.println(message).unwrap_or_default();
    }

    fn on_phase_change(&self, download_id: u64, phase: String) {
        let normalized = normalize_phase_label(&phase).to_string();
        let transition = self.theme.format_phase(download_id, &phase);

        let mut bars = self.bars.lock().unwrap();
        if let Some(entry) = bars.get_mut(&download_id) {
            if entry.last_phase.as_deref() != Some(normalized.as_str()) {
                self.multi_progress.println(transition).unwrap_or_default();
            }

            entry.last_phase = Some(normalized);

            if let Some(info) = entry.last_progress.as_mut() {
                info.phase = phase;
                entry.bar.set_message(format_progress_block(
                    self.theme.as_ref(),
                    download_id,
                    info,
                ));
            }
        }
    }

    fn on_media_preview(
        &self,
        url: String,
        title: String,
        author: String,
        _thumbnail_url: Option<String>,
        duration_seconds: Option<f64>,
    ) {
        let platform = extract_platform(&url);
        let duration = duration_seconds
            .map(|seconds| format_duration(seconds as u64))
            .unwrap_or_else(|| "--".to_string());

        let icon = "[]";
        let message = format!(
            "{margin}{info}{icon} Found{reset} {title} by {author} [{platform}] [{duration}]",
            margin = ACTIVE_BLOCK_MARGIN,
            info = self.theme.color_info(),
            icon = icon,
            reset = self.theme.color_reset(),
            title = truncate_text(&title, 36),
            author = truncate_text(&author, 20),
            platform = platform,
            duration = duration,
        );

        self.multi_progress.println(message).unwrap_or_default();
    }

    fn on_queue_update(&self, _state: Vec<QueueItemInfo>) {}

    fn on_system_progress(&self, title: &str, percent: f32, message: &str) {
        let mut bars = self.bars.lock().unwrap();
        let entry = bars
            .entry(SYSTEM_DOWNLOAD_ID)
            .or_insert_with(|| ProgressEntry::new(self.create_progress_bar()));

        entry.label = Some(title.to_string());
        entry.bar.set_position(percent.clamp(0.0, 100.0) as u64);
        entry.bar.set_message(format_system_progress_block(
            self.theme.as_ref(),
            title,
            percent as f64,
            message,
        ));

        if percent >= 100.0 {
            entry.bar.finish_with_message(format!(
                "{margin}{ok} System ready{reset} {title}",
                margin = ACTIVE_BLOCK_MARGIN,
                ok = self.theme.color_success(),
                reset = self.theme.color_reset(),
                title = title,
            ));
            bars.remove(&SYSTEM_DOWNLOAD_ID);
        }
    }
}

pub fn normalize_phase_label(raw: &str) -> &'static str {
    let lower = raw.trim().to_lowercase();

    if lower.is_empty() {
        "Preparing"
    } else if lower.contains("error") || lower.contains("fail") {
        "Error"
    } else if lower.contains("complete") || lower.contains("done") {
        "Complete"
    } else if lower.contains("final") || lower.contains("rename") || lower.contains("move") {
        "Finalizing"
    } else if lower.contains("merge")
        || lower.contains("merg")
        || lower.contains("mux")
        || lower.contains("convert")
        || lower.contains("process")
    {
        "Processing"
    } else if lower.contains("download") {
        "Downloading"
    } else if lower.contains("fetch")
        || lower.contains("info")
        || lower.contains("resolve")
        || lower.contains("metadata")
    {
        "Fetching"
    } else {
        "Preparing"
    }
}

fn format_speed(speed_bytes_per_sec: f64) -> String {
    if speed_bytes_per_sec <= 0.0 {
        "--".to_string()
    } else if speed_bytes_per_sec < 1_048_576.0 {
        format!("{:.0} KB/s", speed_bytes_per_sec / 1024.0)
    } else {
        format!("{:.1} MB/s", speed_bytes_per_sec / 1_048_576.0)
    }
}

fn format_eta(info: &QueueItemProgress) -> String {
    if info.speed_bytes_per_sec <= 0.0 {
        return "--".to_string();
    }

    let Some(total) = info.total_bytes else {
        return "--".to_string();
    };

    let remaining = total.saturating_sub(info.downloaded_bytes);
    let eta_seconds = (remaining as f64 / info.speed_bytes_per_sec) as u64;
    format_duration(eta_seconds)
}

fn format_transfer(downloaded: u64, total: Option<u64>) -> String {
    match total {
        Some(total) => format!("{} / {}", format_bytes(downloaded), format_bytes(total)),
        None => format!("{} / --", format_bytes(downloaded)),
    }
}

fn phase_detail(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return default_phase_detail("Preparing").to_string();
    }

    let normalized = normalize_phase_label(trimmed);
    if trimmed.eq_ignore_ascii_case(normalized) {
        default_phase_detail(normalized).to_string()
    } else {
        trimmed.to_string()
    }
}

fn default_phase_detail(normalized: &str) -> &'static str {
    match normalized {
        "Fetching" => "Resolving media metadata",
        "Downloading" => "Streaming bytes from source",
        "Processing" => "Combining and polishing media",
        "Finalizing" => "Writing final output to disk",
        "Complete" => "Finished successfully",
        "Error" => "Stopped by an unrecoverable error",
        _ => "Preparing work queue",
    }
}

fn render_bar(percent: f64, width: usize, unicode: bool) -> String {
    let clamped = percent.clamp(0.0, 100.0);
    let filled = ((clamped / 100.0) * width as f64).floor() as usize;
    let is_full = filled >= width;

    if unicode {
        let full = "█";
        let head = "▓";
        let empty = "░";

        if is_full {
            full.repeat(width)
        } else if clamped > 0.0 && filled == 0 {
            format!("{}{}", head, empty.repeat(width - 1))
        } else if filled == 0 {
            empty.repeat(width)
        } else {
            format!(
                "{}{}{}",
                full.repeat(filled),
                head,
                empty.repeat(width.saturating_sub(filled + 1))
            )
        }
    } else {
        let full = "=";
        let head = ">";
        let empty = "-";

        if is_full {
            full.repeat(width)
        } else if clamped > 0.0 && filled == 0 {
            format!("{}{}", head, empty.repeat(width - 1))
        } else if filled == 0 {
            empty.repeat(width)
        } else {
            format!(
                "{}{}{}",
                full.repeat(filled),
                head,
                empty.repeat(width.saturating_sub(filled + 1))
            )
        }
    }
}

fn phase_color(theme: &dyn CliTheme, phase: &str) -> String {
    match phase {
        "Complete" => theme.color_success(),
        "Error" => theme.color_error(),
        "Fetching" | "Downloading" => theme.color_info(),
        "Processing" | "Finalizing" => theme.color_warning(),
        _ => theme.color_accent(),
    }
}

fn format_progress_block(
    theme: &dyn CliTheme,
    download_id: u64,
    info: &QueueItemProgress,
) -> String {
    let normalized = normalize_phase_label(&info.phase);
    let title = truncate_text(&info.title, 40);
    let phase_color = phase_color(theme, normalized);
    let percent = info.percent.clamp(0.0, 100.0).round() as u64;
    let bar = render_bar(info.percent, ACTIVE_BAR_WIDTH, theme.supports_unicode());
    let speed = format_speed(info.speed_bytes_per_sec);
    let eta = format_eta(info);
    let transfer = format_transfer(info.downloaded_bytes, info.total_bytes);
    let detail = phase_detail(&info.phase);
    let platform = theme.format_platform(&info.platform);

    format!(
        "{margin}DL#{download_id:02}  {title}  [{platform}]\n\
{margin}{phase_color}{phase}{reset}\n\
{margin}{info_color}{bar}{reset} {percent:>3}%\n\
{margin}{accent}{speed}{reset}   ETA {eta}   {transfer}\n\
{margin}{detail}\n",
        margin = ACTIVE_BLOCK_MARGIN,
        download_id = download_id,
        title = title,
        platform = platform,
        phase_color = phase_color,
        phase = normalized,
        reset = theme.color_reset(),
        info_color = theme.color_info(),
        bar = bar,
        percent = percent,
        accent = theme.color_accent(),
        speed = speed,
        eta = eta,
        transfer = transfer,
        detail = detail,
    )
}

fn format_system_progress_block(
    theme: &dyn CliTheme,
    title: &str,
    percent: f64,
    message: &str,
) -> String {
    let bar = render_bar(percent, ACTIVE_BAR_WIDTH, theme.supports_unicode());
    let clamped = percent.clamp(0.0, 100.0).round() as u64;

    format!(
        "{margin}SYSTEM  {title}\n\
{margin}{info}{bar}{reset} {percent:>3}%\n\
{margin}{message}\n",
        margin = ACTIVE_BLOCK_MARGIN,
        title = truncate_text(title, 48),
        info = theme.color_info(),
        bar = bar,
        reset = theme.color_reset(),
        percent = clamped,
        message = if message.trim().is_empty() {
            "Preparing dependencies"
        } else {
            message
        },
    )
}

fn extract_platform(url: &str) -> String {
    if url.contains("youtube.com") || url.contains("youtu.be") {
        "YouTube".to_string()
    } else if url.contains("instagram.com") {
        "Instagram".to_string()
    } else if url.contains("tiktok.com") {
        "TikTok".to_string()
    } else if url.contains("twitter.com") || url.contains("x.com") {
        "Twitter/X".to_string()
    } else if url.contains("reddit.com") {
        "Reddit".to_string()
    } else if url.contains("twitch.tv") {
        "Twitch".to_string()
    } else if url.contains("pinterest.com") {
        "Pinterest".to_string()
    } else if url.contains("vimeo.com") {
        "Vimeo".to_string()
    } else if url.contains("bluesky.app") {
        "Bluesky".to_string()
    } else if url.contains("bilibili.com") {
        "Bilibili".to_string()
    } else {
        "Generic".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1_048_576), "1.0 MB");
        assert_eq!(format_bytes(1_073_741_824), "1.0 GB");
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(30), "30s");
        assert_eq!(format_duration(90), "1m 30s");
        assert_eq!(format_duration(3661), "1h 1m");
    }

    #[test]
    fn test_extract_platform() {
        assert_eq!(
            extract_platform("https://youtube.com/watch?v=123"),
            "YouTube"
        );
        assert_eq!(
            extract_platform("https://www.tiktok.com/@user/video/123"),
            "TikTok"
        );
        assert_eq!(
            extract_platform("https://www.instagram.com/reel/123/"),
            "Instagram"
        );
    }

    #[test]
    fn test_brutalist_theme() {
        let theme = BrutalistTheme::new(true);
        assert!(theme.supports_unicode());
        assert!(!theme.color_success().is_empty());
        assert!(!theme.progress_template().is_empty());
    }

    #[test]
    fn test_brutalist_theme_base_colors() {
        let theme = BrutalistTheme::new(true);
        assert_eq!(theme.color_success(), "\x1b[1;32m");
        assert_eq!(theme.color_error(), "\x1b[1;31m");
        assert_eq!(theme.color_warning(), "\x1b[1;33m");
        assert_eq!(theme.color_info(), "\x1b[1;36m");
        assert_eq!(theme.color_accent(), "\x1b[1;97m");
        assert_eq!(theme.color_reset(), "\x1b[0m");
    }

    #[test]
    fn test_brutalist_theme_platform_colors() {
        let theme = BrutalistTheme::new(true);

        // Test supported platforms
        assert_eq!(theme.color_platform("youtube"), "\x1b[1;31m");
        assert_eq!(theme.color_platform("instagram"), "\x1b[1;35m");
        assert_eq!(theme.color_platform("tiktok"), "\x1b[1;36m");
        assert_eq!(theme.color_platform("twitter"), "\x1b[1;34m");
        assert_eq!(theme.color_platform("twitter/x"), "\x1b[1;34m");
        assert_eq!(theme.color_platform("x"), "\x1b[1;34m");
        assert_eq!(theme.color_platform("reddit"), "\x1b[1;33m");
        assert_eq!(theme.color_platform("twitch"), "\x1b[1;35m");
        assert_eq!(theme.color_platform("pinterest"), "\x1b[1;31m");
        assert_eq!(theme.color_platform("vimeo"), "\x1b[1;36m");
        assert_eq!(theme.color_platform("bluesky"), "\x1b[1;34m");
        assert_eq!(theme.color_platform("bilibili"), "\x1b[1;36m");

        // Test case insensitivity
        assert_eq!(theme.color_platform("YouTube"), "\x1b[1;31m");
        assert_eq!(theme.color_platform("INSTAGRAM"), "\x1b[1;35m");

        // Test default case
        assert_eq!(theme.color_platform("unknown"), "\x1b[1;37m");
    }

    #[test]
    fn normalizes_common_phase_labels() {
        assert_eq!(normalize_phase_label("Fetching info"), "Fetching");
        assert_eq!(normalize_phase_label("Downloading media"), "Downloading");
        assert_eq!(normalize_phase_label("Merging streams"), "Processing");
        assert_eq!(normalize_phase_label("Final rename"), "Finalizing");
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
        assert!(rendered.contains("Downloading"));
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
        assert!(rendered.contains("Preparing"));
        assert!(rendered.contains("ETA --"));
        assert!(rendered.contains("12 B / --"));
    }
}
