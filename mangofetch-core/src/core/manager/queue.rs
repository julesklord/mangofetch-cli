use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};

static EMIT_COUNT: AtomicU64 = AtomicU64::new(0);

use serde::Serialize;
use tokio_util::sync::CancellationToken;

use crate::core::traits::SharedReporter;
use crate::models::media::MediaInfo;
use crate::models::queue::{QueueItemInfo, QueueStatus};
use crate::platforms::traits::PlatformDownloader;

fn shared_http_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        crate::core::http_client::apply_global_proxy(reqwest::Client::builder())
            .build()
            .unwrap_or_default()
    })
}

struct CachedInfo {
    info: MediaInfo,
    cached_at: std::time::Instant,
}

static INFO_CACHE: OnceLock<tokio::sync::Mutex<HashMap<String, CachedInfo>>> = OnceLock::new();

fn info_cache() -> &'static tokio::sync::Mutex<HashMap<String, CachedInfo>> {
    INFO_CACHE.get_or_init(|| tokio::sync::Mutex::new(HashMap::new()))
}

const INFO_CACHE_TTL: std::time::Duration = std::time::Duration::from_secs(600);

static IN_FLIGHT_FETCHES: OnceLock<
    tokio::sync::Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>,
> = OnceLock::new();

fn in_flight_map() -> &'static tokio::sync::Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>> {
    IN_FLIGHT_FETCHES.get_or_init(|| tokio::sync::Mutex::new(HashMap::new()))
}

#[derive(Debug, Clone, Serialize)]
pub struct MediaPreviewEvent {
    pub url: String,
    pub title: String,
    pub author: String,
    pub thumbnail_url: Option<String>,
    pub duration_seconds: Option<f64>,
}

pub struct QueueItem {
    pub id: u64,
    pub url: String,
    pub platform: String,
    pub title: String,
    pub status: QueueStatus,
    pub cancel_token: CancellationToken,
    pub output_dir: String,
    pub download_mode: Option<String>,
    pub quality: Option<String>,
    pub video_format: Option<String>,
    pub audio_format: Option<String>,
    pub audio_quality: Option<String>,
    pub format_id: Option<String>,
    pub referer: Option<String>,
    pub extra_headers: Option<std::collections::HashMap<String, String>>,
    pub page_url: Option<String>,
    pub user_agent: Option<String>,
    pub percent: f64,
    pub speed_bytes_per_sec: f64,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub file_path: Option<String>,
    pub file_size_bytes: Option<u64>,
    pub file_count: Option<u32>,
    pub media_info: Option<MediaInfo>,
    pub downloader: Arc<dyn PlatformDownloader>,
    pub ytdlp_path: Option<PathBuf>,
    pub from_hotkey: bool,
    pub torrent_id: Option<usize>,
    pub download_subtitles: Option<bool>,
    pub phase: String,
}

impl QueueItem {
    pub fn to_info(&self) -> QueueItemInfo {
        QueueItemInfo {
            id: self.id,
            url: self.url.clone(),
            platform: self.platform.clone(),
            title: self.title.clone(),
            status: self.status.clone(),
            percent: self.percent,
            speed_bytes_per_sec: self.speed_bytes_per_sec,
            downloaded_bytes: self.downloaded_bytes,
            total_bytes: self.total_bytes,
            phase: self.phase.clone(),
            file_path: self.file_path.clone(),
            file_size_bytes: self.file_size_bytes,
            file_count: self.file_count,
            thumbnail_url: self
                .media_info
                .as_ref()
                .and_then(|m| m.thumbnail_url.clone()),
        }
    }
}

pub struct DownloadQueue {
    pub items: Vec<QueueItem>,
    pub max_concurrent: u32,
    pub stagger_delay_ms: u64,
    pub reporter: Option<SharedReporter>,
}

impl DownloadQueue {
    pub fn new(max_concurrent: u32, reporter: Option<SharedReporter>) -> Self {
        Self {
            items: Vec::new(),
            max_concurrent,
            stagger_delay_ms: 150,
            reporter,
        }
    }

    pub fn set_reporter(&mut self, reporter: SharedReporter) {
        self.reporter = Some(reporter);
    }

    pub fn load_from_recovery(&mut self, registry: &crate::core::registry::PlatformRegistry) {
        let recovery_items = crate::core::manager::recovery::list();
        for item in recovery_items {
            let downloader = registry
                .find_platform(&item.url)
                .or_else(|| registry.find_by_name(&item.platform));

            if let Some(dl) = downloader {
                let percent = if matches!(item.status, QueueStatus::Complete { success: true }) {
                    100.0
                } else {
                    0.0
                };

                let status = match item.status {
                    QueueStatus::Active | QueueStatus::Seeding => QueueStatus::Paused,
                    other => other,
                };

                let q_item = QueueItem {
                    id: item.id,
                    url: item.url,
                    platform: item.platform,
                    title: item.title,
                    status,
                    cancel_token: CancellationToken::new(),
                    output_dir: item.output_dir,
                    download_mode: item.download_mode,
                    quality: item.quality,
                    video_format: None,
                    audio_format: None,
                    audio_quality: None,
                    format_id: item.format_id,
                    referer: item.referer,
                    extra_headers: None,
                    page_url: None,
                    user_agent: None,
                    percent,
                    speed_bytes_per_sec: 0.0,
                    downloaded_bytes: 0,
                    total_bytes: item.file_size_bytes,
                    file_path: item.file_path,
                    file_size_bytes: item.file_size_bytes,
                    file_count: None,
                    media_info: None,
                    downloader: dl,
                    ytdlp_path: None,
                    from_hotkey: false,
                    torrent_id: None,
                    download_subtitles: None,
                    phase: "Restored".to_string(),
                };
                self.items.push(q_item);
            }
        }
    }

    fn sync_recovery(item: &QueueItem) {
        crate::core::manager::recovery::persist(crate::core::manager::recovery::RecoveryItem {
            id: item.id,
            url: item.url.clone(),
            title: item.title.clone(),
            platform: item.platform.clone(),
            output_dir: item.output_dir.clone(),
            status: item.status.clone(),
            download_mode: item.download_mode.clone(),
            quality: item.quality.clone(),
            format_id: item.format_id.clone(),
            referer: item.referer.clone(),
            file_path: item.file_path.clone(),
            file_size_bytes: item.file_size_bytes,
        });
    }

    #[allow(clippy::too_many_arguments)]
    pub fn enqueue(
        &mut self,
        id: u64,
        url: String,
        platform: String,
        title: String,
        output_dir: String,
        download_mode: Option<String>,
        quality: Option<String>,
        video_format: Option<String>,
        audio_format: Option<String>,
        audio_quality: Option<String>,
        download_subtitles: Option<bool>,
        format_id: Option<String>,
        referer: Option<String>,
        extra_headers: Option<std::collections::HashMap<String, String>>,
        page_url: Option<String>,
        user_agent: Option<String>,
        media_info: Option<MediaInfo>,
        total_bytes: Option<u64>,
        file_count: Option<u32>,
        downloader: Arc<dyn PlatformDownloader>,
        ytdlp_path: Option<PathBuf>,
        from_hotkey: bool,
    ) {
        let item = QueueItem {
            id,
            url,
            platform,
            title,
            status: QueueStatus::Queued,
            cancel_token: CancellationToken::new(),
            output_dir,
            download_mode,
            quality,
            video_format,
            audio_format,
            audio_quality,
            format_id,
            referer,
            extra_headers,
            page_url,
            user_agent,
            percent: 0.0,
            speed_bytes_per_sec: 0.0,
            downloaded_bytes: 0,
            total_bytes,
            file_path: None,
            file_size_bytes: None,
            file_count,
            media_info,
            downloader,
            ytdlp_path,
            from_hotkey,
            torrent_id: None,
            download_subtitles,
            phase: "Queued".to_string(),
        };
        self.items.push(item);
        Self::sync_recovery(self.items.last().unwrap());
    }

    pub fn active_count(&self) -> u32 {
        self.items
            .iter()
            .filter(|i| i.status == QueueStatus::Active)
            .count() as u32
    }

    pub fn next_queued_ids(&self) -> Vec<u64> {
        let slots = self.max_concurrent.saturating_sub(self.active_count()) as usize;
        self.items
            .iter()
            .filter(|i| i.status == QueueStatus::Queued)
            .take(slots)
            .map(|i| i.id)
            .collect()
    }

    pub fn mark_active(&mut self, id: u64) {
        let item = self.items.iter_mut().find(|i| i.id == id);
        if let Some(item) = item {
            item.status = QueueStatus::Active;
            item.cancel_token = CancellationToken::new();
            Self::sync_recovery(item);
        }
    }

    pub fn mark_complete(
        &mut self,
        id: u64,
        success: bool,
        error: Option<String>,
        file_path: Option<String>,
        file_size_bytes: Option<u64>,
    ) {
        let item = self.items.iter_mut().find(|i| i.id == id);
        if let Some(item) = item {
            if success {
                item.status = QueueStatus::Complete { success: true };
                item.percent = 100.0;
            } else {
                item.status = QueueStatus::Error {
                    message: error.unwrap_or_default(),
                };
            }
            item.file_path = file_path;
            item.file_size_bytes = file_size_bytes;
            item.speed_bytes_per_sec = 0.0;
            Self::sync_recovery(item);
        }
    }

    pub fn mark_seeding(
        &mut self,
        id: u64,
        file_path: Option<String>,
        file_size_bytes: Option<u64>,
        torrent_id: Option<usize>,
    ) {
        let item = self.items.iter_mut().find(|i| i.id == id);
        if let Some(item) = item {
            item.status = QueueStatus::Seeding;
            item.percent = 100.0;
            item.file_path = file_path;
            item.file_size_bytes = file_size_bytes;
            item.speed_bytes_per_sec = 0.0;
            item.torrent_id = torrent_id;
            Self::sync_recovery(item);
        }
    }

    pub fn update_progress(
        &mut self,
        id: u64,
        percent: f64,
        speed: f64,
        downloaded: u64,
        total: Option<u64>,
        torrent_id: Option<usize>,
    ) {
        if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
            item.percent = percent;
            item.speed_bytes_per_sec = speed;
            item.downloaded_bytes = downloaded;
            if let Some(t) = total {
                item.total_bytes = Some(t);
            }
            if torrent_id.is_some() && item.torrent_id.is_none() {
                item.torrent_id = torrent_id;
            }
        }
    }

    pub fn pause(&mut self, id: u64) -> bool {
        if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
            if item.status == QueueStatus::Active {
                if item.platform != "magnet" {
                    item.cancel_token.cancel();
                }
                item.status = QueueStatus::Paused;
                item.speed_bytes_per_sec = 0.0;
                Self::sync_recovery(item);
                return true;
            }
        }
        false
    }

    pub fn resume(&mut self, id: u64) -> bool {
        let item = self.items.iter_mut().find(|i| i.id == id);
        if let Some(item) = item {
            if item.status == QueueStatus::Paused {
                if item.platform == "magnet" {
                    item.status = QueueStatus::Active;
                } else {
                    item.status = QueueStatus::Queued;
                    item.cancel_token = CancellationToken::new();
                }
                Self::sync_recovery(item);
                return true;
            }
        }
        false
    }

    pub fn cancel(&mut self, id: u64) -> (bool, Option<usize>) {
        let result = self.cancel_inner(id);
        if result.0 {
            crate::core::manager::recovery::remove(id);
        }
        result
    }

    fn cancel_inner(&mut self, id: u64) -> (bool, Option<usize>) {
        if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
            match &item.status {
                QueueStatus::Active => {
                    item.cancel_token.cancel();
                    item.status = QueueStatus::Error {
                        message: "Cancelled".to_string(),
                    };
                    item.speed_bytes_per_sec = 0.0;
                    Self::sync_recovery(item);
                    return (true, None);
                }
                QueueStatus::Seeding => {
                    let tid = item.torrent_id;
                    item.status = QueueStatus::Error {
                        message: "Cancelled".to_string(),
                    };
                    item.speed_bytes_per_sec = 0.0;
                    Self::sync_recovery(item);
                    return (true, tid);
                }
                QueueStatus::Paused => {
                    item.cancel_token.cancel();
                    let tid = if item.platform == "magnet" {
                        item.torrent_id
                    } else {
                        None
                    };
                    item.status = QueueStatus::Error {
                        message: "Cancelled".to_string(),
                    };
                    item.speed_bytes_per_sec = 0.0;
                    Self::sync_recovery(item);
                    return (true, tid);
                }
                QueueStatus::Queued => {
                    item.status = QueueStatus::Error {
                        message: "Cancelled".to_string(),
                    };
                    Self::sync_recovery(item);
                    return (true, None);
                }
                _ => {}
            }
        }
        (false, None)
    }

    pub fn retry(&mut self, id: u64) -> bool {
        if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
            if matches!(item.status, QueueStatus::Error { .. }) {
                item.status = QueueStatus::Queued;
                item.cancel_token = CancellationToken::new();
                item.percent = 0.0;
                item.speed_bytes_per_sec = 0.0;
                item.downloaded_bytes = 0;
                item.file_path = None;
                item.file_size_bytes = None;
                Self::sync_recovery(item);
                return true;
            }
        }
        false
    }

    pub fn remove(&mut self, id: u64) -> Option<Option<usize>> {
        let result = self.remove_inner(id);
        if result.is_some() {
            crate::core::manager::recovery::remove(id);
        }
        result
    }

    fn remove_inner(&mut self, id: u64) -> Option<Option<usize>> {
        if let Some(pos) = self.items.iter().position(|i| i.id == id) {
            let item = &self.items[pos];
            if item.status == QueueStatus::Active {
                item.cancel_token.cancel();
            }
            if item.status == QueueStatus::Paused && item.platform == "magnet" {
                item.cancel_token.cancel();
            }
            let torrent_id = if item.status == QueueStatus::Seeding
                || (item.status == QueueStatus::Paused && item.platform == "magnet")
            {
                item.torrent_id
            } else {
                None
            };
            self.items.remove(pos);
            return Some(torrent_id);
        }
        None
    }

    pub fn clear_finished(&mut self) {
        let to_remove: Vec<u64> = self
            .items
            .iter()
            .filter(|i| {
                matches!(
                    i.status,
                    QueueStatus::Complete { .. } | QueueStatus::Error { .. }
                )
            })
            .map(|i| i.id)
            .collect();
        for id in &to_remove {
            crate::core::manager::recovery::remove(*id);
        }
        self.items.retain(|i| {
            !matches!(
                i.status,
                QueueStatus::Complete { .. } | QueueStatus::Error { .. }
            )
        });
    }

    pub fn get_state(&self) -> Vec<QueueItemInfo> {
        self.items.iter().map(|i| i.to_info()).collect()
    }

    pub fn has_url(&self, url: &str) -> bool {
        self.items.iter().any(|i| {
            i.url == url
                && matches!(
                    i.status,
                    QueueStatus::Queued
                        | QueueStatus::Active
                        | QueueStatus::Paused
                        | QueueStatus::Seeding
                )
        })
    }
}

pub struct ProgressThrottle {
    last_emit: std::time::Instant,
    min_interval: std::time::Duration,
}

impl ProgressThrottle {
    pub fn new(min_interval_ms: u64) -> Self {
        Self {
            last_emit: std::time::Instant::now() - std::time::Duration::from_secs(10),
            min_interval: std::time::Duration::from_millis(min_interval_ms),
        }
    }

    pub fn should_emit(&mut self) -> bool {
        let now = std::time::Instant::now();
        if now.duration_since(self.last_emit) >= self.min_interval {
            self.last_emit = now;
            true
        } else {
            false
        }
    }
}

pub fn emit_queue_state_from_state(reporter: &Option<SharedReporter>, state: Vec<QueueItemInfo>) {
    let n = EMIT_COUNT.fetch_add(1, Ordering::Relaxed);
    if n.is_multiple_of(10) {
        tracing::debug!("[perf] emit_queue_state called {} times", n);
    }
    if let Some(reporter) = reporter {
        reporter.on_queue_update(state);
    }
}

pub fn emit_queue_state(queue: &DownloadQueue) {
    let state = queue.get_state();
    emit_queue_state_from_state(&queue.reporter, state);
}

/// RAII guard that ensures an Active queue item never leaks a slot.
struct ActiveJobSlot {
    queue: Arc<tokio::sync::Mutex<DownloadQueue>>,
    item_id: u64,
    armed: bool,
}

impl ActiveJobSlot {
    fn new(queue: Arc<tokio::sync::Mutex<DownloadQueue>>, item_id: u64) -> Self {
        Self {
            queue,
            item_id,
            armed: true,
        }
    }

    fn disarm(mut self) {
        self.armed = false;
    }
}

impl Drop for ActiveJobSlot {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        let queue = self.queue.clone();
        let item_id = self.item_id;
        tokio::spawn(async move {
            let state = {
                let mut q = queue.lock().await;
                let still_active = q
                    .items
                    .iter()
                    .find(|i| i.id == item_id)
                    .map(|i| i.status == QueueStatus::Active)
                    .unwrap_or(false);
                if !still_active {
                    return;
                }
                tracing::warn!(
                    "[queue] ActiveJobSlot guard firing for {} — download ended without clean release",
                    item_id
                );
                q.mark_complete(
                    item_id,
                    false,
                    Some("Download interrupted".to_string()),
                    None,
                    None,
                );
                q.get_state()
            };
            let reporter = { queue.lock().await.reporter.clone() };
            emit_queue_state_from_state(&reporter, state);
            tokio::spawn(try_start_next(queue));
        });
    }
}

pub fn spawn_download(
    queue: Arc<tokio::sync::Mutex<DownloadQueue>>,
    item_id: u64,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
    Box::pin(async move {
        let _timer_start = std::time::Instant::now();
        let slot = ActiveJobSlot::new(queue.clone(), item_id);
        tokio::spawn(spawn_download_inner(queue.clone(), item_id));
        slot.disarm();
        tracing::debug!(
            "[perf] spawn_download {} took {:?}",
            item_id,
            _timer_start.elapsed()
        );
    })
}

struct DownloadContext {
    url: String,
    output_dir: String,
    download_mode: Option<String>,
    quality: Option<String>,
    video_format: Option<String>,
    audio_format: Option<String>,
    audio_quality: Option<String>,
    download_subtitles: Option<bool>,
    format_id: Option<String>,
    referer: Option<String>,
    extra_headers: Option<std::collections::HashMap<String, String>>,
    page_url: Option<String>,
    user_agent: Option<String>,
    cancel_token: tokio_util::sync::CancellationToken,
    media_info: Option<crate::models::media::MediaInfo>,
    platform_name: String,
    downloader: std::sync::Arc<dyn crate::platforms::traits::PlatformDownloader>,
    ytdlp_path: Option<std::path::PathBuf>,
    from_hotkey: bool,
}

async fn extract_download_context(
    queue: &Arc<tokio::sync::Mutex<DownloadQueue>>,
    item_id: u64,
) -> Option<DownloadContext> {
    let q = queue.lock().await;
    let item = q.items.iter().find(|i| i.id == item_id)?;
    Some(DownloadContext {
        url: item.url.clone(),
        output_dir: item.output_dir.clone(),
        download_mode: item.download_mode.clone(),
        quality: item.quality.clone(),
        video_format: item.video_format.clone(),
        audio_format: item.audio_format.clone(),
        audio_quality: item.audio_quality.clone(),
        download_subtitles: item.download_subtitles,
        format_id: item.format_id.clone(),
        referer: item.referer.clone(),
        extra_headers: item.extra_headers.clone(),
        page_url: item.page_url.clone(),
        user_agent: item.user_agent.clone(),
        cancel_token: item.cancel_token.clone(),
        media_info: item.media_info.clone(),
        platform_name: item.platform.clone(),
        downloader: item.downloader.clone(),
        ytdlp_path: item.ytdlp_path.clone(),
        from_hotkey: item.from_hotkey,
    })
}

async fn prepare_media_info(
    queue: &Arc<tokio::sync::Mutex<DownloadQueue>>,
    item_id: u64,
    ctx: &DownloadContext,
    reporter: &Option<crate::core::traits::SharedReporter>,
) -> Option<crate::models::media::MediaInfo> {
    let info_start = std::time::Instant::now();
    let info = match &ctx.media_info {
        Some(i) => {
            tracing::info!(
                "[queue] info for {} from cache/pre-fetched in {:?}",
                item_id,
                info_start.elapsed()
            );
            i.clone()
        }
        None => {
            tracing::debug!(
                "[perf] spawn_download_inner {}: media_info is None, fetching info",
                item_id
            );
            if let Some(r) = reporter {
                r.on_progress(
                    item_id,
                    crate::core::events::QueueItemProgress {
                        id: item_id,
                        title: ctx.url.clone(),
                        platform: ctx.platform_name.clone(),
                        percent: 0.0,
                        speed_bytes_per_sec: 0.0,
                        downloaded_bytes: 0,
                        total_bytes: None,
                        phase: "fetching_info".to_string(),
                    },
                );
            }

            let info_result = tokio::time::timeout(
                std::time::Duration::from_secs(60),
                fetch_and_cache_info(&ctx.url, &*ctx.downloader, &ctx.platform_name),
            )
            .await;

            match info_result {
                Ok(Ok(i)) => i,
                Ok(Err(e)) => {
                    let state = {
                        let mut q = queue.lock().await;
                        q.mark_complete(item_id, false, Some(e.to_string()), None, None);
                        q.get_state()
                    };
                    emit_queue_state_from_state(reporter, state);
                    tokio::spawn(try_start_next(queue.clone()));
                    return None;
                }
                Err(_) => {
                    tracing::warn!("[queue] info fetch timed out for {} after 60s", item_id);
                    let state = {
                        let mut q = queue.lock().await;
                        q.mark_complete(
                            item_id,
                            false,
                            Some("Timed out fetching video info".to_string()),
                            None,
                            None,
                        );
                        q.get_state()
                    };
                    emit_queue_state_from_state(reporter, state);
                    tokio::spawn(try_start_next(queue.clone()));
                    return None;
                }
            }
        }
    };
    tracing::info!(
        "[queue] info fetch for {} took {:?}",
        item_id,
        info_start.elapsed()
    );

    let state = {
        let mut q = queue.lock().await;
        if let Some(item) = q.items.iter_mut().find(|i| i.id == item_id) {
            item.title = info.title.clone();
            item.total_bytes = info.file_size_bytes;
            let fc = if info.media_type == crate::models::media::MediaType::Carousel
                || info.media_type == crate::models::media::MediaType::Playlist
            {
                info.available_qualities.len() as u32
            } else {
                1
            };
            item.file_count = Some(fc);
            item.media_info = Some(info.clone());
        }
        q.get_state()
    };
    emit_queue_state_from_state(reporter, state);

    if let Some(r) = reporter {
        r.on_progress(
            item_id,
            crate::core::events::QueueItemProgress {
                id: item_id,
                title: info.title.clone(),
                platform: ctx.platform_name.clone(),
                percent: 0.5,
                speed_bytes_per_sec: 0.0,
                downloaded_bytes: 0,
                total_bytes: info.file_size_bytes,
                phase: "starting".to_string(),
            },
        );
    }

    Some(info)
}

fn build_download_options(
    ctx: &DownloadContext,
) -> (
    crate::models::media::DownloadOptions,
    std::sync::Arc<tokio::sync::Mutex<Option<usize>>>,
) {
    let settings = crate::models::settings::AppSettings::load_from_disk();
    let tmpl = settings.download.filename_template.clone();
    let mut final_output_dir = std::path::PathBuf::from(&ctx.output_dir);
    if settings.download.organize_by_platform {
        final_output_dir = final_output_dir.join(&ctx.platform_name);
    }
    let torrent_id_slot = std::sync::Arc::new(tokio::sync::Mutex::new(None));
    let opts = crate::models::media::DownloadOptions {
        quality: ctx
            .quality
            .clone()
            .or_else(|| Some(settings.download.video_quality.clone())),
        video_format: ctx
            .video_format
            .clone()
            .or_else(|| Some(settings.download.video_format.clone())),
        audio_format: ctx
            .audio_format
            .clone()
            .or_else(|| Some(settings.download.audio_format.clone())),
        audio_quality: ctx
            .audio_quality
            .clone()
            .or_else(|| Some(settings.download.audio_quality.clone())),
        output_dir: final_output_dir,
        filename_template: Some(tmpl),
        download_subtitles: ctx
            .download_subtitles
            .unwrap_or(settings.download.download_subtitles),
        include_auto_subtitles: settings.download.include_auto_subtitles,
        download_mode: ctx.download_mode.clone(),
        format_id: ctx.format_id.clone(),
        referer: ctx.referer.clone(),
        extra_headers: ctx.extra_headers.clone(),
        page_url: ctx.page_url.clone(),
        user_agent: ctx.user_agent.clone(),
        cancel_token: ctx.cancel_token.clone(),
        concurrent_fragments: settings.advanced.concurrent_fragments,
        ytdlp_path: ctx.ytdlp_path.clone(),
        torrent_listen_port: Some(settings.advanced.torrent_listen_port),
        torrent_id_slot: Some(torrent_id_slot.clone()),
    };
    (opts, torrent_id_slot)
}

async fn handle_download_result(
    queue: Arc<tokio::sync::Mutex<DownloadQueue>>,
    item_id: u64,
    ctx: DownloadContext,
    info: crate::models::media::MediaInfo,
    reporter: Option<crate::core::traits::SharedReporter>,
    result: anyhow::Result<crate::models::media::DownloadResult>,
) {
    let settings = crate::models::settings::AppSettings::load_from_disk();
    match result {
        Ok(dl) => {
            if settings.download.embed_metadata
                && ctx.platform_name != "magnet"
                && crate::core::ffmpeg::is_ffmpeg_available().await
            {
                let metadata = crate::core::ffmpeg::MetadataEmbed {
                    title: Some(info.title.clone()),
                    artist: Some(info.author.clone()),
                    thumbnail_url: info.thumbnail_url.clone(),
                    ..Default::default()
                };
                if let Err(e) = crate::core::ffmpeg::embed_metadata(
                    &dl.file_path,
                    &metadata,
                    settings.download.embed_thumbnail,
                    shared_http_client(),
                )
                .await
                {
                    tracing::warn!("Metadata embed failed for '{}': {}", info.title, e);
                }
            }

            if ctx.from_hotkey && settings.download.copy_to_clipboard_on_hotkey {
                #[cfg(not(target_os = "android"))]
                {
                    match crate::core::clipboard::copy_file_to_clipboard(&dl.file_path).await {
                        Ok(()) => {
                            tracing::info!("[clipboard] file copied: {:?}", dl.file_path);
                        }
                        Err(e) => {
                            tracing::warn!("[clipboard] failed to copy file: {}", e);
                        }
                    }
                }
            }

            let state = {
                let mut q = queue.lock().await;
                if ctx.platform_name == "magnet" && dl.torrent_id.is_some() {
                    q.mark_seeding(
                        item_id,
                        Some(dl.file_path.to_string_lossy().to_string()),
                        Some(dl.file_size_bytes),
                        dl.torrent_id,
                    );
                } else {
                    q.mark_complete(
                        item_id,
                        true,
                        None,
                        Some(dl.file_path.to_string_lossy().to_string()),
                        Some(dl.file_size_bytes),
                    );
                }
                q.get_state()
            };
            if let Some(r) = &reporter {
                r.on_complete(
                    item_id,
                    Some(dl.file_path.to_string_lossy().to_string()),
                    Some(dl.file_size_bytes),
                );
            }
            emit_queue_state_from_state(&reporter, state);
        }
        Err(e) => {
            let raw_err = format!("{}", e);
            let (category, hint) = crate::core::errors::classify_download_error(&raw_err);
            let user_msg = if category != "unknown" {
                format!("{} ({})", hint, raw_err)
            } else {
                raw_err.clone()
            };
            tracing::error!(
                "Download error '{}' [{}]: {}",
                ctx.platform_name,
                category,
                raw_err
            );
            let state = {
                let mut q = queue.lock().await;
                q.mark_complete(item_id, false, Some(user_msg.clone()), None, None);
                q.get_state()
            };
            if let Some(r) = &reporter {
                r.on_error(item_id, user_msg);
            }
            emit_queue_state_from_state(&reporter, state);
        }
    }
}

async fn spawn_download_inner(queue: Arc<tokio::sync::Mutex<DownloadQueue>>, item_id: u64) {
    tracing::info!("[queue] download {} started", item_id);

    let reporter = { queue.lock().await.reporter.clone() };

    if let Some(r) = &reporter {
        r.on_progress(
            item_id,
            crate::core::events::QueueItemProgress {
                id: item_id,
                title: "".to_string(),
                platform: "".to_string(),
                percent: 0.0,
                speed_bytes_per_sec: 0.0,
                downloaded_bytes: 0,
                total_bytes: None,
                phase: "preparing".to_string(),
            },
        );
    }

    let ctx = match extract_download_context(&queue, item_id).await {
        Some(c) => c,
        None => return,
    };

    let info = match prepare_media_info(&queue, item_id, &ctx, &reporter).await {
        Some(i) => i,
        None => return,
    };

    let (opts, torrent_id_slot) = build_download_options(&ctx);

    let total_bytes = info.file_size_bytes;
    let item_title = info.title.clone();
    let item_platform = ctx.platform_name.clone();
    let (tx, mut rx) = tokio::sync::mpsc::channel::<f64>(32);

    let reporter_progress = reporter.clone();
    let queue_progress = queue.clone();
    let torrent_id_slot_progress = torrent_id_slot.clone();
    let progress_forwarder = tokio::spawn(async move {
        let mut last_bytes: u64 = 0;
        let mut last_time = std::time::Instant::now();
        let mut throttle = ProgressThrottle::new(250);
        let mut current_speed: f64 = 0.0;

        while let Some(percent) = rx.recv().await {
            if !throttle.should_emit() && percent < 100.0 {
                continue;
            }

            let now = std::time::Instant::now();
            let clamped = percent.max(0.0);
            let downloaded_bytes = total_bytes
                .map(|total| (clamped / 100.0 * total as f64) as u64)
                .unwrap_or(0);

            if total_bytes.is_some() && downloaded_bytes > last_bytes {
                let dt = now.duration_since(last_time).as_secs_f64();
                if dt > 0.1 {
                    let instant_speed = (downloaded_bytes - last_bytes) as f64 / dt;
                    current_speed = if current_speed > 0.0 {
                        current_speed * 0.7 + instant_speed * 0.3
                    } else {
                        instant_speed
                    };
                }
            }

            last_bytes = downloaded_bytes;
            last_time = now;

            let phase = match percent {
                p if p < -1.5 => "connecting",
                p if p < -0.5 => "starting",
                p if p > 0.0 => "downloading",
                _ => "starting",
            };

            {
                let mut q = queue_progress.lock().await;
                let tid = { *torrent_id_slot_progress.lock().await };
                q.update_progress(
                    item_id,
                    clamped,
                    current_speed,
                    downloaded_bytes,
                    total_bytes,
                    tid,
                );
            }

            if let Some(r) = &reporter_progress {
                r.on_progress(
                    item_id,
                    crate::core::events::QueueItemProgress {
                        id: item_id,
                        title: item_title.clone(),
                        platform: item_platform.clone(),
                        percent: clamped,
                        speed_bytes_per_sec: current_speed,
                        downloaded_bytes,
                        total_bytes,
                        phase: phase.to_string(),
                    },
                );
            }
        }
    });

    if let Some(ua) = opts.user_agent.clone() {
        crate::core::ytdlp::register_ext_user_agent(ctx.url.clone(), ua);
    }
    if let Some(hdrs) = opts.extra_headers.clone() {
        crate::core::ytdlp::register_ext_headers(ctx.url.clone(), hdrs);
    }

    // Ensure FFmpeg is installed if needed and try to surface progress to reporter (TUI/CLI)
    if !crate::core::ffmpeg::is_ffmpeg_available().await {
        let rep_ref = reporter.as_ref().map(|r| r.as_ref());
        match crate::core::dependencies::ensure_ffmpeg(rep_ref).await {
            Ok(path) => {
                tracing::info!("[ffmpeg] auto-installed to {:?}", path);
                crate::core::ffmpeg::reset_ffmpeg_available_cache();
            }
            Err(e) => tracing::warn!("[ffmpeg] auto-install failed: {}", e),
        }
    }

    let dl_start = std::time::Instant::now();
    let dl_future = async {
        tokio::select! {
            r = ctx.downloader.download(&info, &opts, tx) => r,
            _ = ctx.cancel_token.cancelled() => {
                Err(anyhow::anyhow!("Download cancelled"))
            }
        }
    };
    let result = crate::core::log_hook::CURRENT_DOWNLOAD_ID
        .scope(item_id, dl_future)
        .await;
    crate::core::ytdlp::clear_ext_user_agent(&ctx.url);
    crate::core::ytdlp::clear_ext_headers(&ctx.url);
    tracing::info!(
        "[queue] download {} completed in {:?}",
        item_id,
        dl_start.elapsed()
    );

    let _ = progress_forwarder.await;

    let was_paused = {
        let q = queue.lock().await;
        q.items
            .iter()
            .find(|i| i.id == item_id)
            .map(|i| i.status == QueueStatus::Paused)
            .unwrap_or(false)
    };

    if was_paused {
        let state = {
            let q = queue.lock().await;
            q.get_state()
        };
        emit_queue_state_from_state(&reporter, state);
        tokio::spawn(try_start_next(queue));
        return;
    }

    handle_download_result(queue.clone(), item_id, ctx, info, reporter, result).await;

    tokio::spawn(try_start_next(queue));
}

pub async fn fetch_and_cache_info(
    url: &str,
    downloader: &dyn PlatformDownloader,
    platform: &str,
) -> anyhow::Result<MediaInfo> {
    {
        let cache = info_cache().lock().await;
        if let Some(entry) = cache.get(url) {
            if entry.cached_at.elapsed() < INFO_CACHE_TTL {
                tracing::debug!("[perf] fetch_and_cache_info: cache hit for {}", platform);
                return Ok(entry.info.clone());
            }
        }
    }

    let url_lock = {
        let mut map = in_flight_map().lock().await;
        map.entry(url.to_string())
            .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
            .clone()
    };
    let _guard = url_lock.lock().await;

    {
        let cache = info_cache().lock().await;
        if let Some(entry) = cache.get(url) {
            if entry.cached_at.elapsed() < INFO_CACHE_TTL {
                tracing::debug!(
                    "[perf] fetch_and_cache_info: dedup cache hit for {}",
                    platform
                );
                return Ok(entry.info.clone());
            }
        }
    }

    tracing::debug!("[perf] fetch_and_cache_info: fetching for {}", platform);
    let mut info = downloader.get_media_info(url).await?;

    if is_generic_title(&info.title) {
        let name = crate::core::random_names::get_random_name();
        info.title = format!("video_{}", name);
    }

    let mut cache = info_cache().lock().await;
    cache.insert(
        url.to_string(),
        CachedInfo {
            info: info.clone(),
            cached_at: std::time::Instant::now(),
        },
    );
    if cache.len() > 50 {
        cache.retain(|_, v| v.cached_at.elapsed() < INFO_CACHE_TTL);
    }
    Ok(info)
}

pub async fn try_get_cached_info(url: &str) -> Option<MediaInfo> {
    let cache = info_cache().lock().await;
    cache
        .get(url)
        .filter(|entry| entry.cached_at.elapsed() < INFO_CACHE_TTL)
        .map(|entry| entry.info.clone())
}

pub async fn prefetch_info(url: &str, downloader: &dyn PlatformDownloader, platform: &str) {
    prefetch_info_with_emit(url, downloader, platform, None).await;
}

pub async fn prefetch_info_with_emit(
    url: &str,
    downloader: &dyn PlatformDownloader,
    platform: &str,
    reporter: Option<SharedReporter>,
) {
    let _timer_start = std::time::Instant::now();
    tracing::debug!("[perf] prefetch_info: started");
    match fetch_and_cache_info(url, downloader, platform).await {
        Ok(info) => {
            tracing::debug!(
                "[perf] prefetch_info: completed in {:?} — {}",
                _timer_start.elapsed(),
                info.title
            );
            if let Some(r) = reporter {
                r.on_media_preview(
                    url.to_string(),
                    info.title.clone(),
                    info.author.clone(),
                    info.thumbnail_url.clone(),
                    info.duration_seconds,
                );
            }
        }
        Err(e) => tracing::warn!(
            "[perf] prefetch_info: failed in {:?} — {}",
            _timer_start.elapsed(),
            e
        ),
    }
}

pub async fn try_start_next(queue: Arc<tokio::sync::Mutex<DownloadQueue>>) {
    let _timer_start = std::time::Instant::now();
    let (next_ids, stagger, state_to_emit, reporter) = {
        let mut q = queue.lock().await;
        let ids = q.next_queued_ids();
        for nid in &ids {
            q.mark_active(*nid);
        }
        let state = if !ids.is_empty() {
            Some(q.get_state())
        } else {
            None
        };
        (ids, q.stagger_delay_ms, state, q.reporter.clone())
    };

    if let Some(state) = state_to_emit {
        emit_queue_state_from_state(&reporter, state);
    }

    let batch_size = next_ids.len();
    for (i, nid) in next_ids.into_iter().enumerate() {
        if let Some(r) = &reporter {
            r.on_progress(
                nid,
                crate::core::events::QueueItemProgress {
                    id: nid,
                    title: String::new(),
                    platform: String::new(),
                    percent: 0.0,
                    speed_bytes_per_sec: 0.0,
                    downloaded_bytes: 0,
                    total_bytes: None,
                    phase: "queued_starting".to_string(),
                },
            );
        }

        if i > 0 {
            let item_platform = {
                let q = queue.lock().await;
                q.items
                    .iter()
                    .find(|item| item.id == nid)
                    .map(|item| item.platform.clone())
            };
            let delay_ms = if item_platform.as_deref() == Some("youtube") {
                2000
            } else if batch_size > 3 {
                stagger.max(1000)
            } else {
                stagger
            };
            if delay_ms > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
            }
        }
        let queue_c = queue.clone();
        tokio::spawn(async move {
            tokio::spawn(spawn_download(queue_c, nid));
        });
    }
    tracing::debug!("[perf] try_start_next took {:?}", _timer_start.elapsed());
}

fn is_generic_title(title: &str) -> bool {
    let t = title.to_lowercase();
    let t = t.trim();
    t.is_empty()
        || t == "video"
        || t == "media"
        || t == "untitled"
        || t == "unknown"
        || t.starts_with("video [video]")
        || t.starts_with("media [media]")
}
