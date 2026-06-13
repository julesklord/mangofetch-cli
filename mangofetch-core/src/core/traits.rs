use crate::models::queue::QueueItemInfo;
use std::sync::Arc;

/// Reporter trait for download progress and events.
/// Implementations can be Tauri-based (desktop app) or CLI-based (terminal output).
pub trait DownloadReporter: Send + Sync + 'static {
    /// Report download progress update
    fn on_progress(&self, download_id: u64, progress: crate::core::events::QueueItemProgress);

    /// Report download completion
    fn on_complete(
        &self,
        download_id: u64,
        file_path: Option<String>,
        file_size_bytes: Option<u64>,
    );

    /// Report download error
    fn on_error(&self, download_id: u64, error_message: String);

    /// Report retry attempt
    fn on_retry(&self, download_id: u64, attempt: u32, delay_ms: u64);

    /// Report phase change (e.g., "Fetching Info", "Downloading", "Merging")
    fn on_phase_change(&self, download_id: u64, phase: String);

    /// Report media preview info (for URL previews)
    fn on_media_preview(
        &self,
        url: String,
        title: String,
        author: String,
        thumbnail_url: Option<String>,
        duration_seconds: Option<f64>,
    );

    /// Report a full queue state update
    fn on_queue_update(&self, state: Vec<QueueItemInfo>);

    /// Report system-level progress (e.g., dependency updates)
    fn on_system_progress(&self, title: &str, percent: f32, message: &str);
}

pub type SharedReporter = Arc<dyn DownloadReporter>;
