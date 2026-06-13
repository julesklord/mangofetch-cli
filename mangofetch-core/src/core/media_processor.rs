use crate::core::hls_downloader::HlsDownloadResult;
use tokio_util::sync::CancellationToken;

pub struct HlsDownloadOptions<'a> {
    pub m3u8_url: &'a str,
    pub output: &'a str,
    pub referer: &'a str,
    pub bytes_tx: Option<tokio::sync::mpsc::UnboundedSender<u64>>,
    pub cancel_token: CancellationToken,
    pub max_concurrent: u32,
    pub max_retries: u32,
    pub client: Option<reqwest::Client>,
    pub max_height: Option<u32>,
}

pub struct MediaProcessor;

impl MediaProcessor {
    pub async fn download_hls(
        options: HlsDownloadOptions<'_>,
    ) -> anyhow::Result<HlsDownloadResult> {
        let downloader = match options.client {
            Some(c) => crate::core::hls_downloader::HlsDownloader::with_client(c),
            None => crate::core::hls_downloader::HlsDownloader::new(),
        };
        downloader
            .download(
                options.m3u8_url,
                options.output,
                options.referer,
                options.bytes_tx,
                options.cancel_token,
                options.max_concurrent,
                options.max_retries,
            )
            .await
    }

    pub async fn download_hls_with_quality(
        options: HlsDownloadOptions<'_>,
    ) -> anyhow::Result<HlsDownloadResult> {
        let downloader = match options.client {
            Some(c) => crate::core::hls_downloader::HlsDownloader::with_client(c),
            None => crate::core::hls_downloader::HlsDownloader::new(),
        };
        downloader
            .download_with_quality(
                options.m3u8_url,
                options.output,
                options.referer,
                options.bytes_tx,
                options.cancel_token,
                options.max_concurrent,
                options.max_retries,
                options.max_height,
            )
            .await
    }

    pub async fn remux(input: &str, output: &str) -> anyhow::Result<()> {
        let status = crate::core::process::command("ffmpeg")
            .args(["-y", "-i", input, "-c", "copy", output])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .status()
            .await?;

        if !status.success() {
            anyhow::bail!("FFmpeg remux falhou com status {}", status);
        }

        Ok(())
    }

    pub async fn merge_audio_video(video: &str, audio: &str, output: &str) -> anyhow::Result<()> {
        let status = crate::core::process::command("ffmpeg")
            .args([
                "-y", "-i", video, "-i", audio, "-map", "0:v", "-map", "1:a", "-c", "copy", output,
            ])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .status()
            .await?;

        if !status.success() {
            anyhow::bail!("FFmpeg merge falhou com status {}", status);
        }

        Ok(())
    }

    pub async fn download_direct(
        url: &str,
        output: &str,
        headers: &[(&str, &str)],
    ) -> anyhow::Result<()> {
        let mut args = vec!["-y".to_string()];

        if !headers.is_empty() {
            let header_str: String = headers
                .iter()
                .map(|(k, v)| format!("{}: {}\r\n", k, v))
                .collect();
            args.extend(["-headers".to_string(), header_str]);
        }

        args.extend([
            "-i".to_string(),
            url.to_string(),
            "-c".to_string(),
            "copy".to_string(),
            output.to_string(),
        ]);

        let status = crate::core::process::command("ffmpeg")
            .args(&args)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .status()
            .await?;

        if !status.success() {
            anyhow::bail!("FFmpeg download_direct falhou com status {}", status);
        }

        Ok(())
    }
}

pub trait CommandRunner {
    fn check_command_success(&self, program: &str, args: &[&str]) -> bool;
}

pub struct RealCommandRunner;

impl CommandRunner for RealCommandRunner {
    fn check_command_success(&self, program: &str, args: &[&str]) -> bool {
        crate::core::process::std_command(program)
            .args(args)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
}

pub fn check_ffmpeg_with_runner(runner: &impl CommandRunner) -> bool {
    runner.check_command_success("ffmpeg", &["-version"])
}

pub fn check_ffmpeg() -> bool {
    check_ffmpeg_with_runner(&RealCommandRunner)
}

pub fn check_ytdlp_with_runner(runner: &impl CommandRunner) -> bool {
    runner.check_command_success("yt-dlp", &["--version"])
}

pub fn check_ytdlp() -> bool {
    check_ytdlp_with_runner(&RealCommandRunner)
}

pub fn check_dependencies_with_runner(runner: &impl CommandRunner) -> Vec<String> {
    let mut missing = Vec::new();
    if !check_ytdlp_with_runner(runner) {
        missing.push("yt-dlp".into());
    }
    if !check_ffmpeg_with_runner(runner) {
        missing.push("ffmpeg".into());
    }
    missing
}

pub fn check_dependencies() -> Vec<String> {
    check_dependencies_with_runner(&RealCommandRunner)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    struct MockCommandRunner {
        // maps "program" -> success status
        responses: HashMap<String, bool>,
    }

    impl MockCommandRunner {
        fn new() -> Self {
            Self {
                responses: HashMap::new(),
            }
        }

        fn set_response(mut self, program: &str, success: bool) -> Self {
            self.responses.insert(program.to_string(), success);
            self
        }
    }

    impl CommandRunner for MockCommandRunner {
        fn check_command_success(&self, program: &str, _args: &[&str]) -> bool {
            *self.responses.get(program).unwrap_or(&false)
        }
    }

    #[test]
    fn test_check_dependencies_both_present() {
        let runner = MockCommandRunner::new()
            .set_response("ffmpeg", true)
            .set_response("yt-dlp", true);

        assert!(check_ffmpeg_with_runner(&runner));
        assert!(check_ytdlp_with_runner(&runner));
        let missing = check_dependencies_with_runner(&runner);
        assert!(missing.is_empty());
    }

    #[test]
    fn test_check_dependencies_ffmpeg_missing() {
        let runner = MockCommandRunner::new()
            .set_response("ffmpeg", false)
            .set_response("yt-dlp", true);

        assert!(!check_ffmpeg_with_runner(&runner));
        assert!(check_ytdlp_with_runner(&runner));
        let missing = check_dependencies_with_runner(&runner);
        assert_eq!(missing, vec!["ffmpeg".to_string()]);
    }

    #[test]
    fn test_check_dependencies_ytdlp_missing() {
        let runner = MockCommandRunner::new()
            .set_response("ffmpeg", true)
            .set_response("yt-dlp", false);

        assert!(check_ffmpeg_with_runner(&runner));
        assert!(!check_ytdlp_with_runner(&runner));
        let missing = check_dependencies_with_runner(&runner);
        assert_eq!(missing, vec!["yt-dlp".to_string()]);
    }

    #[test]
    fn test_check_dependencies_both_missing() {
        let runner = MockCommandRunner::new()
            .set_response("ffmpeg", false)
            .set_response("yt-dlp", false);

        assert!(!check_ffmpeg_with_runner(&runner));
        assert!(!check_ytdlp_with_runner(&runner));
        let missing = check_dependencies_with_runner(&runner);
        assert_eq!(missing, vec!["yt-dlp".to_string(), "ffmpeg".to_string()]);
    }
}
