//! Tokio runtime en thread separado
//! Maneja la comunicación entre UI (egui) y core (tokio async)

use crate::bridge::{CoreEvent, GuiCommand, MediaInfo, QueueItemInfo};
use mangofetch_core::core::dependencies::ensure_dependencies;
use mangofetch_core::core::manager::queue::{fetch_and_cache_info, try_start_next, DownloadQueue};
use mangofetch_core::core::manager::recovery;
use mangofetch_core::core::registry::PlatformRegistry;
use mangofetch_core::core::traits::DownloadReporter;
use mangofetch_core::models::queue::QueueStatus;
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tokio::runtime::Runtime;
use tokio::sync::Mutex;

pub struct AppRuntime {
    pub cmd_tx: Sender<GuiCommand>,
    pub event_rx: Receiver<CoreEvent>,
    _thread_handle: thread::JoinHandle<()>,
}

/// Custom reporter that pushes events directly to our egui main loop through the channel.
struct GuiReporter {
    event_tx: Arc<std::sync::Mutex<Sender<CoreEvent>>>,
}

impl GuiReporter {
    fn new(tx: Sender<CoreEvent>) -> Self {
        Self {
            event_tx: Arc::new(std::sync::Mutex::new(tx)),
        }
    }

    fn send(&self, ev: CoreEvent) {
        if let Ok(tx) = self.event_tx.lock() {
            let _ = tx.send(ev);
        }
    }

    fn format_bytes_compact(bytes: u64) -> String {
        if bytes < 1024 {
            format!("{} B", bytes)
        } else if bytes < 1_048_576 {
            format!("{:.1} KB", bytes as f64 / 1024.0)
        } else if bytes < 1_073_741_824 {
            format!("{:.1} MB", bytes as f64 / 1_048_576.0)
        } else {
            format!("{:.2} GB", bytes as f64 / 1_073_741_824.0)
        }
    }
}

impl DownloadReporter for GuiReporter {
    fn on_progress(
        &self,
        download_id: u64,
        info: mangofetch_core::core::events::QueueItemProgress,
    ) {
        self.send(CoreEvent::DownloadProgress {
            id: download_id,
            progress: info.percent as f32,
            speed: info.speed_bytes_per_sec,
            eta: None,
        });

        let speed_str = if info.speed_bytes_per_sec > 0.0 {
            format!(
                "{}/s",
                Self::format_bytes_compact(info.speed_bytes_per_sec as u64)
            )
        } else {
            "--".to_string()
        };

        self.send(CoreEvent::LogLine(format!(
            "[DL#{:02}] {:.1}% | Speed: {} | Phase: {}",
            download_id, info.percent, speed_str, info.phase
        )));
    }

    fn on_complete(
        &self,
        download_id: u64,
        file_path: Option<String>,
        file_size_bytes: Option<u64>,
    ) {
        let size = file_size_bytes
            .map(Self::format_bytes_compact)
            .unwrap_or_else(|| "--".to_string());
        let path = file_path.as_deref().unwrap_or("--");

        self.send(CoreEvent::DownloadComplete {
            id: download_id,
            title: path.to_string(),
        });

        self.send(CoreEvent::LogLine(format!(
            "✓ [DL#{:02}] COMPLETE | Size: {} | Path: {}",
            download_id, size, path
        )));
    }

    fn on_error(&self, download_id: u64, error_message: String) {
        self.send(CoreEvent::DownloadError {
            id: download_id,
            error: error_message.clone(),
        });

        self.send(CoreEvent::LogLine(format!(
            "✗ [DL#{:02}] ERROR | {}",
            download_id, error_message
        )));
    }

    fn on_retry(&self, download_id: u64, attempt: u32, delay_ms: u64) {
        self.send(CoreEvent::LogLine(format!(
            "↻ [DL#{:02}] RETRY | Attempt {} in {}ms",
            download_id, attempt, delay_ms
        )));
    }

    fn on_phase_change(&self, download_id: u64, phase: String) {
        self.send(CoreEvent::LogLine(format!(
            "⟫ [DL#{:02}] Phase transition: {}",
            download_id, phase
        )));
    }

    fn on_media_preview(
        &self,
        _url: String,
        title: String,
        _author: String,
        _thumbnail_url: Option<String>,
        duration_seconds: Option<f64>,
    ) {
        self.send(CoreEvent::MediaInfoFetched(Ok(MediaInfo {
            title,
            duration: duration_seconds.map(|d| d as u64),
            platform: "Generic".to_string(),
            available_formats: vec![],
        })));
    }

    fn on_queue_update(&self, state: Vec<mangofetch_core::models::queue::QueueItemInfo>) {
        let gui_items = state
            .into_iter()
            .map(|i| {
                let status_str = match &i.status {
                    QueueStatus::Active => "Active".to_string(),
                    QueueStatus::Queued => "Queued".to_string(),
                    QueueStatus::Paused => "Paused".to_string(),
                    QueueStatus::Seeding => "Seeding".to_string(),
                    QueueStatus::Complete { .. } => "Complete".to_string(),
                    QueueStatus::Error { message } => format!("Error: {}", message),
                };
                QueueItemInfo {
                    id: i.id,
                    title: i.title,
                    platform: i.platform,
                    status: status_str,
                    progress: i.percent as f32,
                    speed: i.speed_bytes_per_sec,
                    eta: None,
                }
            })
            .collect();
        self.send(CoreEvent::QueueUpdated(gui_items));
    }

    fn on_system_progress(&self, title: &str, percent: f32, message: &str) {
        self.send(CoreEvent::LogLine(format!(
            "⚙ System: {} [{:.1}%] - {}",
            title, percent, message
        )));
    }
}

pub fn register_platforms(registry: &mut PlatformRegistry) {
    use mangofetch_core::platforms::*;
    registry.register(Arc::new(instagram::InstagramDownloader::new()));
    registry.register(Arc::new(pinterest::PinterestDownloader::new()));
    registry.register(Arc::new(tiktok::TikTokDownloader::new()));
    registry.register(Arc::new(twitter::TwitterDownloader::new()));
    registry.register(Arc::new(twitch::TwitchClipsDownloader::new()));
    registry.register(Arc::new(bluesky::BlueskyDownloader::new()));
    registry.register(Arc::new(reddit::RedditDownloader::new()));
    registry.register(Arc::new(youtube::YouTubeDownloader::new()));
    registry.register(Arc::new(vimeo::VimeoDownloader::new()));
    registry.register(Arc::new(bilibili::BilibiliDownloader::new()));
    let torrent_session = Arc::new(tokio::sync::Mutex::new(None));
    registry.register(Arc::new(magnet::MagnetDownloader::new(torrent_session)));
    registry.register(Arc::new(p2p::P2pDownloader::new()));
    registry.register(Arc::new(generic_ytdlp::GenericYtdlpDownloader::new()));
}

impl AppRuntime {
    /// Inicia el runtime en un thread separado
    pub fn start() -> Self {
        let (cmd_tx, cmd_rx) = channel::<GuiCommand>();
        let (event_tx, event_rx) = channel::<CoreEvent>();

        let event_tx_reporter = event_tx.clone();

        let thread_handle = thread::spawn(move || {
            let rt = Runtime::new().expect("Failed to create Tokio runtime");

            tracing::info!("AppRuntime started");

            // Initialize recovery from disk
            recovery::init_from_disk();

            // Set up our reporter
            let reporter = Arc::new(GuiReporter::new(event_tx_reporter));

            let mut registry = PlatformRegistry::new();
            register_platforms(&mut registry);
            let registry = Arc::new(registry);

            let mut q_obj = DownloadQueue::new(3, Some(reporter.clone()));
            q_obj.load_from_recovery(&registry);
            let queue = Arc::new(Mutex::new(q_obj));

            // Polling interval for updates
            let poll_duration = Duration::from_millis(250);

            loop {
                match cmd_rx.recv_timeout(poll_duration) {
                    Ok(cmd) => {
                        tracing::info!("AppRuntime received command: {:?}", cmd);

                        match cmd {
                            GuiCommand::StartDownload {
                                url,
                                output_dir,
                                quality,
                                video_format,
                                audio_format,
                                audio_quality,
                                audio_only,
                            } => {
                                let queue = queue.clone();
                                let registry = registry.clone();
                                let reporter = reporter.clone();

                                rt.spawn(async move {
                                    let downloader = match registry.find_platform(&url) {
                                        Some(d) => d,
                                        None => {
                                            reporter.send(CoreEvent::LogLine(
                                                "Error: Plataforma no soportada".to_string(),
                                            ));
                                            return;
                                        }
                                    };
                                    let platform_name = downloader.name().to_string();

                                    let deps =
                                        match ensure_dependencies(false, Some(reporter.clone()))
                                            .await
                                        {
                                            Ok(d) => d,
                                            Err(e) => {
                                                reporter.send(CoreEvent::LogLine(format!(
                                                    "Error de dependencias: {}",
                                                    e
                                                )));
                                                return;
                                            }
                                        };

                                    let media_info =
                                        fetch_and_cache_info(&url, &*downloader, &platform_name)
                                            .await
                                            .ok();
                                    let id = recovery::get_next_id();
                                    let download_mode = if audio_only {
                                        Some("audio".to_string())
                                    } else {
                                        None
                                    };

                                    let mut q = queue.lock().await;
                                    q.enqueue(
                                        id,
                                        url,
                                        platform_name,
                                        media_info
                                            .as_ref()
                                            .map(|i| i.title.clone())
                                            .unwrap_or_else(|| "Download".to_string()),
                                        output_dir,
                                        download_mode,
                                        quality,
                                        video_format,
                                        audio_format,
                                        audio_quality,
                                        None,
                                        None,
                                        None,
                                        None,
                                        None,
                                        None,
                                        media_info,
                                        None,
                                        None,
                                        downloader,
                                        deps.ytdlp,
                                        audio_only,
                                    );
                                    drop(q);
                                    try_start_next(queue).await;
                                });
                            }
                            GuiCommand::PauseDownload { id } => {
                                let queue = queue.clone();
                                rt.spawn(async move {
                                    let mut q = queue.lock().await;
                                    q.pause(id);
                                });
                            }
                            GuiCommand::ResumeDownload { id } => {
                                let queue = queue.clone();
                                rt.spawn(async move {
                                    let mut q = queue.lock().await;
                                    q.resume(id);
                                });
                            }
                            GuiCommand::RemoveDownload { id } => {
                                let queue = queue.clone();
                                rt.spawn(async move {
                                    let mut q = queue.lock().await;
                                    q.remove(id);
                                });
                            }
                            GuiCommand::RefreshQueue => {
                                let queue = queue.clone();
                                let event_tx_reporter = event_tx.clone();
                                rt.spawn(async move {
                                    let q = queue.lock().await;
                                    let state = q.get_state();

                                    let gui_items = state
                                        .into_iter()
                                        .map(|i| {
                                            let status_str = match &i.status {
                                                QueueStatus::Active => "Active".to_string(),
                                                QueueStatus::Queued => "Queued".to_string(),
                                                QueueStatus::Paused => "Paused".to_string(),
                                                QueueStatus::Seeding => "Seeding".to_string(),
                                                QueueStatus::Complete { .. } => {
                                                    "Complete".to_string()
                                                }
                                                QueueStatus::Error { message } => {
                                                    format!("Error: {}", message)
                                                }
                                            };
                                            QueueItemInfo {
                                                id: i.id,
                                                title: i.title,
                                                platform: i.platform,
                                                status: status_str,
                                                progress: i.percent as f32,
                                                speed: i.speed_bytes_per_sec,
                                                eta: None,
                                            }
                                        })
                                        .collect();
                                    let _ =
                                        event_tx_reporter.send(CoreEvent::QueueUpdated(gui_items));
                                });
                            }
                            GuiCommand::CheckDependencies => {
                                let reporter = reporter.clone();
                                rt.spawn(async move {
                                    match ensure_dependencies(false, Some(reporter.clone())).await {
                                        Ok(deps) => {
                                            let _ = reporter.event_tx.lock().unwrap().send(
                                                CoreEvent::DependencyStatus {
                                                    ytdlp: deps.ytdlp.is_some(),
                                                    ffmpeg: deps.ffmpeg.is_some(),
                                                },
                                            );
                                        }
                                        Err(_) => {
                                            let _ = reporter.event_tx.lock().unwrap().send(
                                                CoreEvent::DependencyStatus {
                                                    ytdlp: false,
                                                    ffmpeg: false,
                                                },
                                            );
                                        }
                                    }
                                });
                            }
                            GuiCommand::FetchMediaInfo { url } => {
                                let registry = registry.clone();
                                let reporter = reporter.clone();
                                rt.spawn(async move {
                                    let downloader = match registry.find_platform(&url) {
                                        Some(d) => d,
                                        None => {
                                            let _ = reporter.event_tx.lock().unwrap().send(
                                                CoreEvent::MediaInfoFetched(Err(
                                                    "Platform not supported".to_string(),
                                                )),
                                            );
                                            return;
                                        }
                                    };
                                    let platform_name = downloader.name().to_string();

                                    match fetch_and_cache_info(&url, &*downloader, &platform_name)
                                        .await
                                    {
                                        Ok(info) => {
                                            let formats = info
                                                .available_qualities
                                                .iter()
                                                .map(|q| q.label.clone())
                                                .collect();
                                            let _ = reporter.event_tx.lock().unwrap().send(
                                                CoreEvent::MediaInfoFetched(Ok(MediaInfo {
                                                    title: info.title,
                                                    duration: info
                                                        .duration_seconds
                                                        .map(|d| d as u64),
                                                    platform: platform_name,
                                                    available_formats: formats,
                                                })),
                                            );
                                        }
                                        Err(e) => {
                                            let _ = reporter.event_tx.lock().unwrap().send(
                                                CoreEvent::MediaInfoFetched(Err(e.to_string())),
                                            );
                                        }
                                    }
                                });
                            }
                            GuiCommand::Shutdown => {
                                break;
                            }
                        }
                    }
                    Err(RecvTimeoutError::Timeout) => {
                        // Periodic state updates to UI
                        let queue = queue.clone();
                        let event_tx_reporter = event_tx.clone();
                        rt.spawn(async move {
                            let q = queue.lock().await;
                            let state = q.get_state();

                            let gui_items = state
                                .into_iter()
                                .map(|i| {
                                    let status_str = match &i.status {
                                        QueueStatus::Active => "Active".to_string(),
                                        QueueStatus::Queued => "Queued".to_string(),
                                        QueueStatus::Paused => "Paused".to_string(),
                                        QueueStatus::Seeding => "Seeding".to_string(),
                                        QueueStatus::Complete { .. } => "Complete".to_string(),
                                        QueueStatus::Error { message } => {
                                            format!("Error: {}", message)
                                        }
                                    };
                                    QueueItemInfo {
                                        id: i.id,
                                        title: i.title,
                                        platform: i.platform,
                                        status: status_str,
                                        progress: i.percent as f32,
                                        speed: i.speed_bytes_per_sec,
                                        eta: None,
                                    }
                                })
                                .collect();
                            let _ = event_tx_reporter.send(CoreEvent::QueueUpdated(gui_items));
                        });
                    }
                    Err(RecvTimeoutError::Disconnected) => {
                        tracing::info!(
                            "Command channel disconnected, shutting down runtime thread"
                        );
                        break;
                    }
                }
            }

            tracing::info!("AppRuntime shutdown");
        });

        AppRuntime {
            cmd_tx,
            event_rx,
            _thread_handle: thread_handle,
        }
    }

    /// Enviar un comando al runtime
    pub fn send_command(&self, cmd: GuiCommand) -> anyhow::Result<()> {
        self.cmd_tx.send(cmd)?;
        Ok(())
    }

    /// Drenar todos los eventos pendientes
    pub fn drain_events(&self) -> Vec<CoreEvent> {
        let mut events = Vec::new();
        loop {
            match self.event_rx.try_recv() {
                Ok(ev) => events.push(ev),
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => break,
            }
        }
        events
    }
}

impl Drop for AppRuntime {
    fn drop(&mut self) {
        // Enviar shutdown al cerrar el App
        let _ = self.cmd_tx.send(GuiCommand::Shutdown);
    }
}
