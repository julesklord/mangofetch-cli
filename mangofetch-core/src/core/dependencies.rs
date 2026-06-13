use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Stdio;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedDependencies {
    pub ytdlp: Option<PathBuf>,
    pub ffmpeg: Option<PathBuf>,
}

pub async fn ensure_dependencies(
    force: bool,
    reporter: Option<crate::core::traits::SharedReporter>,
) -> Result<ResolvedDependencies> {
    let rep_ref = reporter.as_ref().map(|r| r.as_ref());

    if let Some(r) = rep_ref {
        r.on_system_progress("Checking dependencies", 0.0, "Starting...");
    }

    if force {
        tracing::info!("Force updating dependencies...");
        // Reset caches to ensure we don't return old paths
        crate::core::ytdlp::reset_ytdlp_cache();
        crate::core::ytdlp::reset_ffmpeg_location_cache();
        crate::core::ytdlp::reset_js_runtime_cache();

        // Re-download yt-dlp
        let ytdlp = crate::core::ytdlp::force_update_ytdlp(rep_ref).await.ok();

        // Re-download ffmpeg
        let ffmpeg = download_ffmpeg(rep_ref).await.ok();

        if let Some(r) = rep_ref {
            r.on_system_progress("Update complete", 100.0, "Ready");
        }

        return Ok(ResolvedDependencies { ytdlp, ffmpeg });
    }

    let ytdlp = crate::core::ytdlp::ensure_ytdlp(rep_ref).await.ok();
    let ffmpeg = ensure_ffmpeg(rep_ref).await.ok();

    if let Some(r) = rep_ref {
        r.on_system_progress("Dependencies ready", 100.0, "Ready");
    }

    Ok(ResolvedDependencies { ytdlp, ffmpeg })
}

/// Return true when the runtime is expected to avoid network auto-downloads.
/// This is driven by the MANGOFETCH_OFFLINE env var (set to "1" or "true").
pub fn is_offline_mode() -> bool {
    std::env::var("MANGOFETCH_OFFLINE")
        .map(|v| {
            let s = v.to_ascii_lowercase();
            s == "1" || s == "true"
        })
        .unwrap_or(false)
}

/// Verify file at `path` matches expected sha256 hex string.
pub fn verify_sha256(path: &PathBuf, expected_hex: &str) -> anyhow::Result<bool> {
    use std::fs::File;
    use std::io::Read;

    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    let hash = hasher.finalize();
    let hex = hex::encode(hash);
    Ok(hex.eq_ignore_ascii_case(expected_hex))
}

/// Read expected hash for a tool from app_data_dir/tool_hashes.json
pub fn read_expected_hash(tool: &str) -> Option<String> {
    let data_dir = crate::core::paths::app_data_dir()?;
    let file = data_dir.join("tool_hashes.json");
    let s = std::fs::read_to_string(&file).ok()?;
    let map: serde_json::Value = serde_json::from_str(&s).ok()?;
    map.get(tool).and_then(|v| v.as_str()).map(|s| s.to_string())
}

use anyhow::anyhow;

pub fn is_flatpak() -> bool {
    std::path::Path::new("/.flatpak-info").exists() || std::env::var("FLATPAK_ID").is_ok()
}

fn managed_bin_dir() -> Option<PathBuf> {
    Some(crate::core::paths::app_data_dir()?.join("bin"))
}

pub fn bin_name(tool: &str) -> String {
    if cfg!(target_os = "windows") {
        format!("{}.exe", tool)
    } else {
        tool.to_string()
    }
}

pub async fn find_tool(tool: &str) -> Option<PathBuf> {
    let _timer_start = std::time::Instant::now();
    let name = bin_name(tool);
    let version_flag = version_flag_for(tool);

    #[cfg(target_os = "linux")]
    {
        let flatpak_path = PathBuf::from("/app/bin").join(&name);
        if flatpak_path.exists() {
            tracing::debug!(
                "[perf] find_tool({}) took {:?}",
                tool,
                _timer_start.elapsed()
            );
            return Some(flatpak_path);
        }
    }

    // Check managed bin dir first — managed binaries are known-good.
    let managed = managed_bin_dir().map(|d| d.join(&name));
    if let Some(ref managed_path) = managed {
        if managed_path.exists() {
            let check = {
                let managed = managed_path.clone();
                let vf = version_flag.to_string();
                tokio::task::spawn_blocking(move || {
                    crate::core::process::std_command(&managed)
                        .arg(&vf)
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .status()
                        .ok()
                        .filter(|s| s.success())
                })
                .await
                .ok()
                .flatten()
            };

            if check.is_some() {
                tracing::debug!(
                    "[perf] find_tool({}) took {:?}",
                    tool,
                    _timer_start.elapsed()
                );
                return Some(managed_path.clone());
            }
            tracing::warn!(
                "find_tool({}): binary exists at {} but failed to execute",
                tool,
                managed_path.display()
            );
        }
    }

    // Fall back to system PATH. Resolve to an absolute path so callers
    // (e.g. find_ffmpeg_location) can derive the parent directory.
    let result = {
        let name = name.clone();
        let vf = version_flag.to_string();
        tokio::task::spawn_blocking(move || {
            crate::core::process::std_command(&name)
                .arg(&vf)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .ok()
                .filter(|s| s.success())
        })
        .await
        .ok()
        .flatten()
    };

    if result.is_some() {
        let abs = resolve_absolute_path(&name);
        tracing::debug!(
            "[perf] find_tool({}) took {:?}",
            tool,
            _timer_start.elapsed()
        );
        return Some(abs);
    }

    tracing::debug!(
        "[perf] find_tool({}) took {:?}",
        tool,
        _timer_start.elapsed()
    );
    None
}

/// Resolve a bare binary name to its absolute path via `where` (Windows)
/// or `which` (Unix). Returns the original name as fallback.
fn resolve_absolute_path(bin_name: &str) -> PathBuf {
    let finder = if cfg!(target_os = "windows") {
        "where"
    } else {
        "which"
    };
    if let Ok(output) = crate::core::process::std_command(finder)
        .arg(bin_name)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()
    {
        if output.status.success() {
            if let Some(line) = String::from_utf8_lossy(&output.stdout).lines().next() {
                let path = line.trim();
                if !path.is_empty() {
                    return PathBuf::from(path);
                }
            }
        }
    }
    PathBuf::from(bin_name)
}

fn version_flag_for(tool: &str) -> &'static str {
    match tool {
        "ffmpeg" | "ffprobe" => "-version",
        _ => "--version",
    }
}

pub fn parse_version_output(tool: &str, stdout: &str) -> Option<String> {
    let first_line = stdout.lines().next().unwrap_or("");

    if tool == "ffmpeg" || tool == "ffprobe" {
        first_line.split_whitespace().nth(2).map(|s| s.to_string())
    } else if tool == "yt-dlp" {
        if first_line.trim().is_empty() {
            None
        } else {
            Some(first_line.trim().to_string())
        }
    } else if tool == "aria2c" {
        first_line.split_whitespace().nth(2).map(|s| s.to_string())
    } else {
        if first_line.trim().is_empty() {
            None
        } else {
            Some(first_line.trim().to_string())
        }
    }
}

pub async fn check_version(tool: &str) -> Option<String> {
    let _timer_start = std::time::Instant::now();
    let path = find_tool(tool).await?;
    let version_flag = version_flag_for(tool);
    let output = {
        let path = path.clone();
        let vf = version_flag.to_string();
        tokio::task::spawn_blocking(move || {
            crate::core::process::std_command(&path)
                .arg(&vf)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
        })
        .await
        .ok()?
        .ok()?
    };

    if !output.status.success() {
        tracing::debug!(
            "[perf] check_version({}) took {:?}",
            tool,
            _timer_start.elapsed()
        );
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let result = parse_version_output(tool, &stdout);

    tracing::debug!(
        "[perf] check_version({}) took {:?}",
        tool,
        _timer_start.elapsed()
    );
    result
}

pub async fn ensure_ffmpeg(
    _reporter: Option<&dyn crate::core::traits::DownloadReporter>,
) -> anyhow::Result<PathBuf> {
    // Always ensure the managed binary exists — the standalone yt-dlp.exe
    // cannot discover system FFmpeg from PATH.
    if !is_flatpak() {
        let managed = managed_bin_dir().map(|d| d.join(bin_name("ffmpeg")));
        if managed.as_ref().is_none_or(|p| !p.exists()) {
            if let Ok(path) = download_ffmpeg(_reporter).await {
                crate::core::ytdlp::reset_ffmpeg_location_cache();
                return Ok(path);
            }
        }
    }

    if let Some(path) = find_tool("ffmpeg").await {
        return Ok(path);
    }
    if is_flatpak() {
        return Err(anyhow!("FFmpeg not found in Flatpak sandbox"));
    }
    let path = download_ffmpeg(_reporter).await?;
    crate::core::ytdlp::reset_ffmpeg_location_cache();
    Ok(path)
}

async fn download_ffmpeg(
    _reporter: Option<&dyn crate::core::traits::DownloadReporter>,
) -> anyhow::Result<PathBuf> {
    if is_offline_mode() {
        return Err(anyhow!("Offline mode enabled: automatic FFmpeg download disabled"));
    }
    let bin_dir = managed_bin_dir().ok_or_else(|| anyhow!("Could not determine data directory"))?;
    std::fs::create_dir_all(&bin_dir)?;

    let ffmpeg_name = bin_name("ffmpeg");
    let ffprobe_name = bin_name("ffprobe");
    let ffmpeg_target = bin_dir.join(&ffmpeg_name);

    let downloads = ffmpeg_download_urls();

    for (url, archive_type) in downloads {
        tracing::info!("Downloading FFmpeg component from {}", url);
        let bytes = crate::core::http_client::download_with_progress(url, |percent| {
            if let Some(r) = _reporter {
                r.on_system_progress("ffmpeg", percent, "Downloading FFmpeg...");
            }
        })
        .await?;

        let temp_path = bin_dir.join(".ffmpeg_download.tmp");
        let data = bytes.to_vec();
        let temp_clone = temp_path.clone();
        tokio::task::spawn_blocking(move || std::fs::write(&temp_clone, &data))
            .await
            .map_err(|e| anyhow!("spawn_blocking failed: {}", e))??;

        let file_size = std::fs::metadata(&temp_path)?.len();
        if file_size < 1_000_000 {
            let _ = std::fs::remove_file(&temp_path);
            return Err(anyhow!(
                "Downloaded file from {} is too small ({}B) — likely an error page",
                url,
                file_size
            ));
        }

        match archive_type {
            ArchiveType::Zip => {
                extract_zip_ffmpeg(&temp_path, &bin_dir, &ffmpeg_name, &ffprobe_name).await?
            }
            ArchiveType::TarXz => {
                extract_tar_xz_ffmpeg(&temp_path, &bin_dir, &ffmpeg_name, &ffprobe_name).await?
            }
        }

        let _ = std::fs::remove_file(&temp_path);
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o755);
        let _ = std::fs::set_permissions(&ffmpeg_target, perms.clone());
        let ffprobe_path = bin_dir.join(&ffprobe_name);
        if ffprobe_path.exists() {
            let _ = std::fs::set_permissions(&ffprobe_path, perms);
        }
    }

    #[cfg(target_os = "macos")]
    {
        let ffmpeg_mac = ffmpeg_target.clone();
        if let Err(e) = tokio::task::spawn_blocking(move || {
            crate::core::process::std_command("xattr")
                .args(["-d", "com.apple.quarantine"])
                .arg(&ffmpeg_mac)
                .output()
        })
        .await
        .map_err(|e| std::io::Error::other(e.to_string()))
        .and_then(|r| r)
        {
            tracing::warn!("Failed to remove quarantine from ffmpeg: {}", e);
        }
        let ffprobe_path = bin_dir.join(&ffprobe_name);
        if ffprobe_path.exists() {
            let ffprobe_mac = ffprobe_path.clone();
            if let Err(e) = tokio::task::spawn_blocking(move || {
                crate::core::process::std_command("xattr")
                    .args(["-d", "com.apple.quarantine"])
                    .arg(&ffprobe_mac)
                    .output()
            })
            .await
            .map_err(|e| std::io::Error::other(e.to_string()))
            .and_then(|r| r)
            {
                tracing::warn!("Failed to remove quarantine from ffprobe: {}", e);
            }
        }
    }

    if !ffmpeg_target.exists() {
        return Err(anyhow!("FFmpeg binary not found after extraction"));
    }

    // If an expected hash is present in app data, verify the assembled ffmpeg binary.
    if let Some(expected) = read_expected_hash("ffmpeg") {
        let target_clone = ffmpeg_target.clone();
        let expected_clone = expected.clone();
        let ok = tokio::task::spawn_blocking(move || verify_sha256(&target_clone, &expected_clone))
            .await
            .map_err(|e| anyhow!("spawn_blocking failed: {}", e))??;
        if !ok {
            let _ = std::fs::remove_file(&ffmpeg_target);
            return Err(anyhow!("FFmpeg download failed SHA256 verification"));
        }
    }

    let verify = {
        let target = ffmpeg_target.clone();
        tokio::task::spawn_blocking(move || {
            crate::core::process::std_command(&target)
                .arg("-version")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
        })
        .await
        .map_err(|e| anyhow!("spawn_blocking failed: {}", e))?
    };
    match verify {
        Ok(s) if s.success() => {}
        Ok(s) => {
            return Err(anyhow!(
                "FFmpeg installed but failed to execute (exit code {})",
                s
            ))
        }
        Err(e) => return Err(anyhow!("FFmpeg installed but failed to execute: {}", e)),
    }

    tracing::info!("FFmpeg installed to {}", ffmpeg_target.display());
    Ok(ffmpeg_target)
}

enum ArchiveType {
    Zip,
    TarXz,
}

fn ffmpeg_download_urls() -> Vec<(&'static str, ArchiveType)> {
    if cfg!(target_os = "windows") {
        vec![(
            "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-gpl.zip",
            ArchiveType::Zip,
        )]
    } else if cfg!(target_os = "macos") {
        vec![
            (
                "https://evermeet.cx/ffmpeg/getrelease/zip",
                ArchiveType::Zip,
            ),
            (
                "https://evermeet.cx/ffmpeg/getrelease/ffprobe/zip",
                ArchiveType::Zip,
            ),
        ]
    } else if cfg!(target_arch = "aarch64") {
        vec![(
            "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-linuxarm64-gpl.tar.xz",
            ArchiveType::TarXz,
        )]
    } else {
        vec![(
            "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-linux64-gpl.tar.xz",
            ArchiveType::TarXz,
        )]
    }
}

async fn extract_zip_ffmpeg(
    archive_path: &std::path::Path,
    bin_dir: &std::path::Path,
    ffmpeg_name: &str,
    ffprobe_name: &str,
) -> anyhow::Result<()> {
    let archive_path = archive_path.to_path_buf();
    let bin_dir = bin_dir.to_path_buf();
    let ffmpeg_name = ffmpeg_name.to_string();
    let ffprobe_name = ffprobe_name.to_string();

    tokio::task::spawn_blocking(move || {
        let file = std::fs::File::open(&archive_path)
            .map_err(|e| anyhow!("Failed to open archive: {}", e))?;
        let mut archive =
            zip::ZipArchive::new(file).map_err(|e| anyhow!("Failed to open zip: {}", e))?;

        let targets = [ffmpeg_name.as_str(), ffprobe_name.as_str()];

        for i in 0..archive.len() {
            let mut entry = archive
                .by_index(i)
                .map_err(|e| anyhow!("Failed to read zip entry: {}", e))?;

            let name = entry.name().to_string();
            for target in &targets {
                if name.ends_with(target) {
                    let dest = bin_dir.join(target);
                    let mut out = std::fs::File::create(&dest)?;
                    std::io::copy(&mut entry, &mut out)?;
                    break;
                }
            }
        }

        Ok::<(), anyhow::Error>(())
    })
    .await
    .map_err(|e| anyhow!("Spawn blocking failed: {}", e))??;

    Ok(())
}

async fn extract_tar_xz_ffmpeg(
    archive_path: &std::path::Path,
    bin_dir: &std::path::Path,
    ffmpeg_name: &str,
    ffprobe_name: &str,
) -> anyhow::Result<()> {
    let archive_path = archive_path.to_path_buf();
    let bin_dir = bin_dir.to_path_buf();
    let ffmpeg_name = ffmpeg_name.to_string();
    let ffprobe_name = ffprobe_name.to_string();

    tokio::task::spawn_blocking(move || {
        let file = std::fs::File::open(&archive_path)
            .map_err(|e| anyhow!("Failed to open archive: {}", e))?;
        let decompressor = xz2::read::XzDecoder::new(file);
        let mut archive = tar::Archive::new(decompressor);
        let targets = [ffmpeg_name.as_str(), ffprobe_name.as_str()];

        for entry_result in archive
            .entries()
            .map_err(|e| anyhow!("Failed to read tar entries: {}", e))?
        {
            let mut entry = entry_result.map_err(|e| anyhow!("Failed to read tar entry: {}", e))?;
            let path = entry
                .path()
                .map_err(|e| anyhow!("Failed to read entry path: {}", e))?;
            let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            for target in &targets {
                if file_name == *target {
                    let dest = bin_dir.join(target);
                    let mut out = std::fs::File::create(&dest)?;
                    std::io::copy(&mut entry, &mut out)?;
                    break;
                }
            }
        }
        Ok::<(), anyhow::Error>(())
    })
    .await
    .map_err(|e| anyhow!("Spawn blocking failed: {}", e))??;
    Ok(())
}

// --- aria2c ---

// --- deno (JS runtime for yt-dlp nsig challenge) ---

/// Ensures a JavaScript runtime is available for yt-dlp's YouTube nsig
/// challenge solver. Checks for any existing runtime first (Node.js, Deno,
/// Bun), then auto-downloads Deno if none is found.
pub async fn ensure_js_runtime(
    _reporter: Option<&dyn crate::core::traits::DownloadReporter>,
) -> Option<PathBuf> {
    // Check system-installed runtimes first.
    for tool in &["deno", "node", "bun"] {
        if let Some(path) = find_tool(tool).await {
            return Some(path);
        }
    }

    // Check well-known install locations on Windows.
    #[cfg(target_os = "windows")]
    {
        let candidates = [
            r"C:\Program Files\nodejs\node.exe",
            r"C:\Program Files (x86)\nodejs\node.exe",
        ];
        for path in &candidates {
            let p = std::path::PathBuf::from(path);
            if p.exists() {
                return Some(p);
            }
        }
    }

    match download_deno(_reporter).await {
        Ok(path) => Some(path),
        Err(e) => {
            tracing::warn!("Failed to download Deno JS runtime: {}", e);
            None
        }
    }
}

async fn download_deno(
    _reporter: Option<&dyn crate::core::traits::DownloadReporter>,
) -> anyhow::Result<PathBuf> {
    let bin_dir = managed_bin_dir().ok_or_else(|| anyhow!("Could not determine data directory"))?;
    std::fs::create_dir_all(&bin_dir)?;

    let deno_name = bin_name("deno");
    let deno_target = bin_dir.join(&deno_name);

    if deno_target.exists() {
        return Ok(deno_target);
    }

    let url = if cfg!(target_os = "windows") {
        "https://github.com/denoland/deno/releases/latest/download/deno-x86_64-pc-windows-msvc.zip"
    } else if cfg!(target_os = "macos") {
        if cfg!(target_arch = "aarch64") {
            "https://github.com/denoland/deno/releases/latest/download/deno-aarch64-apple-darwin.zip"
        } else {
            "https://github.com/denoland/deno/releases/latest/download/deno-x86_64-apple-darwin.zip"
        }
    } else if cfg!(target_arch = "aarch64") {
        "https://github.com/denoland/deno/releases/latest/download/deno-aarch64-unknown-linux-gnu.zip"
    } else {
        "https://github.com/denoland/deno/releases/latest/download/deno-x86_64-unknown-linux-gnu.zip"
    };

    tracing::info!("Downloading Deno JS runtime from {}", url);

    let bytes = crate::core::http_client::download_with_progress(url, |percent| {
        if let Some(r) = _reporter {
            r.on_system_progress("deno", percent, "Downloading Deno...");
        }
    })
    .await?;
    let data = bytes.to_vec();
    let bin_dir_clone = bin_dir.clone();
    let deno_name_clone = deno_name.clone();

    tokio::task::spawn_blocking(move || {
        let cursor = std::io::Cursor::new(&data);
        let mut archive =
            zip::ZipArchive::new(cursor).map_err(|e| anyhow!("Failed to open Deno zip: {}", e))?;

        for i in 0..archive.len() {
            let mut file = archive
                .by_index(i)
                .map_err(|e| anyhow!("Failed to read zip entry: {}", e))?;

            let name = file.name().to_string();
            if name.ends_with(&deno_name_clone) || name == "deno" || name == "deno.exe" {
                let dest = bin_dir_clone.join(&deno_name_clone);
                let mut buf = Vec::new();
                std::io::Read::read_to_end(&mut file, &mut buf)?;
                std::fs::write(&dest, &buf)?;
                break;
            }
        }

        Ok::<(), anyhow::Error>(())
    })
    .await
    .map_err(|e| anyhow!("Spawn blocking failed: {}", e))??;

    if !deno_target.exists() {
        return Err(anyhow!("Deno binary not found after extraction"));
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&deno_target, std::fs::Permissions::from_mode(0o755));
    }

    #[cfg(target_os = "macos")]
    {
        let deno_mac = deno_target.clone();
        let _ = tokio::task::spawn_blocking(move || {
            crate::core::process::std_command("xattr")
                .args(["-d", "com.apple.quarantine"])
                .arg(&deno_mac)
                .output()
        })
        .await;
    }

    tracing::info!("Deno installed to {}", deno_target.display());
    Ok(deno_target)
}

pub async fn ensure_aria2c(
    _reporter: Option<&dyn crate::core::traits::DownloadReporter>,
) -> Option<PathBuf> {
    if let Some(path) = find_tool("aria2c").await {
        return Some(path);
    }

    // Auto-download only on Windows
    #[cfg(target_os = "windows")]
    {
        match download_aria2c(_reporter).await {
            Ok(path) => return Some(path),
            Err(e) => {
                tracing::warn!("Failed to download aria2c: {}", e);
            }
        }
    }

    None
}

#[cfg(target_os = "windows")]
async fn download_aria2c(
    _reporter: Option<&dyn crate::core::traits::DownloadReporter>,
) -> anyhow::Result<PathBuf> {
    let bin_dir = managed_bin_dir().ok_or_else(|| anyhow!("Could not determine data directory"))?;
    std::fs::create_dir_all(&bin_dir)?;

    let aria2c_name = bin_name("aria2c");
    let aria2c_target = bin_dir.join(&aria2c_name);

    let url = "https://github.com/aria2/aria2/releases/download/release-1.37.0/aria2-1.37.0-win-64bit-build1.zip";

    let bytes = crate::core::http_client::download_with_progress(url, |percent| {
        if let Some(r) = _reporter {
            r.on_system_progress("aria2c", percent, "Downloading aria2c...");
        }
    })
    .await?;

    let data = bytes.to_vec();
    let bin_dir_clone = bin_dir.clone();
    let aria2c_name_clone = aria2c_name.clone();

    tokio::task::spawn_blocking(move || {
        let cursor = std::io::Cursor::new(&data);
        let mut archive = zip::ZipArchive::new(cursor)
            .map_err(|e| anyhow!("Failed to open aria2c zip: {}", e))?;

        for i in 0..archive.len() {
            let mut file = archive
                .by_index(i)
                .map_err(|e| anyhow!("Failed to read zip entry: {}", e))?;

            let name = file.name().to_string();
            if name.ends_with(&aria2c_name_clone) {
                let dest = bin_dir_clone.join(&aria2c_name_clone);
                let mut buf = Vec::new();
                std::io::Read::read_to_end(&mut file, &mut buf)?;
                std::fs::write(&dest, &buf)?;
                break;
            }
        }

        Ok::<(), anyhow::Error>(())
    })
    .await
    .map_err(|e| anyhow!("Spawn blocking failed: {}", e))??;

    if !aria2c_target.exists() {
        return Err(anyhow!("aria2c binary not found after extraction"));
    }

    Ok(aria2c_target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::events::QueueItemProgress;
    use crate::core::traits::DownloadReporter;
    use crate::models::queue::QueueItemInfo;
    use crate::models::settings::ProxySettings;
    use std::sync::Arc;
    use std::sync::Mutex;

    static TEST_MUTEX: Mutex<()> = Mutex::new(());

    struct MockReporter;

    impl DownloadReporter for MockReporter {
        fn on_progress(&self, _id: u64, _prog: QueueItemProgress) {}
        fn on_complete(&self, _id: u64, _path: Option<String>, _size: Option<u64>) {}
        fn on_error(&self, _id: u64, _msg: String) {}
        fn on_retry(&self, _id: u64, _attempt: u32, _delay: u64) {}
        fn on_phase_change(&self, _id: u64, _phase: String) {}
        fn on_media_preview(
            &self,
            _u: String,
            _t: String,
            _a: String,
            _th: Option<String>,
            _d: Option<f64>,
        ) {
        }
        fn on_queue_update(&self, _s: Vec<QueueItemInfo>) {}
        fn on_system_progress(&self, _title: &str, _pct: f32, _msg: &str) {}
    }

    #[test]
    fn test_bin_name() {
        if cfg!(target_os = "windows") {
            assert_eq!(bin_name("test-tool"), "test-tool.exe");
            assert_eq!(bin_name("yt-dlp"), "yt-dlp.exe");
        } else {
            assert_eq!(bin_name("test-tool"), "test-tool");
            assert_eq!(bin_name("yt-dlp"), "yt-dlp");
        }
    }

    #[test]
    fn test_is_flatpak() {
        let _guard = TEST_MUTEX.lock().unwrap();

        let original_val = std::env::var("FLATPAK_ID");

        std::env::set_var("FLATPAK_ID", "org.mangofetch.App");
        assert!(is_flatpak(), "Should be true when FLATPAK_ID is set");

        std::env::remove_var("FLATPAK_ID");
        let expected = std::path::Path::new("/.flatpak-info").exists();
        assert_eq!(
            is_flatpak(),
            expected,
            "When FLATPAK_ID is not set, it should match the existence of /.flatpak-info"
        );

        match original_val {
            Ok(v) => std::env::set_var("FLATPAK_ID", v),
            Err(_) => std::env::remove_var("FLATPAK_ID"),
        }
    }

    #[test]
    fn test_parse_version_output() {
        // ffmpeg
        assert_eq!(
            parse_version_output("ffmpeg", "ffmpeg version 2024-05-13-git-93afb9c47c-full_build-www.gyan.dev Copyright (c) 2000-2024 the FFmpeg developers"),
            Some("2024-05-13-git-93afb9c47c-full_build-www.gyan.dev".to_string())
        );
        assert_eq!(
            parse_version_output(
                "ffmpeg",
                "ffmpeg version N-111111-g1234567890 Copyright (c) 2000-2023 the FFmpeg developers"
            ),
            Some("N-111111-g1234567890".to_string())
        );
        assert_eq!(parse_version_output("ffmpeg", "ffmpeg version"), None);
        assert_eq!(parse_version_output("ffmpeg", ""), None);

        // ffprobe
        assert_eq!(
            parse_version_output("ffprobe", "ffprobe version 2024-05-13-git-93afb9c47c-full_build-www.gyan.dev Copyright (c) 2000-2024 the FFmpeg developers"),
            Some("2024-05-13-git-93afb9c47c-full_build-www.gyan.dev".to_string())
        );
        assert_eq!(parse_version_output("ffprobe", "ffprobe version"), None);
        assert_eq!(parse_version_output("ffprobe", ""), None);

        // yt-dlp
        assert_eq!(
            parse_version_output("yt-dlp", "2024.04.09\n"),
            Some("2024.04.09".to_string())
        );
        assert_eq!(
            parse_version_output("yt-dlp", "2023.11.16"),
            Some("2023.11.16".to_string())
        );
        assert_eq!(
            parse_version_output("yt-dlp", "  2024.04.09  "),
            Some("2024.04.09".to_string())
        );
        assert_eq!(parse_version_output("yt-dlp", ""), None);
        assert_eq!(parse_version_output("yt-dlp", "   \n"), None);

        // aria2c
        assert_eq!(
            parse_version_output(
                "aria2c",
                "aria2 version 1.37.0\nCopyright (C) 2006, 2019 Tatsuhiro Tsujikawa"
            ),
            Some("1.37.0".to_string())
        );
        assert_eq!(
            parse_version_output("aria2c", "aria2 version 1.36.0"),
            Some("1.36.0".to_string())
        );
        assert_eq!(parse_version_output("aria2c", "aria2 version"), None);
        assert_eq!(parse_version_output("aria2c", ""), None);

        // other / default
        assert_eq!(
            parse_version_output("other", "1.2.3\n"),
            Some("1.2.3".to_string())
        );
        assert_eq!(
            parse_version_output("other", "  1.2.3  "),
            Some("1.2.3".to_string())
        );
        assert_eq!(parse_version_output("other", ""), None);
        assert_eq!(parse_version_output("other", "   \n"), None);
    }

    #[tokio::test]
    async fn test_ensure_dependencies_force_error() {
        let _guard = TEST_MUTEX.lock().unwrap();

        let reporter: Arc<dyn DownloadReporter> = Arc::new(MockReporter);

        // Set an invalid proxy to force download failures
        crate::core::http_client::init_proxy(ProxySettings {
            enabled: true,
            proxy_type: "http".into(),
            host: "0.0.0.0".into(), // Unroutable IP to simulate network failure
            port: 1,
            username: "".into(),
            password: "".into(),
        });

        // Test with force=true
        let result = ensure_dependencies(true, Some(reporter.clone())).await;

        // ensure_dependencies itself should still succeed because it gracefully handles download errors
        assert!(result.is_ok());
        let deps = result.unwrap();

        // However, the missing dependencies should not have been downloaded
        assert!(
            deps.ytdlp.is_none(),
            "ytdlp should be none on network error"
        );
        assert!(
            deps.ffmpeg.is_none(),
            "ffmpeg should be none on network error"
        );

        // Restore global proxy setting
        crate::core::http_client::init_proxy(ProxySettings::default());
    }

    #[test]
    fn test_verify_sha256_and_read_expected_hash() {
        let _guard = TEST_MUTEX.lock().unwrap();

        // Create a temporary app data dir
        let tmp = std::env::temp_dir().join(format!("mangofetch_test_{}", uuid::Uuid::new_v4()));
        let _ = std::fs::create_dir_all(&tmp);

        // Ensure we restore env var after test
        let original = std::env::var("MANGOFETCH_DATA_DIR");
        std::env::set_var("MANGOFETCH_DATA_DIR", &tmp);

        // Create a test file and compute its sha256
        let file_path = tmp.join("test.bin");
        let data = b"hello-mangofetch";
        std::fs::write(&file_path, &data[..]).expect("write temp file");
        let mut hasher = Sha256::new();
        hasher.update(data);
        let expected = hex::encode(hasher.finalize());

        // verify_sha256 should succeed with the correct hash
        assert!(verify_sha256(&file_path, &expected).unwrap());

        // and fail for an incorrect hash
        assert!(!verify_sha256(&file_path, "deadbeef").unwrap());

        // Create a tool_hashes.json manifest and read_expected_hash
        let manifest = serde_json::json!({"yt-dlp": expected});
        let manifest_path = tmp.join("tool_hashes.json");
        std::fs::write(&manifest_path, serde_json::to_string(&manifest).unwrap()).unwrap();

        // read_expected_hash should pick up the value
        let found = read_expected_hash("yt-dlp");
        assert_eq!(found.as_deref(), Some(expected.as_str()));

        // Non-existent entry returns None
        assert!(read_expected_hash("ffmpeg").is_none());

        // restore original env
        match original {
            Ok(v) => std::env::set_var("MANGOFETCH_DATA_DIR", v),
            Err(_) => std::env::remove_var("MANGOFETCH_DATA_DIR"),
        }

        // cleanup
        let _ = std::fs::remove_file(&file_path);
        let _ = std::fs::remove_file(&manifest_path);
        let _ = std::fs::remove_dir(&tmp);
    }
}
