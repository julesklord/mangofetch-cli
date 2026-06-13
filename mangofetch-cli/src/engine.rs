use anyhow::Result;
use mangofetch_core::core::dependencies::ensure_dependencies;
use mangofetch_core::core::manager::queue::DownloadQueue;
use mangofetch_core::core::registry::PlatformRegistry;
use std::sync::Arc;
use tokio::sync::Mutex;

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

#[allow(clippy::too_many_arguments)]
pub async fn enqueue_download_with_quality(
    url: &str,
    output_dir: Option<String>,
    quality: Option<String>,
    video_format: Option<String>,
    audio_format: Option<String>,
    audio_quality: Option<String>,
    registry: Arc<PlatformRegistry>,
    queue: Arc<Mutex<DownloadQueue>>,
) -> Result<()> {
    let downloader = registry
        .find_platform(url)
        .ok_or_else(|| anyhow::anyhow!("No supported platform found for URL"))?;
    let platform_name = downloader.name().to_string();

    let output = output_dir.unwrap_or_else(|| {
        dirs::download_dir()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
            .to_string_lossy()
            .to_string()
    });

    let deps = ensure_dependencies(false, None).await?;

    let media_info = mangofetch_core::core::manager::queue::fetch_and_cache_info(
        url,
        &*downloader,
        &platform_name,
    )
    .await
    .ok();

    let id = mangofetch_core::core::manager::recovery::get_next_id();

    if let Some(ref info) = media_info {
        if info.media_type == mangofetch_core::models::media::MediaType::Playlist {
            for entry in &info.available_qualities {
                let pid = mangofetch_core::core::manager::recovery::get_next_id();
                let mut q = queue.lock().await;
                q.enqueue(
                    pid,
                    entry.url.clone(),
                    platform_name.clone(),
                    entry.label.clone(),
                    output.clone(),
                    None,
                    quality.clone(),
                    video_format.clone(),
                    audio_format.clone(),
                    audio_quality.clone(),
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
                    deps.ytdlp.clone(),
                    false,
                );
            }
            mangofetch_core::core::manager::queue::try_start_next(queue.clone()).await;
            return Ok(());
        }
    }

    let mut q = queue.lock().await;
    q.enqueue(
        id,
        url.to_string(),
        platform_name,
        media_info
            .as_ref()
            .map(|i| i.title.clone())
            .unwrap_or_else(|| url.to_string()),
        output,
        None,
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
        false,
    );

    drop(q);
    mangofetch_core::core::manager::queue::try_start_next(queue.clone()).await;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn enqueue_download_with_overrides(
    url: &str,
    output_dir: Option<String>,
    quality: Option<String>,
    download_mode: Option<String>,
    video_format: Option<String>,
    audio_format: Option<String>,
    audio_quality: Option<String>,
    download_subtitles: Option<bool>,
    registry: Arc<PlatformRegistry>,
    queue: Arc<Mutex<DownloadQueue>>,
) -> Result<()> {
    let downloader = registry
        .find_platform(url)
        .ok_or_else(|| anyhow::anyhow!("No supported platform found for URL"))?;
    let platform_name = downloader.name().to_string();

    let output = output_dir.unwrap_or_else(|| {
        dirs::download_dir()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
            .to_string_lossy()
            .to_string()
    });

    let deps = ensure_dependencies(false, None).await?;

    let media_info = mangofetch_core::core::manager::queue::fetch_and_cache_info(
        url,
        &*downloader,
        &platform_name,
    )
    .await
    .ok();

    let id = mangofetch_core::core::manager::recovery::get_next_id();

    if let Some(ref info) = media_info {
        if info.media_type == mangofetch_core::models::media::MediaType::Playlist {
            for entry in &info.available_qualities {
                let pid = mangofetch_core::core::manager::recovery::get_next_id();
                let mut q = queue.lock().await;
                q.enqueue(
                    pid,
                    entry.url.clone(),
                    platform_name.clone(),
                    entry.label.clone(),
                    output.clone(),
                    download_mode.clone(),
                    quality.clone(),
                    video_format.clone(),
                    audio_format.clone(),
                    audio_quality.clone(),
                    download_subtitles,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    downloader.clone(),
                    deps.ytdlp.clone(),
                    false,
                );
            }
            mangofetch_core::core::manager::queue::try_start_next(queue.clone()).await;
            return Ok(());
        }
    }

    let mut q = queue.lock().await;
    q.enqueue(
        id,
        url.to_string(),
        platform_name,
        media_info
            .as_ref()
            .map(|i| i.title.clone())
            .unwrap_or_else(|| url.to_string()),
        output,
        download_mode,
        quality,
        video_format,
        audio_format,
        audio_quality,
        download_subtitles,
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
        false,
    );

    drop(q);
    mangofetch_core::core::manager::queue::try_start_next(queue.clone()).await;
    Ok(())
}

#[allow(dead_code)]
pub async fn enqueue_download(
    url: &str,
    output_dir: Option<String>,
    video_format: Option<String>,
    audio_format: Option<String>,
    audio_quality: Option<String>,
    registry: Arc<PlatformRegistry>,
    queue: Arc<Mutex<DownloadQueue>>,
) -> Result<()> {
    let downloader = registry
        .find_platform(url)
        .ok_or_else(|| anyhow::anyhow!("No supported platform found for URL"))?;
    let platform_name = downloader.name().to_string();

    let output = output_dir.unwrap_or_else(|| {
        dirs::download_dir()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
            .to_string_lossy()
            .to_string()
    });

    let deps = ensure_dependencies(false, None).await?;

    let media_info = mangofetch_core::core::manager::queue::fetch_and_cache_info(
        url,
        &*downloader,
        &platform_name,
    )
    .await
    .ok();

    let id = mangofetch_core::core::manager::recovery::get_next_id();

    let mut q = queue.lock().await;
    q.enqueue(
        id,
        url.to_string(),
        platform_name,
        media_info
            .as_ref()
            .map(|i| i.title.clone())
            .unwrap_or_else(|| url.to_string()),
        output,
        None,
        None,
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
        false,
    );

    drop(q);
    mangofetch_core::core::manager::queue::try_start_next(queue.clone()).await;
    Ok(())
}
