use anyhow::anyhow;
use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::core::direct_downloader;
use crate::core::hls_downloader::HlsDownloader;
use crate::core::ytdlp;
use crate::models::media::{
    DownloadOptions, DownloadResult, MediaInfo, MediaType, VideoQuality as MediaVideoQuality,
};
use crate::platforms::traits::PlatformDownloader;

pub struct GenericYtdlpDownloader;

impl Default for GenericYtdlpDownloader {
    fn default() -> Self {
        Self::new()
    }
}

impl GenericYtdlpDownloader {
    pub fn new() -> Self {
        Self
    }

    fn detect_platform(json: &serde_json::Value) -> String {
        json.get("extractor_key")
            .or_else(|| json.get("extractor"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_lowercase())
            .unwrap_or_else(|| "generic".to_string())
    }

    fn detect_media_type(json: &serde_json::Value) -> MediaType {
        let has_video = json
            .get("formats")
            .and_then(|v| v.as_array())
            .map(|formats| {
                formats.iter().any(|f| {
                    f.get("vcodec")
                        .and_then(|v| v.as_str())
                        .map(|v| v != "none")
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false);

        if has_video {
            MediaType::Video
        } else {
            MediaType::Audio
        }
    }

    pub fn parse_video_info(json: &serde_json::Value) -> anyhow::Result<MediaInfo> {
        let title = json
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let author = json
            .get("uploader")
            .or_else(|| json.get("channel"))
            .or_else(|| json.get("uploader_id"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let duration = json.get("duration").and_then(|v| v.as_f64());

        let thumbnail = json
            .get("thumbnail")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let webpage_url = json
            .get("webpage_url")
            .or_else(|| json.get("url"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let platform = Self::detect_platform(json);
        let media_type = Self::detect_media_type(json);

        let mut qualities: Vec<MediaVideoQuality> = Vec::new();

        if media_type == MediaType::Video {
            let mut quality_map: std::collections::BTreeMap<u32, MediaVideoQuality> =
                std::collections::BTreeMap::new();
            if let Some(formats) = json.get("formats").and_then(|v| v.as_array()) {
                for f in formats {
                    let height = f.get("height").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                    let width = f.get("width").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                    let vcodec = f.get("vcodec").and_then(|v| v.as_str()).unwrap_or("none");
                    let acodec = f.get("acodec").and_then(|v| v.as_str()).unwrap_or("none");

                    if vcodec == "none" || height == 0 {
                        continue;
                    }

                    let has_audio = acodec != "none";
                    let filesize_bytes = f
                        .get("filesize")
                        .and_then(|v| v.as_u64())
                        .or_else(|| f.get("filesize_approx").and_then(|v| v.as_u64()));

                    let label = if has_audio {
                        format!("{}p", height)
                    } else {
                        format!("{}p (HD)", height)
                    };

                    let q = MediaVideoQuality {
                        label,
                        width,
                        height,
                        url: webpage_url.clone(),
                        format: "ytdlp".to_string(),
                        filesize_bytes,
                    };

                    if let Some(existing) = quality_map.get(&height) {
                        let existing_has_audio = !existing.label.contains("(HD)");
                        let replace = (!existing_has_audio && has_audio)
                            || (filesize_bytes.is_some() && existing.filesize_bytes.is_none());
                        if replace {
                            quality_map.insert(height, q);
                        }
                    } else {
                        quality_map.insert(height, q);
                    }
                }
            }

            qualities = quality_map.into_values().rev().collect();
        }

        if qualities.is_empty() {
            qualities.push(MediaVideoQuality {
                label: "best".to_string(),
                width: 0,
                height: 0,
                url: webpage_url,
                format: "ytdlp".to_string(),

                filesize_bytes: None,
            });
        }

        Ok(MediaInfo {
            title,
            author,
            platform,
            duration_seconds: duration,
            thumbnail_url: thumbnail,
            available_qualities: qualities,
            media_type,
            file_size_bytes: None,
        })
    }
}

fn is_direct_media_url(url: &str) -> Option<&'static str> {
    let path = url::Url::parse(url).ok().map(|u| u.path().to_lowercase())?;

    if path.contains(".m3u8") {
        return Some("hls");
    }

    for ext in &[".mp4", ".webm", ".m4v", ".mkv", ".avi", ".mov", ".flv"] {
        if path.contains(ext) {
            return Some("direct");
        }
    }

    for ext in &[".mp3", ".m4a", ".ogg", ".wav", ".flac", ".aac"] {
        if path.contains(ext) {
            return Some("direct");
        }
    }

    None
}

fn filename_from_url(url: &str) -> String {
    url::Url::parse(url)
        .ok()
        .and_then(|u| {
            let path = u.path();
            let last = path.rsplit('/').next()?;
            if last.is_empty() || !last.contains('.') {
                return None;
            }
            Some(
                urlencoding::decode(last)
                    .unwrap_or_else(|_| last.into())
                    .to_string(),
            )
        })
        .map(|name| sanitize_filename::sanitize(&name))
        .unwrap_or_else(|| "download".to_string())
}

fn build_direct_media_info(url: &str, media_type_hint: &str) -> MediaInfo {
    let title = filename_from_url(url);
    let (format, media_type) = match media_type_hint {
        "hls" => ("hls".to_string(), MediaType::Video),
        _ => {
            let lower = url.to_lowercase();
            if lower.contains(".mp3")
                || lower.contains(".m4a")
                || lower.contains(".ogg")
                || lower.contains(".wav")
                || lower.contains(".flac")
                || lower.contains(".aac")
            {
                ("direct_audio".to_string(), MediaType::Audio)
            } else {
                ("direct_video".to_string(), MediaType::Video)
            }
        }
    };

    MediaInfo {
        title,
        author: String::new(),
        platform: "generic".to_string(),
        duration_seconds: None,
        thumbnail_url: None,
        available_qualities: vec![MediaVideoQuality {
            label: "original".to_string(),
            width: 0,
            height: 0,
            url: url.to_string(),
            format,

            filesize_bytes: None,
        }],
        media_type,
        file_size_bytes: None,
    }
}

#[async_trait]
impl PlatformDownloader for GenericYtdlpDownloader {
    fn name(&self) -> &str {
        "generic"
    }

    fn can_handle(&self, url: &str) -> bool {
        if let Ok(parsed) = url::Url::parse(url) {
            let scheme = parsed.scheme();
            return scheme == "http" || scheme == "https";
        }
        false
    }

    async fn get_media_info(&self, url: &str) -> anyhow::Result<MediaInfo> {
        if let Some(media_type) = is_direct_media_url(url) {
            return Ok(build_direct_media_info(url, media_type));
        }

        let ytdlp_path = ytdlp::ensure_ytdlp(None)
            .await
            .map_err(|e| anyhow!("yt-dlp unavailable: {}", e))?;

        let extra = platform_extra_flags(url);
        let json = ytdlp::get_video_info(&ytdlp_path, url, &extra).await?;
        Self::parse_video_info(&json)
    }

    async fn download(
        &self,
        info: &MediaInfo,
        opts: &DownloadOptions,
        progress: mpsc::Sender<f64>,
    ) -> anyhow::Result<DownloadResult> {
        let _ = progress.send(0.0).await;

        let first = info
            .available_qualities
            .first()
            .ok_or_else(|| anyhow!("No quality available"))?;

        let selected = if let Some(ref wanted) = opts.quality {
            info.get_closest_quality(wanted).unwrap_or(first)
        } else {
            first
        };

        if selected.format == "hls" {
            let title = sanitize_filename::sanitize(&info.title);
            let filename = if title.ends_with(".mp4") {
                title
            } else {
                format!("{}.mp4", title)
            };
            let output_path = opts.output_dir.join(&filename);
            let output_str = output_path.to_string_lossy().to_string();

            let referer = opts
                .referer
                .as_deref()
                .or_else(|| platform_referer(&selected.url))
                .unwrap_or("");

            let mut builder =
                crate::core::http_client::apply_global_proxy(reqwest::Client::builder())
                    .timeout(std::time::Duration::from_secs(600));

            if let Some(ua) = opts.user_agent.as_deref() {
                builder = builder.user_agent(ua);
            }

            if let Some(ref hdrs) = opts.extra_headers {
                let mut default_headers = reqwest::header::HeaderMap::new();
                for (name, value) in hdrs {
                    if let (Ok(hname), Ok(hval)) = (
                        reqwest::header::HeaderName::from_bytes(name.as_bytes()),
                        reqwest::header::HeaderValue::from_str(value),
                    ) {
                        default_headers.insert(hname, hval);
                    }
                }
                if !default_headers.is_empty() {
                    builder = builder.default_headers(default_headers);
                }
            }

            let jar = crate::core::cookie_parser::load_extension_cookies_for_url(&selected.url)
                .or_else(|| {
                    opts.referer
                        .as_deref()
                        .and_then(crate::core::cookie_parser::load_extension_cookies_for_url)
                });
            if let Some(jar) = jar {
                builder = builder.cookie_provider(jar);
            }

            let client = builder.build().unwrap_or_default();
            let downloader = HlsDownloader::with_client(client)
                .with_user_agent_override(opts.user_agent.clone());
            let _ = progress.send(0.0).await;

            let result = downloader
                .download(
                    &selected.url,
                    &output_str,
                    referer,
                    None,
                    opts.cancel_token.clone(),
                    20,
                    3,
                )
                .await?;

            let _ = progress.send(100.0).await;

            return Ok(DownloadResult {
                file_path: result.path,
                file_size_bytes: result.file_size,
                duration_seconds: 0.0,
                torrent_id: None,
            });
        }

        if selected.format == "direct_video" || selected.format == "direct_audio" {
            let title = sanitize_filename::sanitize(&info.title);
            let output_path = opts.output_dir.join(&title);

            let mut builder =
                crate::core::http_client::apply_global_proxy(reqwest::Client::builder())
                    .timeout(std::time::Duration::from_secs(600));

            if let Some(ua) = opts.user_agent.as_deref() {
                builder = builder.user_agent(ua);
            }

            let jar = crate::core::cookie_parser::load_extension_cookies_for_url(&selected.url)
                .or_else(|| {
                    opts.referer
                        .as_deref()
                        .and_then(crate::core::cookie_parser::load_extension_cookies_for_url)
                });
            if let Some(jar) = jar {
                builder = builder.cookie_provider(jar);
            }

            let client = builder.build().unwrap_or_default();

            let mut headers = reqwest::header::HeaderMap::new();
            if let Some(ref r) = opts.referer {
                if let Ok(val) = reqwest::header::HeaderValue::from_str(r) {
                    headers.insert(reqwest::header::REFERER, val);
                }
            }
            if let Some(ref hdrs) = opts.extra_headers {
                for (name, value) in hdrs {
                    if let (Ok(hname), Ok(hval)) = (
                        reqwest::header::HeaderName::from_bytes(name.as_bytes()),
                        reqwest::header::HeaderValue::from_str(value),
                    ) {
                        headers.insert(hname, hval);
                    }
                }
            }
            crate::core::http_client::inject_ua_header(&mut headers, opts.user_agent.as_deref());

            let bytes = direct_downloader::download_direct_with_headers(
                &client,
                &selected.url,
                &output_path,
                progress,
                Some(headers),
                Some(&opts.cancel_token),
            )
            .await?;

            return Ok(DownloadResult {
                file_path: output_path,
                file_size_bytes: bytes,
                duration_seconds: 0.0,
                torrent_id: None,
            });
        }

        let ytdlp_path = if let Some(ref p) = opts.ytdlp_path {
            p.clone()
        } else {
            ytdlp::ensure_ytdlp(None).await?
        };

        let quality_height = if selected.height > 0 {
            Some(selected.height)
        } else {
            None
        };
        let video_url = &selected.url;

        let referer = opts
            .referer
            .as_deref()
            .or_else(|| platform_referer(video_url));

        ytdlp::download_video(
            &ytdlp_path,
            video_url,
            &opts.output_dir,
            quality_height,
            progress,
            opts.download_mode.as_deref(),
            opts.video_format.as_deref(),
            opts.audio_format.as_deref(),
            opts.audio_quality.as_deref(),
            opts.format_id.as_deref(),
            opts.filename_template.as_deref(),
            referer,
            opts.cancel_token.clone(),
            None,
            opts.concurrent_fragments,
            opts.download_subtitles,
            &[],
        )
        .await
    }
}

fn platform_extra_flags(url: &str) -> Vec<String> {
    match platform_referer(url) {
        Some(r) => vec!["--referer".into(), r.to_string()],
        None => Vec::new(),
    }
}

fn platform_referer(url: &str) -> Option<&'static str> {
    let lower = url.to_lowercase();

    if lower.contains("douyin.com") || lower.contains("iesdouyin.com") {
        return Some("https://www.douyin.com/");
    }
    if lower.contains("v.qq.com") || lower.contains("qq.com/x/") {
        return Some("https://v.qq.com/");
    }
    if lower.contains("youku.com") {
        return Some("https://www.youku.com/");
    }
    if lower.contains("iqiyi.com") {
        return Some("https://www.iqiyi.com/");
    }
    if lower.contains("mgtv.com") {
        return Some("https://www.mgtv.com/");
    }
    if lower.contains("kuaishou.com") {
        return Some("https://www.kuaishou.com/");
    }
    if lower.contains("xiaohongshu.com") || lower.contains("xhslink.com") {
        return Some("https://www.xiaohongshu.com/");
    }
    if lower.contains("bilibili.com") || lower.contains("bilibili.tv") {
        return Some("https://www.bilibili.com/");
    }

    None
}
