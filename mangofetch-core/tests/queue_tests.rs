use mangofetch_core::core::manager::queue::DownloadQueue;
use mangofetch_core::core::traits::DownloadReporter;
use mangofetch_core::models::queue::{QueueItemInfo, QueueStatus};
use std::sync::Arc;

struct MockReporter;
impl DownloadReporter for MockReporter {
    fn on_progress(&self, _id: u64, _progress: mangofetch_core::core::events::QueueItemProgress) {}
    fn on_complete(&self, _id: u64, _file_path: Option<String>, _file_size_bytes: Option<u64>) {}
    fn on_error(&self, _id: u64, _error_message: String) {}
    fn on_retry(&self, _id: u64, _attempt: u32, _delay_ms: u64) {}
    fn on_phase_change(&self, _id: u64, _phase: String) {}
    fn on_media_preview(
        &self,
        _url: String,
        _title: String,
        _author: String,
        _thumbnail_url: Option<String>,
        _duration_seconds: Option<f64>,
    ) {
    }
    fn on_queue_update(&self, _state: Vec<QueueItemInfo>) {}
    fn on_system_progress(&self, _title: &str, _percent: f32, _message: &str) {}
}

#[test]
fn test_queue_lifecycle() {
    let reporter = Arc::new(MockReporter);
    let mut queue = DownloadQueue::new(2, Some(reporter));

    // Mock downloader
    struct MockDownloader;
    #[async_trait::async_trait]
    impl mangofetch_core::platforms::traits::PlatformDownloader for MockDownloader {
        fn name(&self) -> &str {
            "mock"
        }
        fn can_handle(&self, _url: &str) -> bool {
            true
        }
        async fn get_media_info(
            &self,
            _url: &str,
        ) -> anyhow::Result<mangofetch_core::models::media::MediaInfo> {
            Ok(mangofetch_core::models::media::MediaInfo {
                title: "mock".into(),
                author: "mock".into(),
                platform: "mock".into(),
                duration_seconds: None,
                thumbnail_url: None,
                available_qualities: vec![],
                media_type: mangofetch_core::models::media::MediaType::Video,
                file_size_bytes: None,
            })
        }
        async fn download(
            &self,
            _info: &mangofetch_core::models::media::MediaInfo,
            _opts: &mangofetch_core::models::media::DownloadOptions,
            _tx: tokio::sync::mpsc::Sender<f64>,
        ) -> anyhow::Result<mangofetch_core::models::media::DownloadResult> {
            Ok(mangofetch_core::models::media::DownloadResult {
                file_path: std::path::PathBuf::from("test"),
                file_size_bytes: 0,
                duration_seconds: 0.0,
                torrent_id: None,
            })
        }
    }
    let downloader = Arc::new(MockDownloader);

    // 1. Enqueue
    queue.enqueue(
        1,
        "url1".into(),
        "mock".into(),
        "title1".into(),
        "dir".into(),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        downloader.clone(),
        None,
        false,
    );
    assert_eq!(queue.items.len(), 1);
    assert_eq!(queue.items[0].status, QueueStatus::Queued);

    // 2. Mark active
    queue.mark_active(1);
    assert_eq!(queue.items[0].status, QueueStatus::Active);

    // 3. Mark complete
    queue.mark_complete(1, true, None, Some("path".into()), Some(100));
    assert_eq!(
        queue.items[0].status,
        QueueStatus::Complete { success: true }
    );

    // 4. Clear finished
    queue.clear_finished();
    assert_eq!(queue.items.len(), 0);
}
